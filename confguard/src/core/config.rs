use crate::errors::{ConfGuardContext, ConfGuardError, ConfGuardResult};
use crate::sops::manager::SopsConfig;
use config::{Config, ConfigError};
use once_cell::sync::{Lazy, OnceCell};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::{env, fs};
use tracing::debug;

/// Global Configuration
pub fn default_base() -> String {
    env::var("HOME")
        .map(|home| format!("{}/xxx/rs-cg", home))
        .unwrap_or_else(|_| "/tmp/xxx/rs-cg".to_string())
}

pub static DEFAULT_BASE: Lazy<String> = Lazy::new(default_base);

const DEFAULT_VERSION: i64 = 3;

pub const CONFGUARD_CONFIG_FILE: &str = "confguard.toml";

static SETTINGS: OnceCell<RwLock<Settings>> = OnceCell::new();

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    // Specifies default function to use if the field CONFGUARD_BASE_DIR is not present in the deserialized data.
    #[serde(default = "default_base")]
    pub confguard_base_dir: String, // base dir for entire confguard
    pub guarded: String, // storage location of guarded files
    #[serde(default = "default_version")]
    pub version: i64,
    #[serde(default)]
    pub sops: Option<SopsConfig>,
}

fn default_version() -> i64 {
    DEFAULT_VERSION
}

impl Settings {
    /// Creates a new Settings instance with values from config sources
    pub fn new() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .set_default("base_dir", DEFAULT_BASE.as_str())?
            .set_default("version", DEFAULT_VERSION)?
            .add_source(config::Environment::with_prefix("CONFGUARD"))
            .build()?;

        let settings = Settings {
            confguard_base_dir: config.get_string("base_dir")?,
            guarded: format!("{}/guarded", config.get_string("base_dir")?),
            version: DEFAULT_VERSION, // Will be populated from confguard.toml if present
            sops: None,               // Will be populated from confguard.toml if present
        };
        debug!("settings: {:?}", settings);

        Ok(settings)
    }

    /// Gets a reference to the global settings: Create it
    pub fn global() -> &'static RwLock<Settings> {
        SETTINGS.get_or_init(|| {
            RwLock::new(Self::new().expect("Settings::new() uses hardcoded defaults - cannot fail"))
        })
    }

    /// Provides shared read access to the global settings
    pub fn read_global() -> std::sync::RwLockReadGuard<'static, Settings> {
        Self::global()
            .read()
            .expect("settings lock not poisoned - no panics in critical section")
    }

    /// Updates the global settings with exclusive write access
    pub fn update_global(
        new_settings: Settings,
    ) -> Result<(), std::sync::PoisonError<std::sync::RwLockWriteGuard<'static, Settings>>> {
        let mut settings = Self::global().write()?;
        *settings = new_settings;
        Ok(())
    }

    /// Force a reload of the settings from the environment
    pub fn reload() -> Result<(), ConfigError> {
        if let Some(lock) = SETTINGS.get() {
            let mut settings = lock
                .write()
                .expect("settings lock not poisoned - no panics in critical section");
            *settings = Self::new()?;
        }
        Ok(())
    }

    pub fn load_sops_config(&mut self, config_path: &Path) -> ConfGuardResult<()> {
        if !config_path.exists() {
            return Ok(());
        }

        let config_str = fs::read_to_string(config_path).context("read SOPS config file")?;

        let config: SopsConfig = toml::from_str(&config_str).context("parse SOPS config")?;

        self.sops = Some(config);
        Ok(())
    }
}

pub fn settings() -> std::sync::RwLockReadGuard<'static, Settings> {
    Settings::read_global()
}

/// Returns settings with an optional base directory override.
///
/// If `base_dir_override` is provided, creates a modified Settings instance
/// with the base directory and guarded path adjusted accordingly.
/// Otherwise, returns a clone of the global settings.
///
/// # Arguments
/// * `base_dir_override` - Optional path to override the base directory
pub fn settings_with_override(base_dir_override: Option<&Path>) -> Settings {
    match base_dir_override {
        Some(path) => {
            let base = path.to_string_lossy().to_string();
            let mut s = Settings::read_global().clone();
            s.confguard_base_dir = base.clone();
            s.guarded = format!("{}/guarded", base);
            s
        }
        None => Settings::read_global().clone(),
    }
}

/// Returns the path to confguard.toml configuration file
///
/// By default, looks for confguard.toml in the confguard base directory.
/// If an override path is provided, uses that instead.
///
/// # Arguments
/// * `override_path` - Optional path to override default config location
///
/// # Returns
/// * `Result<PathBuf>` - Resolved path to the config file
/// ```
pub fn confguard_config_path(override_path: Option<impl AsRef<Path>>) -> ConfGuardResult<PathBuf> {
    match override_path {
        Some(path) => Ok(PathBuf::from(path.as_ref())),
        None => {
            let settings = settings();
            let config_path =
                PathBuf::from(&settings.confguard_base_dir).join(CONFGUARD_CONFIG_FILE);

            // Verify the path exists
            if !config_path.exists() {
                return Err(ConfGuardError::ConfigNotFoundWithHint {
                    path: config_path.display().to_string(),
                });
            }

            Ok(config_path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::testing::{
        create_file_with_content, init_test_setup, setup_test_dir, TEST_VERSION,
    };
    use rstest::rstest;
    use std::env;

    #[rstest]
    fn print_settings() {
        init_test_setup();
        let settings = settings();
        println!("{:?}", settings);
    }

    #[rstest]
    fn test_default_settings() {
        init_test_setup();
        let settings = settings();
        println!("{:?}", settings);
        assert_eq!(settings.confguard_base_dir, DEFAULT_BASE.as_str());
        assert_eq!(settings.version, DEFAULT_VERSION);
    }

    #[test]
    fn test_home_based_default_path() {
        // Save original HOME value
        let original_home = env::var("HOME").ok();

        // Set a test HOME
        env::set_var("HOME", "/test/home");

        // Get the default base
        let base = default_base();
        assert_eq!(base, "/test/home/xxx/rs-cg");

        // Test with no HOME set
        env::remove_var("HOME");
        let fallback = default_base();
        assert_eq!(fallback, "/tmp/xxx/rs-cg");

        // Restore original HOME if it existed
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        }
    }

    #[test]
    fn test_environment_override() {
        // Clean environment first
        std::env::remove_var("CONFGUARD_BASE_DIR");
        std::env::remove_var("CONFGUARD_VERSION");

        // Set new values
        std::env::set_var("CONFGUARD_BASE_DIR", "/custom/path");
        std::env::set_var("CONFGUARD_VERSION", "3");

        // Create new settings instance
        let settings = Settings::new().unwrap();
        println!("{:?}", settings);

        // Verify overrides
        assert_eq!(
            settings.confguard_base_dir, "/custom/path",
            "Environment variable CONFGUARD_BASE_DIR should override default"
        );
        assert_eq!(
            settings.version, 3,
            "Environment variable CONFGUARD_VERSION should override default"
        );

        // Clean up
        std::env::remove_var("CONFGUARD_BASE_DIR");
        std::env::remove_var("CONFGUARD_VERSION");
    }

    #[ignore = "manual test, changes global settings for other tests"]
    #[test]
    fn test_update_global_settings() {
        let new_settings = Settings {
            confguard_base_dir: "/new/path".to_string(),
            guarded: "/new/path/guarded".to_string(),
            version: 4,
            sops: None,
        };

        Settings::update_global(new_settings.clone()).unwrap();
        let settings = Settings::read_global();

        assert_eq!(settings.confguard_base_dir, "/new/path");
        assert_eq!(settings.version, 4);
    }

    #[rstest]
    fn test_get_confguard_config_path_default() -> ConfGuardResult<()> {
        let _ = setup_test_dir();

        // Create test config
        let settings = settings();
        let config_path = PathBuf::from(&settings.confguard_base_dir).join(CONFGUARD_CONFIG_FILE);
        std::fs::create_dir_all(config_path.parent().unwrap())?;
        create_file_with_content(config_path.to_str().unwrap(), "# Test config\n")?;

        let result = confguard_config_path(None as Option<&Path>)?;
        assert_eq!(result, config_path);
        Ok(())
    }

    #[rstest]
    fn test_get_confguard_config_path_override() -> ConfGuardResult<()> {
        let override_path = PathBuf::from("/custom/path/confguard.toml");
        let result = confguard_config_path(Some(&override_path))?;
        assert_eq!(result, override_path);
        Ok(())
    }
}
