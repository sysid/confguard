use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::errors::{ConfGuardContext, ConfGuardError, ConfGuardResult};
use clap::{Command, CommandFactory, Parser};
use clap_complete::{generate, Generator};
use colored::Colorize;
use tracing::{debug, info};

use crate::cli::args::{Cli, Commands};
use crate::core::config::{
    confguard_config_path, settings_with_override, Settings, CONFGUARD_CONFIG_FILE,
};
use crate::core::settings;
use crate::core::ConfGuard;
use crate::sops::manager::SopsManager;
use crate::util::copy_file_from_resources;
use crate::util::helper::path::to_absolute_path;
use crate::util::link::replace_link_with_target;

pub fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

pub fn execute_command(cli: &Cli) -> ConfGuardResult<()> {
    if let Some(generator) = cli.generator {
        let mut cmd = Cli::command();
        eprintln!("Generating completion file for {generator:?}...");
        print_completions(generator, &mut cmd);
    }

    // Resolve effective settings (--base-dir CLI override takes precedence)
    let effective_settings = settings_with_override(cli.base_dir.as_deref());

    match &cli.command {
        Some(Commands::Show { source_dir }) => {
            let abs_dir = to_absolute_path(source_dir)?;
            show(&abs_dir)
        }
        Some(Commands::Guard {
            source_dir,
            absolute,
        }) => {
            let abs_dir = to_absolute_path(source_dir)?;
            guard(&abs_dir, *absolute)
        }
        Some(Commands::Unguard { source_dir }) => {
            let abs_dir = to_absolute_path(source_dir)?;
            unguard(&abs_dir)
        }
        Some(Commands::GuardOne {
            source_dir,
            source_path,
        }) => {
            let abs_dir = to_absolute_path(source_dir)?;
            let abs_path = to_absolute_path(source_path)?;
            guard_one(&abs_dir, &abs_path)
        }
        Some(Commands::Relink { envrc_path }) => {
            let abs_path = to_absolute_path(envrc_path)?;
            relink(&abs_path)
        }
        Some(Commands::ReplaceLink { link }) => {
            let path: &Path = link.as_ref(); // Explicitly annotate the type of `path`

            let abs_link = if path.is_absolute() {
                path.to_path_buf()
            } else {
                // Get the current directory and join it with the relative path
                let current_dir = std::env::current_dir().context("get current directory")?;
                current_dir.join(path)
            };
            replace_link(&abs_link)
        }
        Some(Commands::FixRunConfig { source_dir }) => {
            let abs_dir = to_absolute_path(source_dir)?;
            fix_run_config(&abs_dir)
        }
        Some(Commands::Init {
            source_dir,
            template_path,
        }) => {
            let abs_dir = to_absolute_path(source_dir)?;
            let abs_template = template_path.as_ref().map(to_absolute_path).transpose()?;
            init_envrc(&abs_dir, abs_template.as_deref())
        }
        Some(Commands::Info) => info(&effective_settings),
        Some(Commands::SopsEnc { dir }) => {
            let config_path =
                PathBuf::from(&effective_settings.confguard_base_dir).join(CONFGUARD_CONFIG_FILE);
            let manager = SopsManager::new(&config_path)?;
            let scan_dir = dir.as_ref().map(to_absolute_path).transpose()?;
            manager.encrypt_files(scan_dir.as_deref())
        }
        Some(Commands::SopsDec { dir }) => {
            let config_path =
                PathBuf::from(&effective_settings.confguard_base_dir).join(CONFGUARD_CONFIG_FILE);
            let manager = SopsManager::new(&config_path)?;
            let scan_dir = dir.as_ref().map(to_absolute_path).transpose()?;
            manager.decrypt_files(scan_dir.as_deref())
        }
        Some(Commands::SopsClean { dir }) => {
            let config_path =
                PathBuf::from(&effective_settings.confguard_base_dir).join(CONFGUARD_CONFIG_FILE);
            let manager = SopsManager::new(&config_path)?;
            let scan_dir = dir.as_ref().map(to_absolute_path).transpose()?;
            manager.clean_files(scan_dir.as_deref())
        }
        Some(Commands::SopsInit { template_path }) => {
            let abs_template = template_path.as_ref().map(to_absolute_path).transpose()?;
            sops_init(&effective_settings, abs_template)
        }
        None => Ok(()),
    }
}

fn info(settings: &Settings) -> ConfGuardResult<()> {
    // Version
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    // Configuration
    println!("Configuration:");
    println!("  Base directory: {}", settings.confguard_base_dir);
    println!("  Guarded storage: {}", settings.guarded);
    println!("  Config version: {}", settings.version);

    // confguard.toml status
    let config_path = PathBuf::from(&settings.confguard_base_dir).join("confguard.toml");
    if config_path.exists() {
        let metadata = fs::metadata(&config_path)?;
        println!(
            "  Config file: {} ({} bytes)",
            config_path.display(),
            metadata.len()
        );
    } else {
        println!("  Config file: {} (not found)", config_path.display());
    }
    println!();

    // SOPS Configuration (if config exists)
    if config_path.exists() {
        println!("SOPS Configuration:");
        match SopsManager::new(&config_path) {
            Ok(manager) => {
                println!("  GPG key: {}", manager.config.gpg_key);
                println!(
                    "  Encrypt extensions: {:?}",
                    manager.config.file_extensions_enc
                );
                println!("  Encrypt filenames: {:?}", manager.config.file_names_enc);
                println!(
                    "  Decrypt extensions: {:?}",
                    manager.config.file_extensions_dec
                );
                println!("  Decrypt filenames: {:?}", manager.config.file_names_dec);
            }
            Err(e) => println!("  Error loading config: {}", e),
        }
        println!();
    }

    // Environment Variables
    println!("Environment:");
    display_env_var("CONFGUARD_BASE_DIR");
    display_env_var("CONFGUARD_VERSION");
    display_env_var("RUST_LOG");
    println!();

    // Statistics
    println!("Statistics:");
    let guarded_path = PathBuf::from(&settings.guarded);
    if guarded_path.exists() {
        let count = fs::read_dir(&guarded_path)?.count();
        println!("  Guarded projects: {}", count);
    } else {
        println!("  Guarded projects: 0 (directory not found)");
    }

    Ok(())
}

fn display_env_var(name: &str) {
    match std::env::var(name) {
        Ok(value) => println!("  {}: {}", name, value),
        Err(_) => println!("  {}: (not set)", name),
    }
}

fn init_envrc(source_dir: &Path, template_path: Option<&Path>) -> ConfGuardResult<()> {
    debug!(
        "source_dir: {:?}, template_path: {:?}",
        source_dir, template_path
    );

    let path = PathBuf::from(source_dir).join(".envrc");
    if path.exists() {
        return Err(ConfGuardError::FileAlreadyExists(path));
    }

    match template_path {
        Some(template) => {
            fs::copy(template, &path)
                .with_context(|| format!("copy template: {}", template.display()))?;
            println!(".envrc created from template: {}", template.display());
        }
        None => {
            copy_file_from_resources("dot.envrc", path.to_str().unwrap(), false)
                .context("copy resources/dot.envrc -> .envrc")?;
            println!(".envrc created from default template");
        }
    }
    println!("\nCreated: {}", path.to_string_lossy());
    Ok(())
}

fn fix_run_config(source_dir: &Path) -> ConfGuardResult<()> {
    debug!("source_dir: {:?}", source_dir);

    // This operation requires a guarded project
    let guard = ConfGuard::from_guarded_project(Path::new(source_dir))?;

    let run_config_path = guard.source_dir.join(".idea/runConfigurations/rsenv.sh");
    copy_file_from_resources("rsenv.sh", run_config_path.to_str().unwrap(), true)
        .context("copy rsenv.sh")?;

    println!(
        "Environment file created:\n{}\n",
        run_config_path.to_string_lossy()
    );
    Ok(())
}

fn show(source_dir: &Path) -> ConfGuardResult<()> {
    debug!("source_dir: {:?}", source_dir);

    match ConfGuard::from_guarded_project(Path::new(source_dir)) {
        Ok(guard) => {
            println!("{:#?}", guard);
            Ok(())
        }
        Err(e) => {
            println!("Error: {}", e);
            Ok(())
        }
    }
}

fn guard(source_dir: &Path, absolute: bool) -> ConfGuardResult<()> {
    debug!("source_dir: {:?}, absolute: {}", source_dir, absolute);

    let source_path = PathBuf::from(source_dir);
    let mut guard = ConfGuard::new_for_guarding(source_path, !absolute)?;
    guard.guard(absolute)
}

fn unguard(source_dir: &Path) -> ConfGuardResult<()> {
    debug!("source_dir: {:?}", source_dir);

    // Must be an existing guarded project
    let mut guard = ConfGuard::from_guarded_project(Path::new(source_dir))?;
    guard.unguard()
}

fn guard_one(source_dir: &Path, source_path: &Path) -> ConfGuardResult<()> {
    debug!(
        "source_dir: {:?}, source_path: {:?}",
        source_dir, source_path
    );

    // Must be an existing guarded project
    let guard = ConfGuard::from_guarded_project(Path::new(source_dir))?;

    let source = PathBuf::from(source_path)
        .canonicalize()
        .context("resolve source path")?;

    if !source.exists() {
        return Err(ConfGuardError::SourceNotFound(source));
    }

    // Check that source is within the project directory
    if !source.starts_with(&guard.source_dir) {
        // Validate that source file is within the project boundaries for security
        return Err(ConfGuardError::SourceNotInProject(source));
    }

    // Calculate target path relative to guarded directory
    let relative = source
        .strip_prefix(&guard.source_dir)
        .context("calculate relative path")?;

    let target_dir = guard
        .target_dir
        .as_ref()
        .ok_or(ConfGuardError::TargetDirectoryNotSet)?;

    let target = target_dir.join(relative);

    if target.exists() {
        return Err(ConfGuardError::TargetAlreadyExists(target));
    }

    // Create parent directories if needed
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    // Move file and create link
    fs::rename(&source, &target)?;
    std::os::unix::fs::symlink(&target, &source)?;

    println!("Guarded: {} -> {}", source.display(), target.display());
    Ok(())
}

fn relink(envrc_path: &Path) -> ConfGuardResult<()> {
    debug!("envrc_path: {:?}", envrc_path);

    match ConfGuard::relink(Path::new(envrc_path)) {
        Ok(_) => Ok(()),
        Err(e) => {
            if e.to_string().contains("Link already exists")
                || e.to_string().contains("Link exists but points elsewhere")
            {
                // Print as information rather than error
                println!("{}", e);
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

fn replace_link(link: &Path) -> ConfGuardResult<()> {
    debug!("link: {:?}", link);
    let path = Path::new(link);

    if !path.exists() {
        return Err(ConfGuardError::LinkNotFound(path.to_path_buf()));
    }

    if !path.symlink_metadata()?.file_type().is_symlink() {
        return Err(ConfGuardError::NotSymbolicLink(path.to_path_buf()));
    }

    replace_link_with_target(path)?;
    println!("Replaced link with target: {:?}", path);
    Ok(())
}

fn sops_init(settings: &Settings, template_path: Option<PathBuf>) -> ConfGuardResult<()> {
    debug!(
        "settings.confguard_base_dir: {:?}, template_path: {:?}",
        settings.confguard_base_dir, template_path
    );

    let target_dir = PathBuf::from(&settings.confguard_base_dir);
    let config_path = target_dir.join(CONFGUARD_CONFIG_FILE);
    if config_path.exists() {
        // Prevent overwriting existing configuration without explicit confirmation
        return Err(ConfGuardError::ConfigFileAlreadyExists(config_path));
    }

    // Create target directory if it doesn't exist
    fs::create_dir_all(&target_dir).context("create target directory")?;

    match template_path {
        Some(template) => {
            fs::copy(template.clone(), &config_path).with_context({
                let template_display = template.display().to_string();
                move || format!("copy template: {}", template_display)
            })?;
            println!("Configuration copied from template: {}", template.display());
        }
        None => {
            copy_file_from_resources("confguard.toml", config_path.to_str().unwrap(), false)
                .context("copy default confguard.toml template")?;
            println!("Default configuration created");
        }
    }

    println!("\nCreated: {}", config_path.to_string_lossy());
    Ok(())
}
