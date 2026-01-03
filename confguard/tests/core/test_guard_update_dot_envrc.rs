use chrono::DateTime;
use confguard::core::{ConfGuard, ConfGuardBuilder};
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
fn initialized_guard(test_dir: PathBuf) -> (PathBuf, ConfGuard) {
    let mut guard = ConfGuardBuilder::default()
        .version(2)
        .source_dir(test_dir.clone())
        .is_relative(true)
        .confguard_base_dir(test_dir.join("confguard_base"))
        .sentinel("test-12345678".to_string())
        .build()
        .unwrap();

    guard.target_dir = Some(test_dir.join("confguard_base").join("test-12345678"));
    fs::create_dir_all(guard.target_dir.as_ref().unwrap()).unwrap();

    (test_dir, guard)
}

#[rstest]
fn test_update_dot_envrc_new_file(initialized_guard: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, guard) = initialized_guard;
    let target_file = test_dir.join("dot.envrc");
    create_file_with_content(target_file.to_str().unwrap(), "export FOO=bar\n")?;

    // Act
    guard.update_dot_envrc(&target_file)?;

    // Assert
    let content = fs::read_to_string(&target_file)?;

    // Check original content preserved
    assert!(
        content.contains("export FOO=bar"),
        "Original content should be preserved"
    );

    // Check confguard section
    assert!(
        content.contains("confguard start"),
        "Should have start marker"
    );
    assert!(content.contains("confguard end"), "Should have end marker");
    assert!(
        content.contains("config.relative = true"),
        "Should have relative config"
    );
    assert!(
        content.contains("config.version = 2"),
        "Should have version"
    );
    assert!(
        content.contains(&format!("state.sentinel = '{}'", guard.sentinel)),
        "Should have sentinel"
    );

    // Check SOPS_PATH
    assert!(
        content.contains("export SOPS_PATH=$HOME"),
        "Should have SOPS_PATH"
    );
    assert!(
        content.contains("dotenv $SOPS_PATH/environments/local.env"),
        "Should have dotenv command"
    );

    // Verify timestamp format
    let timestamp_line = content
        .lines()
        .find(|l| l.contains("state.timestamp"))
        .expect("Should contain timestamp");
    let timestamp_str = timestamp_line
        .split('\'')
        .nth(1)
        .expect("Should have timestamp value");
    DateTime::parse_from_rfc3339(timestamp_str).expect("Should be valid RFC3339 timestamp");

    Ok(())
}

#[rstest]
fn test_update_dot_envrc_existing_section(
    initialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, guard) = initialized_guard;
    let target_file = test_dir.join("dot.envrc");
    let initial_content = "\
        export FOO=bar\n\
        #------------------------------- confguard start --------------------------------\n\
        # config.relative = false\n\
        # config.version = 1\n\
        # state.sentinel = 'old-sentinel'\n\
        # state.timestamp = '2023-01-01T00:00:00Z'\n\
        export SOPS_PATH=$HOME/old/path\n\
        #-------------------------------- confguard end ---------------------------------\n\
        export BAZ=qux\n";
    create_file_with_content(target_file.to_str().unwrap(), initial_content)?;

    // Act
    guard.update_dot_envrc(&target_file)?;

    // Assert
    let content = fs::read_to_string(&target_file)?;

    // Check content preservation
    assert!(
        content.contains("export FOO=bar"),
        "Content before section should be preserved"
    );
    assert!(
        content.contains("export BAZ=qux"),
        "Content after section should be preserved"
    );

    // Check section update
    assert!(
        content.contains(&format!("state.sentinel = '{}'", guard.sentinel)),
        "Should have new sentinel"
    );
    assert!(
        !content.contains("old-sentinel"),
        "Old sentinel should be removed"
    );
    assert!(
        content.contains("config.version = 2"),
        "Should have new version"
    );

    Ok(())
}

#[rstest]
fn test_update_dot_envrc_malformed_section(
    initialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, guard) = initialized_guard;
    let target_file = test_dir.join("dot.envrc");
    let initial_content = "\
        export FOO=bar\n\
        #------------------------------- confguard start --------------------------------\n\
        # Malformed content\n\
        export BAZ=qux\n";
    create_file_with_content(target_file.to_str().unwrap(), initial_content)?;

    // Act
    guard.update_dot_envrc(&target_file)?;

    // Assert
    let content = fs::read_to_string(&target_file)?;

    // Should append new section
    assert!(
        content.contains("export FOO=bar"),
        "Original content should be preserved"
    );
    assert!(
        content.contains("# Malformed content"),
        "Malformed section should be preserved"
    );
    assert!(
        content.contains(&format!("state.sentinel = '{}'", guard.sentinel)),
        "Should append new confguard section"
    );

    Ok(())
}

#[rstest]
fn test_update_dot_envrc_path_conversion(
    initialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, guard) = initialized_guard;
    let target_file = test_dir.join("dot.envrc");
    create_file_with_content(target_file.to_str().unwrap(), "export FOO=bar\n")?;

    // Act
    guard.update_dot_envrc(&target_file)?;

    // Assert
    let content = fs::read_to_string(&target_file)?;

    // Check path formats
    assert!(content.contains("$HOME"), "Should use $HOME variable");
    assert!(
        !content.contains("/Users/"),
        "Should not contain absolute home path"
    );
    assert!(
        content.contains("environments/local.env"),
        "Should have correct environments path"
    );

    Ok(())
}

#[ignore = "Needs fixing"]
#[rstest]
fn test_update_dot_envrc_nonexistent_file(
    initialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, guard) = initialized_guard;
    let nonexistent_file = test_dir.join("nonexistent.envrc");

    // Act
    let result = guard.update_dot_envrc(&nonexistent_file);

    // Assert
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Cannot read envrc file"));

    Ok(())
}

#[rstest]
fn test_update_dot_envrc_preserves_line_endings(
    initialized_guard: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, guard) = initialized_guard;
    let target_file = test_dir.join("dot.envrc");
    create_file_with_content(
        target_file.to_str().unwrap(),
        "export FOO=bar\r\nexport BAZ=qux\r\n",
    )?;

    // Act
    guard.update_dot_envrc(&target_file)?;

    // Assert
    let content = fs::read_to_string(&target_file)?;
    assert!(
        content.contains("\r\n"),
        "Should preserve CRLF line endings"
    );

    Ok(())
}
