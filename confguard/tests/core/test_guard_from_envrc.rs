use confguard::core::ConfGuard;
use confguard::errors::ConfGuardResult;
use confguard::util::helper::testing::setup_test_dir;
use confguard::util::testing::{create_file_with_content, generate_envrc_content};
use rstest::*;
use std::path::PathBuf;

#[fixture]
fn test_dir() -> PathBuf {
    setup_test_dir()
}

#[rstest]
fn test_from_envrc_unguarded_empty_file(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(envrc_path.to_str().unwrap(), "")?;

    // Act
    let result = ConfGuard::from_envrc(&envrc_path)?;

    // Assert
    assert!(
        result.is_none(),
        "Empty file should not be considered guarded"
    );

    Ok(())
}

#[rstest]
fn test_from_envrc_unguarded_no_section(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(
        envrc_path.to_str().unwrap(),
        "export FOO=bar\nexport BAZ=qux\n",
    )?;

    // Act
    let result = ConfGuard::from_envrc(&envrc_path)?;

    // Assert
    assert!(
        result.is_none(),
        "File without confguard section should not be considered guarded"
    );

    Ok(())
}

#[rstest]
fn test_from_envrc_valid_guarded(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    let content = generate_envrc_content(&test_dir)?;
    create_file_with_content(envrc_path.to_str().unwrap(), &content)?;

    // Act
    let result = ConfGuard::from_envrc(&envrc_path)?;

    // Assert
    assert!(result.is_some(), "Valid guarded file should be detected");
    let guard = result.unwrap();
    assert_eq!(guard.version, 2);
    assert!(guard.is_relative);
    assert_eq!(guard.sentinel, "test-12345678");
    assert_eq!(guard.source_dir, test_dir);

    Ok(())
}

#[rstest]
fn test_from_envrc_incomplete_section(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    let content =
        "#------------------------------- confguard start --------------------------------
export FOO=bar
#-------------------------------- confguard end ---------------------------------";
    create_file_with_content(envrc_path.to_str().unwrap(), content)?;

    // Act
    let result = ConfGuard::from_envrc(&envrc_path)?;

    // Assert
    assert!(
        result.is_none(),
        "Incomplete section should not be considered guarded"
    );

    Ok(())
}

#[rstest]
fn test_from_envrc_partial_markers(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    let content =
        "#------------------------------- confguard start --------------------------------
# config.relative = true
# config.version = 2
export FOO=bar"; // Missing end marker
    create_file_with_content(envrc_path.to_str().unwrap(), content)?;

    // Act
    let result = ConfGuard::from_envrc(&envrc_path)?;

    // Assert
    assert!(
        result.is_none(),
        "File with only start marker should not be considered guarded"
    );

    Ok(())
}

#[rstest]
fn test_from_envrc_nonexistent_file(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join("nonexistent.envrc");

    // Act
    let result = ConfGuard::from_envrc(&envrc_path);

    // Assert
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Cannot read envrc file"));

    Ok(())
}
