use confguard::core::ConfGuard;
use confguard::errors::ConfGuardResult;
use confguard::util::helper::testing::setup_test_dir;
use confguard::util::testing::{create_file_with_content, generate_envrc_content};
use rstest::*;
use std::fs;
use std::path::PathBuf;

#[fixture]
fn test_dir() -> PathBuf {
    setup_test_dir()
}

#[rstest]
fn test_new_for_guarding_success(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(
        envrc_path.to_str().unwrap(),
        "export FOO=bar\n", // Simple unguarded .envrc
    )?;

    // Act
    let guard = ConfGuard::new_for_guarding(test_dir.clone(), true)?;

    // Assert
    assert_eq!(guard.source_dir, test_dir);
    assert!(guard.is_relative);
    assert!(guard.sentinel.is_empty());
    assert!(guard.target_dir.is_none());
    assert_eq!(guard.version, confguard::core::settings().version);
    assert!(!guard.confguard_base_dir.as_os_str().is_empty());

    Ok(())
}

#[rstest]
fn test_new_for_guarding_no_envrc(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange - don't create .envrc

    // Act
    let result = ConfGuard::new_for_guarding(test_dir.clone(), true);

    // Assert
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No .envrc file found"));

    Ok(())
}

#[rstest]
fn test_new_for_guarding_already_guarded(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange - create a guarded .envrc
    let envrc_path = test_dir.join(".envrc");
    let content = generate_envrc_content(&test_dir)?;
    create_file_with_content(envrc_path.to_str().unwrap(), &content)?;

    // Act
    let result = ConfGuard::new_for_guarding(test_dir.clone(), true);

    // Assert
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already guarded"));

    Ok(())
}

#[rstest]
fn test_new_for_guarding_symlink_envrc(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let real_envrc = test_dir.join("real.envrc");
    let link_envrc = test_dir.join(".envrc");

    // Create source file and link
    create_file_with_content(real_envrc.to_str().unwrap(), "export FOO=bar\n")?;
    std::os::unix::fs::symlink(&real_envrc, &link_envrc)?;

    // Verify setup
    assert!(link_envrc.exists(), "Symlink should exist");
    assert!(
        link_envrc.symlink_metadata()?.file_type().is_symlink(),
        "Should be a symlink"
    );

    // Act
    let result = ConfGuard::new_for_guarding(test_dir.clone(), true);

    // Assert
    assert!(result.is_err(), "Should fail for symlinked .envrc");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("already guarded"),
        "Wrong error message: {}",
        err
    );
    assert!(
        err.to_string().contains("is a symlink"),
        "Error should mention symlink: {}",
        err
    );

    Ok(())
}

#[rstest]
fn test_new_for_guarding_relative_vs_absolute(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(envrc_path.to_str().unwrap(), "export FOO=bar\n")?;

    // Act
    let relative_guard = ConfGuard::new_for_guarding(test_dir.clone(), true)?;
    let absolute_guard = ConfGuard::new_for_guarding(test_dir.clone(), false)?;

    // Assert
    assert!(relative_guard.is_relative);
    assert!(!absolute_guard.is_relative);

    Ok(())
}

#[rstest]
fn test_new_for_guarding_invalid_source_dir(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let invalid_dir = test_dir.join("nonexistent");

    // Act
    let result = ConfGuard::new_for_guarding(invalid_dir, true);

    // Assert
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No .envrc file found"));

    Ok(())
}

#[cfg(unix)]
#[rstest]
fn test_new_for_guarding_permission_denied(test_dir: PathBuf) -> ConfGuardResult<()> {
    use std::os::unix::fs::PermissionsExt;

    // Arrange
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(envrc_path.to_str().unwrap(), "export FOO=bar\n")?;

    // Remove read permissions
    let metadata = fs::metadata(&envrc_path)?;
    let mut perms = metadata.permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&envrc_path, perms.clone())?;

    // Act
    let result = ConfGuard::new_for_guarding(test_dir.clone(), true);

    // Assert
    assert!(result.is_err());

    // Restore permissions for cleanup
    perms.set_mode(0o644);
    fs::set_permissions(&envrc_path, perms)?;

    Ok(())
}
