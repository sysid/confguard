// confguard/src/util/helper/testing.rs
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::core::config::{default_base, DEFAULT_BASE};
use crate::core::{settings, Settings};
use crate::errors::{ConfGuardError, ConfGuardResult};
use crate::util::helper::file_contents;
use crate::util::home_based::to_home_based;
use chrono::TimeZone;
use fs_extra::{copy_items, dir, remove_items};
use itertools::Itertools;
use once_cell::sync::Lazy;
use remove_dir_all::ensure_empty_dir;
use rstest::{fixture, rstest};
use std::sync::Once;
use tempfile::TempDir;
use tracing::info;
use tracing::{debug, warn};
use tracing_subscriber::fmt::format::FmtSpan;

/**
 * Test setup
 * Test directory is DEFAULT_BASE: $HOME/xxx/rs-cg
 */
static TEST_SETUP: Once = Once::new();
pub const TEST_VERSION: i64 = 999;
pub const TEST_ENV_VARS: &[&str] = &["CONFGUARD_BASE_DIR", "RUST_LOG", "NO_CLEANUP"];

pub fn testing_dir() -> String {
    env::var("HOME")
        .map(|home| format!("{}/xxx/rs-cg", home))
        .unwrap_or_else(|_| "/tmp/xxx/rs-cg".to_string())
}

pub static TESTING_DIR: Lazy<String> = Lazy::new(testing_dir);

pub fn init_test_setup() {
    TEST_SETUP.call_once(|| {
        if env::var("RUST_LOG").is_err() {
            env::set_var("RUST_LOG", "debug");
        }
        // global logging subscriber, used by all tracing log macros
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            // .with_env_filter("info") // Set log level
            .with_target(true) // Include module path
            .with_thread_names(false) // Include thread names (optional)
            // .with_span_events(FmtSpan::ACTIVE) // Log span entry/exit
            .with_test_writer()
            .init();

        // Set up test settings
        env::remove_var("CONFGUARD_BASE_DIR");
        env::remove_var("CONFGUARD_VERSION");
        env::set_var("CONFGUARD_BASE_DIR", DEFAULT_BASE.as_str());
        env::set_var("CONFGUARD_VERSION", TEST_VERSION.to_string());

        // Force reload settings with test configuration
        Settings::reload().expect("test env vars are set - reload must succeed");
        info!("Test Setup complete");
    });
}

pub fn setup_test_dir() -> PathBuf {
    init_test_setup(); // Idempotent via Once - ensures logging and env vars are configured
    let settings = settings();
    let test_dir = settings.confguard_base_dir.clone();
    assert_eq!(
        test_dir,
        DEFAULT_BASE.as_str(),
        "Test directory must be DEFAULT_BASE, is: {}",
        test_dir
    );
    let path = PathBuf::from(&test_dir);
    ensure_empty_dir(&path).expect("test dir under temp - creation must succeed");
    path
}

pub fn teardown_test_dir(test_dir: &Path) {
    if env::var("NO_CLEANUP").is_err() && test_dir.exists() {
        fs::remove_dir_all(test_dir).expect("test dir exists and is removable");
    } else {
        println!("Test artifacts left at: {}", test_dir.display());
    }
}

pub fn print_active_env_vars(vars: &[&str]) {
    for var in vars {
        if let Ok(value) = env::var(var) {
            println!("{}={}", var, value);
        } else {
            println!("{} is not set", var);
        }
    }
}

pub fn create_file_with_content(file_path: &str, content: &str) -> ConfGuardResult<()> {
    let path = Path::new(file_path);

    // Create directories if necessary
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;

    Ok(())
}

pub fn generate_envrc_content(test_dir: &Path) -> ConfGuardResult<String> {
    Ok(format!(
        "#------------------------------- confguard start --------------------------------
# config.relative = true
# config.version = 2
# state.sentinel = 'test-12345678'
# state.timestamp = '2024-04-15T10:00:00.000Z'
# state.sourceDir = '{}'
export SOPS_PATH=$HOME/path/to/sops
dotenv $SOPS_PATH/environments/local.env
#-------------------------------- confguard end ---------------------------------",
        to_home_based(test_dir)?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[fixture]
    fn test_dir() -> PathBuf {
        setup_test_dir()
    }

    #[rstest]
    fn test_setup_test_dir(test_dir: PathBuf) {
        assert!(test_dir.exists());
    }

    #[rstest]
    fn test_teardown_test_dir(test_dir: PathBuf) {
        teardown_test_dir(&test_dir);
        assert!(!test_dir.exists());
    }

    #[rstest]
    fn test_print_active_env_vars() {
        print_active_env_vars(TEST_ENV_VARS);
    }

    #[rstest]
    fn test_generate_envrc_content(test_dir: PathBuf) {
        let content = generate_envrc_content(test_dir.as_path()).unwrap();
        println!("{}", content);
        assert!(content.contains("confguard start"));
        assert!(content.contains("confguard end"));
        assert!(content.contains("SOPS_PATH=$HOME/path/to/sops"));
    }

    #[rstest]
    fn test_create_file_with_content(test_dir: PathBuf) {
        let file_path = test_dir.join("nested/test.txt");
        let content = "test content";
        create_file_with_content(file_path.to_str().unwrap(), content).unwrap();
        let file_content = read_to_string(file_path).unwrap();
        assert_eq!(file_content.trim(), content);
        // teardown_test_dir(&base_dir);
    }
}
