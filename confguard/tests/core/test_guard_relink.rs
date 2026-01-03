use confguard::core::ConfGuard;
use confguard::errors::ConfGuardResult;
use confguard::util::helper::testing::setup_test_dir;
use confguard::util::testing::create_file_with_content;
use rstest::*;
use std::fs;
use std::path::PathBuf;

#[fixture]
fn test_dir() -> PathBuf {
    setup_test_dir()
}

#[fixture]
fn guarded_project(test_dir: PathBuf) -> (PathBuf, PathBuf, ConfGuard) {
    // Create a subdirectory to avoid interference with test_dir
    let project_dir = test_dir.join("project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create and guard a project
    let envrc_path = project_dir.join(".envrc");
    create_file_with_content(envrc_path.to_str().unwrap(), "export FOO=bar\n").unwrap();

    let mut guard = ConfGuard::new_for_guarding(project_dir.clone(), true).unwrap();
    guard.guard(false).unwrap();

    let target_envrc = guard.target_dir.as_ref().unwrap().join("dot.envrc");

    (project_dir, target_envrc, guard)
}

#[rstest]
fn test_relink_success(guarded_project: (PathBuf, PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (project_dir, target_envrc, _) = guarded_project;
    let envrc_path = project_dir.join(".envrc");

    // Store initial target for verification
    let initial_target = fs::read_link(&envrc_path)?;

    // Remove existing link
    fs::remove_file(&envrc_path)?;
    assert!(!envrc_path.exists(), "Setup verification failed");

    // Act
    ConfGuard::relink(&target_envrc)?;

    // Assert
    assert!(envrc_path.exists(), "Link should exist");
    assert!(
        envrc_path.symlink_metadata()?.file_type().is_symlink(),
        "Should be a symlink"
    );

    // Compare with initial target
    let new_target = fs::read_link(&envrc_path)?;
    assert_eq!(
        new_target, initial_target,
        "Link should point to the same target as before"
    );

    Ok(())
}

#[rstest]
fn test_relink_already_linked(
    guarded_project: (PathBuf, PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (_, target_envrc, _) = guarded_project;

    // Act
    let result = ConfGuard::relink(&target_envrc);

    // Assert
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Link already exists"));

    Ok(())
}

#[rstest]
fn test_relink_different_target(
    guarded_project: (PathBuf, PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (project_dir, target_envrc, _) = guarded_project;
    let envrc_path = project_dir.join(".envrc");
    let wrong_target = project_dir.join("wrong_target");

    // Remove existing link and create one pointing elsewhere
    fs::remove_file(&envrc_path)?;
    create_file_with_content(wrong_target.to_str().unwrap(), "wrong content")?;
    std::os::unix::fs::symlink(&wrong_target, &envrc_path)?;

    // Act
    let result = ConfGuard::relink(&target_envrc);

    // Assert
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("points elsewhere"));

    Ok(())
}

#[rstest]
fn test_relink_invalid_target(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let nonexistent = test_dir.join("nonexistent.envrc");

    // Act
    let result = ConfGuard::relink(&nonexistent);

    // Assert
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("File not found"));

    Ok(())
}

#[rstest]
fn test_relink_unguarded_target(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join("unguarded.envrc");
    create_file_with_content(
        envrc_path.to_str().unwrap(),
        "export FOO=bar\n", // No confguard section
    )?;

    // Act
    let result = ConfGuard::relink(&envrc_path);

    // Assert
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Not a guarded envrc file"));

    Ok(())
}

#[rstest]
fn test_relink_permissions(guarded_project: (PathBuf, PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    use std::os::unix::fs::PermissionsExt;

    // Arrange
    let (project_dir, target_envrc, _) = guarded_project;
    let envrc_path = project_dir.join(".envrc");
    fs::remove_file(&envrc_path)?;

    // Set specific permissions on target
    let mode = 0o644;
    fs::set_permissions(&target_envrc, fs::Permissions::from_mode(mode))?;

    // Act
    ConfGuard::relink(&target_envrc)?;

    // Assert
    assert!(envrc_path.exists(), "Link should exist");
    let target_perms = target_envrc.metadata()?.permissions();
    assert_eq!(
        target_perms.mode() & 0o777,
        mode,
        "Target permissions should be preserved"
    );

    Ok(())
}
