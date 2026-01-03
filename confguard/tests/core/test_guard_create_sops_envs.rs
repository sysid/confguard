use confguard::core::{ConfGuard, ConfGuardBuilder};
use confguard::errors::ConfGuardResult;
use confguard::util::helper::testing::setup_test_dir;
use confguard::util::testing::create_file_with_content;
use rstest::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[fixture]
fn test_dir() -> PathBuf {
    setup_test_dir()
}

#[fixture]
fn initialized_guard(test_dir: PathBuf) -> (PathBuf, ConfGuard) {
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(envrc_path.to_str().unwrap(), "export FOO=bar\n").unwrap();

    // Create guard with target_dir already set
    let mut guard = ConfGuard::new_for_guarding(test_dir.clone(), true).unwrap();
    guard.create_sentinel().unwrap(); // This sets up target_dir

    (test_dir, guard)
}

#[rstest]
fn test_create_sops_envs_success(initialized_guard: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, guard) = initialized_guard;

    // Act
    guard.create_sops_envs()?;

    // Assert
    let target_dir = guard.target_dir.as_ref().unwrap();

    // Check environments directory
    let env_dir = target_dir.join("environments");
    assert!(env_dir.exists(), "environments directory should exist");
    assert!(env_dir.is_dir(), "environments should be a directory");

    // Check local.env
    let local_env = env_dir.join("local.env");
    assert!(local_env.exists(), "local.env should exist");
    let local_env_content = fs::read_to_string(&local_env)?;
    assert!(
        !local_env_content.is_empty(),
        "local.env should not be empty"
    );

    // Check rsenv.sh
    let rsenv_path = test_dir.join(".idea/runConfigurations/rsenv.sh");
    assert!(rsenv_path.exists(), "rsenv.sh should exist");
    assert!(
        rsenv_path.metadata()?.permissions().mode() & 0o111 != 0,
        "rsenv.sh should be executable"
    );
    let rsenv_content = fs::read_to_string(&rsenv_path)?;
    assert!(!rsenv_content.is_empty(), "rsenv.sh should not be empty");

    Ok(())
}

#[rstest]
fn test_create_sops_envs_existing_run_config(
    initialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, guard) = initialized_guard;

    // Create existing runConfigurations directory with content
    let run_config_dir = test_dir.join(".idea/runConfigurations");
    fs::create_dir_all(&run_config_dir)?;
    let existing_config = run_config_dir.join("existing.xml");
    create_file_with_content(existing_config.to_str().unwrap(), "<configuration/>")?;

    // Act
    guard.create_sops_envs()?;

    // Assert
    assert!(
        existing_config.exists(),
        "Existing config should be preserved"
    );
    let rsenv_path = run_config_dir.join("rsenv.sh");
    assert!(rsenv_path.exists(), "rsenv.sh should be created");
    assert!(
        fs::read_to_string(&existing_config)?.contains("<configuration/>"),
        "Existing config content should be preserved"
    );

    Ok(())
}

#[rstest]
fn test_create_sops_envs_no_target_dir(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let guard = ConfGuardBuilder::default()
        .version(2)
        .source_dir(test_dir.clone())
        .is_relative(true)
        .confguard_base_dir(test_dir.join("guarded"))
        .build()?;

    // Act
    let result = guard.create_sops_envs();

    // Assert
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Target directory not set"),
        "Should fail when target_dir is None"
    );

    Ok(())
}

#[rstest]
fn test_create_sops_envs_existing_environments(
    initialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (_, guard) = initialized_guard;
    let env_dir = guard.target_dir.as_ref().unwrap().join("environments");
    fs::create_dir_all(&env_dir)?;
    let custom_env = env_dir.join("custom.env");
    create_file_with_content(custom_env.to_str().unwrap(), "export CUSTOM=value\n")?;

    // Act
    guard.create_sops_envs()?;

    // Assert
    assert!(custom_env.exists(), "Custom env file should be preserved");
    assert_eq!(
        fs::read_to_string(&custom_env)?,
        "export CUSTOM=value\n",
        "Custom env content should be preserved"
    );
    assert!(
        env_dir.join("local.env").exists(),
        "local.env should still be created"
    );

    Ok(())
}

#[rstest]
fn test_create_sops_envs_file_permissions(
    initialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, guard) = initialized_guard;

    // Act
    guard.create_sops_envs()?;

    // Assert
    let local_env = guard
        .target_dir
        .as_ref()
        .unwrap()
        .join("environments")
        .join("local.env");
    let rsenv_sh = test_dir.join(".idea/runConfigurations/rsenv.sh");

    // Check local.env permissions (should be regular file)
    let local_env_mode = local_env.metadata()?.permissions().mode() & 0o777;
    assert_eq!(
        local_env_mode, 0o644,
        "local.env should have regular file permissions"
    );

    // Check rsenv.sh permissions (should be executable)
    let rsenv_mode = rsenv_sh.metadata()?.permissions().mode() & 0o777;
    assert_eq!(
        rsenv_mode, 0o755,
        "rsenv.sh should have executable permissions"
    );

    Ok(())
}

#[rstest]
fn test_create_sops_envs_environment_structure(
    initialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (_, guard) = initialized_guard;

    // Act
    guard.create_sops_envs()?;

    // Assert
    let local_env = guard
        .target_dir
        .as_ref()
        .unwrap()
        .join("environments")
        .join("local.env");
    let content = fs::read_to_string(&local_env)?;

    // Verify local.env structure
    assert!(
        content.contains("export"),
        "Should contain environment exports"
    );
    // Add more specific checks based on your template content

    Ok(())
}
