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
fn envrc_with_section(test_dir: PathBuf) -> PathBuf {
    let envrc_path = test_dir.join(".envrc");
    let content = "export FOO=bar\n\
        #------------------------------- confguard start --------------------------------\n\
        # config.relative = true\n\
        # config.version = 2\n\
        # state.sentinel = 'test-12345678'\n\
        # state.timestamp = '2024-01-01T00:00:00.000Z'\n\
        export SOPS_PATH=$HOME/path\n\
        dotenv $SOPS_PATH/environments/local.env\n\
        #-------------------------------- confguard end ---------------------------------\n\
        export BAZ=qux\n";

    create_file_with_content(envrc_path.to_str().unwrap(), content).unwrap();
    envrc_path
}

#[rstest]
fn test_delete_section_success(envrc_with_section: PathBuf) -> ConfGuardResult<()> {
    // Act
    ConfGuard::delete_section(&envrc_with_section)?;

    // Assert
    let content = fs::read_to_string(&envrc_with_section)?;

    // Check section removal
    assert!(
        !content.contains("confguard start"),
        "Start marker should be removed"
    );
    assert!(
        !content.contains("confguard end"),
        "End marker should be removed"
    );
    assert!(
        !content.contains("config.relative"),
        "Config should be removed"
    );
    assert!(
        !content.contains("SOPS_PATH"),
        "SOPS_PATH should be removed"
    );

    // Check content preservation
    assert!(
        content.contains("export FOO=bar"),
        "Content before section should be preserved"
    );
    assert!(
        content.contains("export BAZ=qux"),
        "Content after section should be preserved"
    );

    Ok(())
}

#[ignore = "need to fix"]
#[rstest]
fn test_delete_section_multiple_sections(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    let content = "\
        #------------------------------- confguard start --------------------------------\n\
        # Section 1\n\
        #-------------------------------- confguard end ---------------------------------\n\
        export MIDDLE=value\n\
        #------------------------------- confguard start --------------------------------\n\
        # Section 2\n\
        #-------------------------------- confguard end ---------------------------------\n";
    create_file_with_content(envrc_path.to_str().unwrap(), content)?;

    // Act
    ConfGuard::delete_section(&envrc_path)?;

    // Assert
    let content = fs::read_to_string(&envrc_path)?;
    assert!(
        !content.contains("confguard start"),
        "All start markers should be removed"
    );
    assert!(
        !content.contains("confguard end"),
        "All end markers should be removed"
    );
    assert!(
        content.contains("export MIDDLE=value"),
        "Middle content should be preserved"
    );

    Ok(())
}

#[rstest]
fn test_delete_section_no_section(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    let content = "export FOO=bar\nexport BAZ=qux\n";
    create_file_with_content(envrc_path.to_str().unwrap(), content)?;

    // Act
    ConfGuard::delete_section(&envrc_path)?;

    // Assert
    let new_content = fs::read_to_string(&envrc_path)?;
    assert_eq!(content, new_content, "Content should remain unchanged");

    Ok(())
}

#[rstest]
fn test_delete_section_empty_file(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(envrc_path.to_str().unwrap(), "")?;

    // Act
    ConfGuard::delete_section(&envrc_path)?;

    // Assert
    let content = fs::read_to_string(&envrc_path)?;
    assert!(content.is_empty(), "File should remain empty");

    Ok(())
}

#[rstest]
fn test_delete_section_partial_section(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    let content = "\
        export FOO=bar\n\
        #------------------------------- confguard start --------------------------------\n\
        # Incomplete section without end marker\n\
        export BAZ=qux\n";
    create_file_with_content(envrc_path.to_str().unwrap(), content)?;

    // Act
    ConfGuard::delete_section(&envrc_path)?;

    // Assert
    let new_content = fs::read_to_string(&envrc_path)?;
    assert_eq!(
        content, new_content,
        "Partial section should remain unchanged"
    );

    Ok(())
}

#[rstest]
fn test_delete_section_whitespace_preservation(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let envrc_path = test_dir.join(".envrc");
    let content = "export FOO=bar\n\n\
        #------------------------------- confguard start --------------------------------\n\
        # Section content\n\
        #-------------------------------- confguard end ---------------------------------\n\n\
        export BAZ=qux\n";
    create_file_with_content(envrc_path.to_str().unwrap(), content)?;

    // Act
    ConfGuard::delete_section(&envrc_path)?;

    // Assert
    let new_content = fs::read_to_string(&envrc_path)?;
    assert!(
        new_content.contains("\n\nexport BAZ=qux"),
        "Should preserve blank lines"
    );

    Ok(())
}

#[rstest]
fn test_delete_section_nonexistent_file(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let nonexistent = test_dir.join("nonexistent.envrc");

    // Act
    let result = ConfGuard::delete_section(&nonexistent);

    // Assert
    assert!(result.is_err());
    // assert!(result.unwrap_err().to_string().contains("Cannot read envrc file"));

    Ok(())
}

#[rstest]
fn test_delete_section_preserves_permissions(envrc_with_section: PathBuf) -> ConfGuardResult<()> {
    use std::os::unix::fs::PermissionsExt;

    // Arrange
    let mode = 0o644;
    fs::set_permissions(&envrc_with_section, fs::Permissions::from_mode(mode))?;

    // Act
    ConfGuard::delete_section(&envrc_with_section)?;

    // Assert
    let perms = envrc_with_section.metadata()?.permissions();
    assert_eq!(
        perms.mode() & 0o777,
        mode,
        "File permissions should be preserved"
    );

    Ok(())
}
