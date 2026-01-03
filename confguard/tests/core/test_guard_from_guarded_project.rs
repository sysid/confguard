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
fn guarded_project(test_dir: PathBuf) -> (PathBuf, ConfGuard) {
    // Create and guard a project
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(envrc_path.to_str().unwrap(), "export FOO=bar\n").unwrap();

    let mut guard = ConfGuard::new_for_guarding(test_dir.clone(), true).unwrap();

    // This will create the sentinel in the guarded subdirectory
    guard.guard(false).unwrap();

    // Verify the target directory structure before returning
    assert!(
        guard
            .target_dir
            .as_ref()
            .unwrap()
            .ends_with(&guard.sentinel),
        "Target dir should end with sentinel"
    );
    assert!(
        guard
            .target_dir
            .as_ref()
            .unwrap()
            .starts_with(guard.confguard_base_dir.join("guarded")),
        "Target dir should be in guarded subdirectory"
    );
    assert!(
        guard.target_dir.as_ref().unwrap().exists(),
        "Target directory should exist"
    );

    (test_dir, guard)
}

#[rstest]
fn test_from_guarded_project_success(guarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, original_guard) = guarded_project;

    // Act
    let loaded_guard = ConfGuard::from_guarded_project(&test_dir)?;

    // Assert
    assert_eq!(loaded_guard.version, original_guard.version);
    assert_eq!(loaded_guard.source_dir, original_guard.source_dir);
    assert_eq!(loaded_guard.is_relative, original_guard.is_relative);
    assert_eq!(loaded_guard.sentinel, original_guard.sentinel);

    // First verify the original guard has correct target dir
    let expected_target = original_guard
        .confguard_base_dir
        .join("guarded")
        .join(&original_guard.sentinel);
    assert_eq!(
        original_guard.target_dir.unwrap(),
        expected_target,
        "Original guard target dir mismatch"
    );

    // Then verify loaded guard matches
    assert_eq!(
        loaded_guard.target_dir.unwrap(),
        expected_target,
        "Loaded guard target dir mismatch"
    );

    Ok(())
}

#[rstest]
fn test_from_guarded_project_unguarded(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(
        envrc_path.to_str().unwrap(),
        "export FOO=bar\n", // No confguard section
    )?;

    // Act
    let result = ConfGuard::from_guarded_project(&test_dir);

    // Assert
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not guarded"));
    // println!("{}", result.unwrap_err());

    Ok(())
}

#[rstest]
fn test_from_guarded_project_no_envrc(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Act
    let result = ConfGuard::from_guarded_project(&test_dir);

    // Assert
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No .envrc file found"));

    Ok(())
}

#[rstest]
fn test_from_guarded_project_invalid_envrc(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(
        envrc_path.to_str().unwrap(),
        "#------------------------------- confguard start --------------------------------\n\
         # Invalid content\n\
         #-------------------------------- confguard end ---------------------------------\n",
    )?;

    // Act
    let result = ConfGuard::from_guarded_project(&test_dir);

    // Assert
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not guarded"));

    Ok(())
}

#[rstest]
fn test_from_guarded_project_broken_link(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    let target_path = test_dir.join("nonexistent_target");
    std::os::unix::fs::symlink(&target_path, &envrc_path)?;

    // Act
    let result = ConfGuard::from_guarded_project(&test_dir);

    // Assert
    // println!("{}", result.unwrap_err());
    // should fail because .envrc is pointing to non existing file (path.exsists())
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No .envrc file found"));

    Ok(())
}

#[rstest]
fn test_from_guarded_project_missing_target_dir(
    guarded_project: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, guard) = guarded_project;

    // Remove target directory
    fs::remove_dir_all(guard.target_dir.unwrap())?;

    // Act
    let result = ConfGuard::from_guarded_project(&test_dir);

    // Assert
    println!("{:?}", result);
    // should fail because .envrc is pointing to non existing file (path.exsists())
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No .envrc file found"));

    Ok(())
}
