use confguard::errors::ConfGuardResult;
use confguard::sops::manager::{read_config, SopsConfig, SopsManager};
use confguard::util::testing::{create_file_with_content, setup_test_dir, teardown_test_dir};
use rstest::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[fixture]
fn test_dir() -> PathBuf {
    setup_test_dir()
}

#[fixture]
fn test_config() -> SopsConfig {
    SopsConfig {
        file_extensions_enc: vec!["env".to_string(), "yaml".to_string()],
        file_names_enc: vec![".envrc".to_string()],
        file_extensions_dec: vec!["enc".to_string()],
        file_names_dec: vec![],
        gpg_key: "60A4127E82E218297532FAB6D750B66AE08F3B90".to_string(),
    }
}

#[fixture]
pub fn toml_config() -> String {
    r#"
# confguard.toml

file_extensions_enc = [
    "envrc",
    "env",
    "yaml"
]

file_names_enc = [
    "dot_pypirc",
    "dot_pgpass",
    "kube_config",
    "dot.m2.settings.xml",
    "http-client.private.env.json"
]

file_extensions_dec = [
    "enc"
]

file_names_dec = []

gpg_key = "60A4127E82E218297532FAB6D750B66AE08F3B90"
"#
    .to_string()
}

#[fixture]
fn confguard_config_file(toml_config: String) -> PathBuf {
    let test_dir = setup_test_dir();
    let toml_path = test_dir.join("confguard.toml");
    fs::write(&toml_path, toml_config).expect("Failed to write toml config in test setup");
    toml_path
}

#[rstest]
fn test_read_config(toml_config: String) -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();
    let toml_path = test_dir.join("confguard.toml");
    fs::write(&toml_path, toml_config)?;

    let config = read_config(toml_path.as_path())?;
    assert_eq!(config.file_extensions_enc, vec!["envrc", "env", "yaml"]);
    assert_eq!(
        config.file_names_enc,
        vec![
            "dot_pypirc",
            "dot_pgpass",
            "kube_config",
            "dot.m2.settings.xml",
            "http-client.private.env.json"
        ]
    );
    assert_eq!(config.file_extensions_dec, vec!["enc"]);
    // assert_eq!(config.file_names_dec, vec![]);

    // fs::remove_file(toml_path)?;
    Ok(())
}

#[rstest]
fn test_collect_files(test_dir: PathBuf, confguard_config_file: PathBuf) -> ConfGuardResult<()> {
    // Create test files
    create_file_with_content(test_dir.join("test.env").to_str().unwrap(), "SECRET=test")?;
    create_file_with_content(test_dir.join(".envrc").to_str().unwrap(), "export FOO=bar")?;
    create_file_with_content(
        test_dir.join("xxx.not-to-be-collected").to_str().unwrap(),
        "export FOO=bar",
    )?;

    let manager = SopsManager::new(confguard_config_file.as_path())?;

    let files = manager.collect_files(&test_dir, &["env".to_string()], &[".envrc".to_string()])?;

    assert_eq!(files.len(), 2);

    teardown_test_dir(&test_dir);
    Ok(())
}

#[ignore = "must be run from terminal/make due to password input"]
#[rstest]
fn test_full_encryption_cycle(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    // Create test files
    create_file_with_content(
        test_dir.join("secrets.env").to_str().unwrap(),
        "SECRET=test",
    )?;
    create_file_with_content(test_dir.join("config.yaml").to_str().unwrap(), "key: value")?;

    let manager = SopsManager::new(confguard_config_file.as_path())?;

    // Test encryption
    manager.encrypt_files(Some(&test_dir))?;
    assert!(test_dir.join("secrets.env.enc").exists());
    assert!(test_dir.join("config.yaml.enc").exists());

    // Test cleaning
    manager.clean_files(Some(&test_dir))?;
    assert!(!test_dir.join("secrets.env").exists());
    assert!(!test_dir.join("config.yaml").exists());

    // Test decryption
    manager.decrypt_files(Some(&test_dir))?;
    let decrypted_env = fs::read_to_string(test_dir.join("secrets.env"))?;
    assert_eq!(decrypted_env.trim(), "SECRET=test");

    // teardown_test_dir(&test_dir);
    Ok(())
}

#[rstest]
fn test_gitignore_integration(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    let manager = SopsManager::new(confguard_config_file.as_path())?;
    manager.encrypt_files(None)?;

    // Verify .gitignore was updated
    let gitignore_content = fs::read_to_string(test_dir.join(".gitignore"))?;
    assert!(gitignore_content.contains("*.env"));
    assert!(gitignore_content.contains("*.yaml"));
    assert!(gitignore_content.contains(".envrc"));

    // teardown_test_dir(&test_dir);
    Ok(())
}

#[rstest]
fn test_clean_files_basic(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    // Arrange
    let test_files = [
        ("test1.env", "SECRET1=test"),
        ("test2.yaml", "key: value"),
        ("dot.envrc", "export FOO=bar"),
        ("ignore.txt", "should not be removed"),
    ];

    for (name, content) in &test_files {
        create_file_with_content(test_dir.join(name).to_str().unwrap(), content)?;
    }

    let manager = SopsManager::new(&confguard_config_file)?;

    // Act
    manager.clean_files(Some(&test_dir))?;

    // Assert
    assert!(
        !test_dir.join("test1.env").exists(),
        "env file should be removed"
    );
    assert!(
        !test_dir.join("test2.yaml").exists(),
        "yaml file should be removed"
    );
    assert!(
        !test_dir.join("dot.envrc").exists(),
        "envrc file should be removed"
    );
    assert!(
        test_dir.join("ignore.txt").exists(),
        "non-matching file should remain"
    );

    Ok(())
}

#[rstest]
fn test_clean_files_nested_directories(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    // Arrange
    let nested_dir = test_dir.join("nested/deep");
    fs::create_dir_all(&nested_dir)?;

    let test_files = [
        ("test.env", "SECRET=test"),
        ("nested/test.yaml", "key: value"),
        ("nested/deep/dot.envrc", "export FOO=bar"),
    ];

    for (path, content) in &test_files {
        create_file_with_content(test_dir.join(path).to_str().unwrap(), content)?;
    }

    let manager = SopsManager::new(&confguard_config_file)?;

    // Act
    manager.clean_files(Some(&test_dir))?;

    // Assert
    for (path, _) in &test_files {
        assert!(
            !test_dir.join(path).exists(),
            "File should be removed: {}",
            path
        );
    }

    // Directories should remain even if empty
    assert!(nested_dir.exists(), "Directory structure should remain");

    Ok(())
}

#[rstest]
fn test_clean_files_no_matching_files(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    // Arrange
    create_file_with_content(
        test_dir.join("test.txt").to_str().unwrap(),
        "not a matching file",
    )?;

    let manager = SopsManager::new(&confguard_config_file)?;

    // Act
    manager.clean_files(Some(&test_dir))?;

    // Assert
    assert!(
        test_dir.join("test.txt").exists(),
        "Non-matching file should remain"
    );
    Ok(())
}

#[allow(unused)]
#[rstest]
fn test_clean_files_already_cleaned(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    // Arrange
    let manager = SopsManager::new(&confguard_config_file)?;

    // Act - cleaning files that don't exist should not fail
    let result = manager.clean_files(Some(&test_dir));

    // Assert
    assert!(
        result.is_ok(),
        "Cleaning non-existent files should not error"
    );
    Ok(())
}

#[rstest]
fn test_clean_files_mixed_permissions(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    use std::os::unix::fs::PermissionsExt;

    // Arrange
    let readonly_file = test_dir.join("readonly.env");
    create_file_with_content(readonly_file.to_str().unwrap(), "SECRET=test")?;
    let mut perms = fs::metadata(&readonly_file)?.permissions();
    perms.set_mode(0o444); // Read-only
    fs::set_permissions(&readonly_file, perms)?;

    let manager = SopsManager::new(&confguard_config_file)?;

    // Act
    manager.clean_files(Some(&test_dir))?;

    // Assert
    assert!(!readonly_file.exists(), "Read-only file should be removed");
    Ok(())
}

#[rstest]
fn test_encrypt_with_custom_path(confguard_config_file: PathBuf) -> ConfGuardResult<()> {
    // Create a temporary directory for the test
    let temp_dir = tempdir()?;
    let test_file_path = temp_dir.path().join("test.env");
    fs::write(&test_file_path, "TEST_KEY=test_value")?;

    // Create manager with test configuration
    let manager = SopsManager::new(&confguard_config_file)?;

    // Test encryption with custom path
    manager.encrypt_files(Some(temp_dir.path()))?;

    // Verify encryption
    let encrypted_path = test_file_path.with_extension("env.enc");
    assert!(encrypted_path.exists());

    Ok(())
}
