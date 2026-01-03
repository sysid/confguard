// core/guard.rs
use crate::core::settings;
use crate::errors::{ConfGuardContext, ConfGuardError, ConfGuardResult};
use crate::util::helper::copy_file_from_resources;
use crate::util::helper::path::find_file_pattern;
use crate::util::home_based::{from_home_based, to_home_based};
use crate::util::link::{create_link, replace_link_with_target};
use crate::util::move_and_link::MoveAndLink;
use chrono::Utc;
use derive_builder::Builder;
use derive_more::Constructor;
use pathdiff::diff_paths;
use regex::Regex;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Constructor, Clone, PartialEq, Builder)]
pub struct ConfGuard {
    #[builder(default)]
    pub version: i64,
    #[builder(default)]
    pub source_dir: PathBuf,
    #[builder(default)]
    pub is_relative: bool,
    #[builder(default)]
    pub confguard_base_dir: PathBuf,
    #[builder(default)]
    pub target_dir: Option<PathBuf>,
    #[builder(default)]
    pub sentinel: String,
}

impl Default for ConfGuard {
    fn default() -> Self {
        Self {
            version: settings().version,
            source_dir: PathBuf::from(""),
            is_relative: true,
            confguard_base_dir: PathBuf::from(settings().confguard_base_dir.as_str()),
            target_dir: None,
            sentinel: "".into(),
        }
    }
}

// in guard.rs
impl ConfGuard {
    /// Reads an existing ConfGuard configuration from a .envrc or dot.envrc file
    /// Returns None if the file doesn't contain a valid confguard section
    #[instrument]
    #[must_use = "check the returned Option to determine if project is guarded"]
    pub fn from_envrc(envrc_path: &Path) -> ConfGuardResult<Option<ConfGuard>> {
        let contents = fs::read_to_string(envrc_path).map_err(|e| {
            ConfGuardError::Internal(format!("Cannot read envrc file {:?}: {}", envrc_path, e))
        })?;

        let lines: Vec<&str> = contents.lines().collect();

        // Check for confguard section markers
        let has_start = lines.iter().any(|l| l.contains("confguard start"));
        let has_end = lines.iter().any(|l| l.contains("confguard end"));

        if !has_start || !has_end {
            return Ok(None);
        }

        // Parse confguard section
        let sentinel_pattern = Regex::new(r"^# state\.sentinel = '(.+)'")?;
        let is_relative_pattern = Regex::new(r"^# config\.relative = (true|false)")?;
        let version_pattern = Regex::new(r"^# config\.version = (\d+)")?;
        let source_dir_pattern = Regex::new(r"^# state\.sourceDir = '(.+)'")?;

        let mut sentinel_option = None;
        let mut is_relative_option = None;
        let mut version_option = None;
        let mut source_dir_option = None;

        for line in lines {
            if let Some(caps) = sentinel_pattern.captures(line) {
                sentinel_option = Some(caps.get(1).unwrap().as_str().to_string());
            } else if let Some(caps) = is_relative_pattern.captures(line) {
                is_relative_option = Some(caps.get(1).unwrap().as_str() == "true");
            } else if let Some(caps) = version_pattern.captures(line) {
                // Parse version string, defaulting to error if malformed
                version_option =
                    Some(caps.get(1).unwrap().as_str().parse::<i64>().map_err(|e| {
                        ConfGuardError::Internal(format!("Invalid version format: {}", e))
                    })?);
            } else if let Some(caps) = source_dir_pattern.captures(line) {
                // Convert home-based path back to absolute path for internal use
                source_dir_option = Some(from_home_based(caps.get(1).unwrap().as_str())?);
            }
        }

        // All fields must be present for a valid guarded project
        match (
            sentinel_option,
            is_relative_option,
            version_option,
            source_dir_option,
        ) {
            (Some(sentinel), Some(is_relative), Some(version), Some(source_dir)) => {
                let mut guard = ConfGuardBuilder::default()
                    .version(version)
                    .source_dir(source_dir)
                    .is_relative(is_relative)
                    .confguard_base_dir(PathBuf::from(settings().confguard_base_dir.as_str()))
                    .sentinel(sentinel.clone())
                    .build()
                    .map_err(|e| ConfGuardError::Internal(format!("Builder error: {}", e)))?;

                // Set target_dir to use the guarded subdirectory
                guard.target_dir = Some(
                    PathBuf::from(&settings().confguard_base_dir)
                        .join("guarded")
                        .join(&sentinel),
                );

                Ok(Some(guard))
            }
            _ => Ok(None),
        }
    }

    /// Creates a new ConfGuard instance for initial guarding
    /// Fails if the project is already guarded
    #[instrument]
    pub fn new_for_guarding(source_dir: PathBuf, is_relative: bool) -> ConfGuardResult<ConfGuard> {
        let envrc_path = source_dir.join(".envrc");
        if !envrc_path.exists() {
            return Err(ConfGuardError::NoEnvrcFile(source_dir.to_path_buf()));
        }

        // Check if .envrc is a symlink
        if envrc_path.symlink_metadata()?.file_type().is_symlink() {
            return Err(ConfGuardError::ProjectAlreadyGuardedSymlink);
        }

        // Check for existing confguard section
        if (Self::from_envrc(&envrc_path)?).is_some() {
            return Err(ConfGuardError::AlreadyGuarded(source_dir));
        }

        ConfGuardBuilder::default()
            .version(settings().version)
            .source_dir(source_dir)
            .is_relative(is_relative)
            .confguard_base_dir(PathBuf::from(settings().confguard_base_dir.as_str()))
            .sentinel(String::new())
            .build()
            .map_err(|e| ConfGuardError::BuilderError(e.to_string()))
    }

    /// Gets the ConfGuard instance for an existing guarded project
    /// Fails if the project is not guarded
    #[instrument]
    pub fn from_guarded_project(source_dir: &Path) -> ConfGuardResult<ConfGuard> {
        let envrc_path = source_dir.join(".envrc");
        if !envrc_path.exists() {
            return Err(ConfGuardError::NoEnvrcFile(source_dir.to_path_buf()));
        }

        match Self::from_envrc(&envrc_path)? {
            Some(guard) => Ok(guard),
            None => Err(ConfGuardError::NotGuarded(source_dir.to_path_buf())),
        }
    }

    /// Recreates a symbolic link to a guarded project's dot.envrc file.
    ///
    /// This function can be used to restore a broken link or relink in a new location.
    /// Will fail if a link already exists or points to a different location.
    ///
    /// # Arguments
    /// * `envrc_path` - Path to the dot.envrc file that we want to link to
    ///
    /// # Returns
    /// * `Result<()>` - Ok(()) if successful, or an Error if:
    ///   - File doesn't exist
    ///   - File is not a guarded envrc
    ///   - Link already exists
    ///   - Link points elsewhere
    ///   - File operations fail
    #[instrument]
    pub fn relink(envrc_path: &Path) -> ConfGuardResult<()> {
        if !envrc_path.exists() {
            return Err(ConfGuardError::FileNotFound(envrc_path.to_path_buf()));
        }

        // Canonicalize path to resolve symlinks and get absolute path for comparison
        let target_path = envrc_path
            .canonicalize()
            .with_context(|| format!("canonicalize path: {:?}", envrc_path))?;

        // Extract guard configuration, ensuring this is actually a guarded project
        let guard = match Self::from_envrc(&target_path)? {
            Some(guard) => guard,
            None => return Err(ConfGuardError::NotGuardedEnvrc(envrc_path.to_path_buf())),
        };

        let original_envrc_path = guard.source_dir.join(".envrc");

        // Check if link already exists
        if original_envrc_path.exists()
            && original_envrc_path
                .symlink_metadata()
                .with_context(|| format!("read metadata for: {:?}", original_envrc_path))?
                .file_type()
                .is_symlink()
        {
            let existing_target = {
                let current_dir =
                    std::env::current_dir().with_context(|| "get current directory".to_string())?;
                // Get parent directory to change context for relative link resolution
                let link_dir = original_envrc_path.parent().ok_or_else(|| {
                    ConfGuardError::NoParentDirectory(original_envrc_path.clone())
                })?;
                std::env::set_current_dir(link_dir)
                    .with_context(|| format!("change directory to: {:?}", link_dir))?;
                let target = fs::read_link(&original_envrc_path)
                    .with_context(|| format!("read link: {:?}", original_envrc_path))?
                    .canonicalize()
                    .with_context(|| {
                        format!("canonicalize link target for: {:?}", original_envrc_path)
                    })?;
                std::env::set_current_dir(current_dir)
                    .with_context(|| "restore current directory".to_string())?;
                target
            };
            debug!("Check: {:?} == {:?}", target_path, existing_target);
            // Check if link points to our target or somewhere else
            if existing_target == target_path {
                return Err(ConfGuardError::LinkAlreadyExists(
                    original_envrc_path.to_string_lossy().to_string(),
                    target_path.to_string_lossy().to_string(),
                ));
            } else {
                return Err(ConfGuardError::LinkPointsElsewhere(
                    original_envrc_path.to_string_lossy().to_string(),
                    existing_target.to_string_lossy().to_string(),
                ));
            }
        }

        // create_link has good error messages
        create_link(&target_path, &original_envrc_path, guard.is_relative)?;

        println!(
            "Relinked: {:?} -> {:?}",
            original_envrc_path.to_string_lossy(),
            target_path.to_string_lossy()
        );
        Ok(())
    }

    /// Guards a project directory by:
    /// - Moving .envrc to a managed location and creating a symlink
    /// - Adding confguard configuration to .envrc
    /// - Setting up SOPS environment for encrypted files
    ///
    /// # Returns
    /// * `Result<()>` - Ok(()) if successful, or an Error if:
    ///   - Source directory doesn't exist
    ///   - Project is already guarded
    ///   - Unable to create sentinel
    ///   - Unable to set up SOPS environment
    ///   - File operations fail
    #[instrument]
    pub fn guard(&mut self, absolute: bool) -> ConfGuardResult<()> {
        // Validate source directory exists
        // Validate that source directory exists before attempting to guard it
        if !self.source_dir.exists() {
            return Err(ConfGuardError::SourceDirectoryNotFound(
                self.source_dir.clone(),
            ));
        }

        let source_envrc = self.source_dir.join(".envrc");

        // Ensure source .envrc exists and is not already a link
        // Verify .envrc exists before we can guard the project
        if !source_envrc.exists() {
            return Err(ConfGuardError::NoEnvrcFile(self.source_dir.clone()));
        }
        if source_envrc
            .symlink_metadata()
            .with_context(|| format!("read metadata for: {:?}", source_envrc))?
            .file_type()
            .is_symlink()
        {
            // Symlinked .envrc indicates project is already guarded
            return Err(ConfGuardError::ProjectAlreadyGuardedSymlink);
        }

        self.is_relative = !absolute;
        self.create_sentinel()?;
        self.create_sops_envs()?;

        let target_dir = self
            .target_dir
            .as_ref()
            .ok_or(ConfGuardError::TargetDirectoryNotSet)?;
        let target_envrc = target_dir.join("dot.envrc");

        // MoveAndLink constructor has good error messages
        let mal = MoveAndLink::new(&source_envrc, &target_envrc)?;
        // move_and_link needs context as its internal errors might be low-level
        mal.move_and_link(self.is_relative)
            .with_context(|| format!("move and link: {:?} -> {:?}", source_envrc, target_envrc))?;
        println!("Guarded: {:?} -> {:?}", &source_envrc, &target_envrc);

        // canonicalize needs context as its error is low-level
        let canonical_path = target_envrc
            .canonicalize()
            .with_context(|| format!("canonicalize path: {:?}", target_envrc))?;

        // update_dot_envrc has good error messages
        self.update_dot_envrc(&canonical_path)?;

        Ok(())
    }

    /// Removes ConfGuard management from a project directory by:
    /// - Replacing the .envrc symlink with its target content
    /// - Removing the confguard configuration section from .envrc
    /// - Replacing all managed symlinks (those pointing to the sentinel directory) with their target content
    /// - Preserving any non-managed symlinks
    /// - Does not remove the sentinel directory or its contents
    #[instrument]
    pub fn unguard(&mut self) -> ConfGuardResult<()> {
        let source_envrc = self.source_dir.join(".envrc");

        // Verify this is actually a guarded project
        // Verify .envrc exists and is a symlink (indicating it's guarded)
        if !source_envrc.exists() {
            return Err(ConfGuardError::NoEnvrcFile(self.source_dir.clone()));
        }
        if !source_envrc.symlink_metadata()?.file_type().is_symlink() {
            return Err(ConfGuardError::ProjectNotGuardedNotSymlink);
        }

        // replace_link_with_target has good error messages
        replace_link_with_target(&source_envrc)?;

        // delete_section has good error messages
        Self::delete_section(&source_envrc)?;

        // Replace managed links with their targets
        for entry in WalkDir::new(&self.source_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_symlink())
        {
            let link_path = entry.path();
            let target_path = fs::read_link(link_path)
                .with_context(|| format!("read symlink: {:?}", link_path))?;

            // Get absolute path of the target for consistent checking
            let absolute_target = if target_path.is_absolute() {
                target_path.clone()
            } else {
                // Convert relative path to absolute
                let parent = link_path.parent().unwrap_or(Path::new(""));
                parent
                    .join(&target_path)
                    .canonicalize()
                    .with_context(|| format!("resolve relative path for: {:?}", link_path))?
            };

            // Check if the link points to our sentinel directory
            if absolute_target.to_string_lossy().contains(&self.sentinel) {
                // replace_link_with_target has good error messages
                replace_link_with_target(link_path)?;
            }
        }

        Ok(())
    }

    /// Creates a unique sentinel directory for the guarded project.
    ///
    /// This generates a unique identifier based on the source directory name and a random
    /// hex suffix, then creates the corresponding directory in the confguard base directory.
    /// The sentinel serves as the storage location for encrypted environment files.
    ///
    /// # Returns
    /// * `Result<()>` - Ok(()) if successful, or an Error if:
    ///   - Sentinel is already set
    ///   - Source directory path is invalid  
    ///   - Directory creation fails
    ///   - Project appears to already be guarded
    ///
    /// # Side effects
    /// * Sets `self.sentinel` to the generated identifier
    /// * Sets `self.target_dir` to the created directory path
    /// * Creates the sentinel directory on filesystem
    #[instrument(skip(self))]
    pub fn create_sentinel(&mut self) -> ConfGuardResult<()> {
        // Prevent double initialization of sentinel
        if !self.sentinel.is_empty() {
            error!("Sentinel already set: {}", self.sentinel);
            return Err(ConfGuardError::SentinelAlreadySet);
        }

        let dir_name = self
            .source_dir
            .file_name()
            .ok_or(ConfGuardError::InvalidSourceDirectory)?
            .to_string_lossy();

        // Create base directory and guarded subdirectory
        let guarded_dir = PathBuf::from(&self.confguard_base_dir).join("guarded");
        fs::create_dir_all(&guarded_dir)?;

        // Check for existing sentinels in guarded directory
        let existing_sentinels =
            find_file_pattern(&guarded_dir, &format!("{}-[a-z0-9]{{8}}$", dir_name))?;

        // Prevent duplicate guarding by checking for existing sentinels
        if !existing_sentinels.is_empty() {
            return Err(ConfGuardError::SentinelExists(existing_sentinels));
        }

        // Create new sentinel
        let uuid_part = Uuid::new_v4().to_string()[..8].to_string();
        self.sentinel = format!("{}-{}", dir_name, uuid_part);

        // Create target directory in guarded subdirectory
        let target_dir = guarded_dir.join(&self.sentinel);
        fs::create_dir_all(&target_dir)?;
        self.target_dir = Some(target_dir);

        info!(
            "Created sentinel '{}' with target directory '{}'",
            self.sentinel,
            self.target_dir.as_ref().unwrap().display()
        );

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn create_sops_envs(&self) -> ConfGuardResult<()> {
        let target_dir = self
            .target_dir
            .as_ref()
            .ok_or(ConfGuardError::TargetDirectoryNotSet)?;

        // Create environments directory
        let env_dir = target_dir.join("environments");
        fs::create_dir_all(&env_dir)?;

        // Create environment files for different stages
        let environments = ["local", "test", "int", "prod"];
        for env in environments {
            self.create_environment_file(&env_dir, env)?;
        }

        // Create RunConfigurations script
        let run_config_path = self.source_dir.join(".idea/runConfigurations/rsenv.sh");
        copy_file_from_resources(
            "rsenv.sh",
            run_config_path
                .to_str()
                .ok_or_else(|| ConfGuardError::InvalidUtf8Path(run_config_path.clone()))?,
            true,
        )?;
        debug!("EnvFile injection placed: {:?}", run_config_path);

        Ok(())
    }

    /// Creates an environment file for a specific environment (local, test, int, prod)
    #[instrument(skip(self))]
    fn create_environment_file(&self, env_dir: &Path, environment: &str) -> ConfGuardResult<()> {
        let env_file_path = env_dir.join(format!("{}.env", environment));

        // Generate environment-specific content
        let content = self.generate_env_content(environment)?;

        // Write the content to the file
        fs::write(&env_file_path, content)?;

        info!("Environment file created: {:?}", env_file_path);
        Ok(())
    }

    /// Generates environment-specific content for each environment file
    fn generate_env_content(&self, environment: &str) -> ConfGuardResult<String> {
        // Only set RUN_ENV according to the filename
        Ok(format!("export RUN_ENV=\"{}\"\n", environment))
    }

    #[instrument(skip(self))]
    pub fn update_dot_envrc(&self, target_file_path: &Path) -> ConfGuardResult<()> {
        // Get home directory for path calculations in .envrc generation
        let home_dir = dirs::home_dir().ok_or(ConfGuardError::CannotDetermineHome)?;

        // Calculate relative path from home to target for SOPS_PATH variable
        let relative_path = diff_paths(target_file_path.parent().unwrap(), &home_dir)
            .ok_or(ConfGuardError::CannotCalculateRelativePath)?;

        // Read existing content
        let content = fs::read_to_string(target_file_path)?;
        let lines: Vec<&str> = content.lines().collect();

        // Generate new confguard section
        let conf_guard_section = format!(
            "#------------------------------- confguard start --------------------------------\n\
             # config.relative = {}\n\
             # config.version = {}\n\
             # state.sentinel = '{}'\n\
             # state.timestamp = '{}'\n\
             # state.sourceDir = '{}'\n\
             export SOPS_PATH=$HOME/{}\n\
             dotenv $SOPS_PATH/environments/local.env\n\
             #-------------------------------- confguard end ---------------------------------\n",
            if self.is_relative { "true" } else { "false" },
            self.version,
            self.sentinel,
            Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            to_home_based(&self.source_dir)?,
            relative_path.to_string_lossy()
        );

        // Find existing section or append
        let mut new_content = String::new();
        let start_idx = lines.iter().position(|l| l.contains("confguard start"));
        let end_idx = lines.iter().position(|l| l.contains("confguard end"));

        match (start_idx, end_idx) {
            (Some(start), Some(end)) if start < end => {
                new_content.push_str(&lines[..start].join("\n"));
                new_content.push_str(&conf_guard_section);
                if end + 1 < lines.len() {
                    new_content.push_str(&lines[end + 1..].join("\n"));
                }
            }
            _ => {
                new_content.push_str(&content);
                if !content.ends_with('\n') {
                    new_content.push('\n');
                }
                new_content.push_str(&conf_guard_section);
            }
        }

        // Write updated content
        fs::write(target_file_path, new_content)?;

        Ok(())
    }

    #[instrument]
    pub fn delete_section(file_path: &Path) -> ConfGuardResult<()> {
        let content = fs::read_to_string(file_path)?;

        let re = Regex::new(
            r"(?s)#------------------------------- confguard start --------------------------------.*#-------------------------------- confguard end ---------------------------------\n",
        )?;

        let new_content = re.replace(&content, "");
        fs::write(file_path, new_content.as_bytes())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {}
