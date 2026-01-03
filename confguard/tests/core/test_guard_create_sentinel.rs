use confguard::core::{ConfGuard, ConfGuardBuilder};
use confguard::errors::ConfGuardResult;
use confguard::util::helper::testing::setup_test_dir;
use rstest::*;
use std::fs;
use std::path::PathBuf;

#[fixture]
fn test_dir() -> PathBuf {
    setup_test_dir()
}

#[fixture]
fn uninitialized_guard(test_dir: PathBuf) -> (PathBuf, ConfGuard) {
    let confguard_base = test_dir.clone();
    fs::create_dir_all(&confguard_base).unwrap();

    let guard = ConfGuardBuilder::default()
        .version(2)
        .source_dir(test_dir.clone())
        .is_relative(true)
        .confguard_base_dir(confguard_base)
        .sentinel(String::new()) // Empty sentinel
        .build()
        .unwrap();

    (test_dir, guard)
}

#[rstest]
fn given_uninitialized_guard_when_creating_sentinel_then_creates_target_directory(
    uninitialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (_, mut guard) = uninitialized_guard;

    // Act
    guard.create_sentinel()?;

    // Assert
    let target_dir = guard.target_dir.as_ref().unwrap();
    assert!(target_dir.exists(), "Target directory should exist");
    assert!(
        target_dir.starts_with(guard.confguard_base_dir.join("guarded")),
        "Target dir {} should be in guarded subdirectory",
        target_dir.display()
    );
    assert!(
        target_dir.ends_with(&guard.sentinel),
        "Target dir {} should end with sentinel {}",
        target_dir.display(),
        guard.sentinel
    );

    Ok(())
}

#[rstest]
fn given_created_sentinel_when_checking_format_then_validates_hex_suffix(
    uninitialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (_test_dir, mut guard) = uninitialized_guard;

    // Act
    guard.create_sentinel()?;

    // Assert
    let parts: Vec<&str> = guard.sentinel.split('-').collect();
    // Check hex part - sentinel should end with 8-character hex string
    let hex_part = parts[parts.len() - 1];
    assert_eq!(hex_part.len(), 8, "Hex part should be 8 characters");
    assert!(
        hex_part.chars().all(|c| c.is_ascii_hexdigit()),
        "Hex suffix should contain only hexadecimal characters"
    );

    Ok(())
}

#[rstest]
fn given_guard_with_existing_sentinel_when_creating_sentinel_then_returns_error(
    uninitialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (_test_dir, mut guard) = uninitialized_guard;
    guard.sentinel = "test-12345678".to_string();

    // Act
    let result = guard.create_sentinel();

    // Assert
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Sentinel already set"));

    Ok(())
}

#[rstest]
fn test_create_sentinel_existing_directory(
    uninitialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (_test_dir, mut guard) = uninitialized_guard;

    // Create pre-existing sentinel directory
    let sentinel_dir = guard
        .confguard_base_dir
        .join("guarded")
        .join("rs-cg-existing");
    fs::create_dir_all(&sentinel_dir)?;

    // Act
    let result = guard.create_sentinel();

    // Assert
    assert!(result.is_err());
    // assert!(result.unwrap_err().to_string().contains("Sentinel already set"));  // TODO: fix error message
    assert!(guard.target_dir.is_none(), "target_dir should not be set");
    assert!(guard.sentinel.is_empty(), "sentinel should remain empty");

    Ok(())
}

#[rstest]
fn test_create_sentinel_directory_permissions(
    uninitialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    use std::os::unix::fs::PermissionsExt;

    // Arrange
    let (_test_dir, mut guard) = uninitialized_guard;

    // Act
    guard.create_sentinel()?;

    // Assert
    let target_dir = guard.target_dir.as_ref().unwrap();
    let perms = target_dir.metadata()?.permissions();
    let mode = perms.mode() & 0o777;
    assert!(mode & 0o700 == 0o700, "Directory should be user accessible");

    Ok(())
}

#[rstest]
fn test_create_sentinel_invalid_base_dir(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let mut guard = ConfGuardBuilder::default()
        .version(2)
        .source_dir(test_dir.clone())
        .is_relative(true)
        .confguard_base_dir(test_dir.join("nonexistent"))
        .sentinel(String::new())
        .build()?;

    // Act
    let result = guard.create_sentinel();

    // Assert
    assert!(result.is_ok(), "Should create intermediate directories");
    assert!(
        guard.confguard_base_dir.exists(),
        "Base directory should be created"
    );

    Ok(())
}

#[ignore = "fix it"]
#[rstest]
fn test_create_sentinel_source_name_validation(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange - create guard with source dir containing special characters
    let special_dir = test_dir.join("test@special#dir");
    fs::create_dir_all(&special_dir)?;

    let mut guard = ConfGuardBuilder::default()
        .version(2)
        .source_dir(special_dir)
        .is_relative(true)
        .confguard_base_dir(test_dir.join("confguard_base"))
        .sentinel(String::new())
        .build()?;

    // Act
    guard.create_sentinel()?;

    // Assert
    let sentinel_base = guard.sentinel.split('-').next().unwrap();
    assert!(
        !sentinel_base.contains(|c: char| !c.is_alphanumeric() && c != '-'),
        "Sentinel should only contain alphanumeric characters"
    );

    Ok(())
}
