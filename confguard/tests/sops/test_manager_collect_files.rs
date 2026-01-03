use confguard::errors::ConfGuardResult;
use confguard::sops::manager::SopsManager;
use confguard::util::testing::{create_file_with_content, setup_test_dir};
use rstest::*;
use std::fs;
use std::path::PathBuf;

#[fixture]
fn test_dir() -> PathBuf {
    setup_test_dir()
}

#[fixture]
fn confguard_config_file(test_dir: PathBuf) -> PathBuf {
    let config = r#"
file_extensions_enc = ["env", "yaml"]
file_names_enc = [".envrc", "config"]
file_extensions_dec = ["enc"]
file_names_dec = []
"#;
    let toml_path = test_dir.join("confguard.toml");
    fs::write(&toml_path, config).expect("Failed to write toml config");
    toml_path
}

#[rstest]
fn test_collect_files_by_extension(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    // Arrange
    let test_files = [
        ("test1.env", "content"),
        ("test2.yaml", "content"),
        ("test3.txt", "content"),     // Should not match
        ("test4.env.bak", "content"), // Should not match
    ];

    for (name, content) in &test_files {
        create_file_with_content(test_dir.join(name).to_str().unwrap(), content)?;
    }

    let manager = SopsManager::new(&confguard_config_file)?;

    // Act
    let files = manager.collect_files(&test_dir, &["env".to_string(), "yaml".to_string()], &[])?;

    // Assert
    assert_eq!(files.len(), 2, "Should find exactly 2 matching files");
    let paths: Vec<String> = files
        .iter()
        .map(|f| f.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(paths.contains(&"test1.env".to_string()));
    assert!(paths.contains(&"test2.yaml".to_string()));

    Ok(())
}

#[rstest]
fn test_collect_files_by_name(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    // Arrange
    let test_files = [
        (".envrc", "content"),
        ("config", "content"),
        ("envrc", "content"),      // Should not match
        (".envrc.bak", "content"), // Should not match
    ];

    for (name, content) in &test_files {
        create_file_with_content(test_dir.join(name).to_str().unwrap(), content)?;
    }

    let manager = SopsManager::new(&confguard_config_file)?;

    // Act
    let files = manager.collect_files(
        &test_dir,
        &[],
        &[".envrc".to_string(), "config".to_string()],
    )?;

    // Assert
    assert_eq!(files.len(), 2, "Should find exactly 2 matching files");
    let paths: Vec<String> = files
        .iter()
        .map(|f| f.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(paths.contains(&".envrc".to_string()));
    assert!(paths.contains(&"config".to_string()));

    Ok(())
}

#[rstest]
fn test_collect_files_nested_directories(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    // Arrange
    let nested_dir = test_dir.join("nested/deep");
    fs::create_dir_all(&nested_dir)?;

    let test_files = [
        ("test1.env", "content"),
        ("nested/test2.yaml", "content"),
        ("nested/deep/.envrc", "content"),
        ("nested/deep/test3.env", "content"),
    ];

    for (path, content) in &test_files {
        create_file_with_content(test_dir.join(path).to_str().unwrap(), content)?;
    }

    let manager = SopsManager::new(&confguard_config_file)?;

    // Act
    let files = manager.collect_files(
        &test_dir,
        &["env".to_string(), "yaml".to_string()],
        &[".envrc".to_string()],
    )?;

    // Assert
    assert_eq!(files.len(), 4, "Should find all matching files recursively");
    let paths: Vec<String> = files
        .iter()
        .map(|f| {
            f.path()
                .strip_prefix(&test_dir)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert!(paths.contains(&"test1.env".to_string()));
    assert!(paths.contains(&"nested/test2.yaml".to_string()));
    assert!(paths.contains(&"nested/deep/.envrc".to_string()));
    assert!(paths.contains(&"nested/deep/test3.env".to_string()));

    Ok(())
}

#[allow(unused)]
#[rstest]
fn test_collect_files_empty_directory(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    // Arrange
    let manager = SopsManager::new(&confguard_config_file)?;

    // Act
    let files = manager.collect_files(&test_dir, &["env".to_string()], &[".envrc".to_string()])?;

    // Assert
    assert!(
        files.is_empty(),
        "Should return empty vec for empty directory"
    );

    Ok(())
}

#[rstest]
fn test_collect_files_no_patterns(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    // Arrange
    create_file_with_content(test_dir.join("test.env").to_str().unwrap(), "content")?;

    let manager = SopsManager::new(&confguard_config_file)?;

    // Act
    let files = manager.collect_files(&test_dir, &[], &[])?;

    // Assert
    assert!(
        files.is_empty(),
        "Should return empty vec when no patterns provided"
    );

    Ok(())
}

#[allow(unused)]
#[rstest]
fn test_collect_files_case_sensitivity(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    let manager = SopsManager::new(&confguard_config_file)?;

    // Arrange - create files in manager's base path
    let test_files = [
        ("test.ENV", "content"),
        ("TEST2.env", "content"),
        (".ENVRC", "content"),
    ];

    for (name, content) in &test_files {
        create_file_with_content(manager.base_path.join(name).to_str().unwrap(), content)?;
    }

    // Verify files are created
    for name in &test_files {
        println!(
            "Created file: {:?} exists: {}",
            name.0,
            manager.base_path.join(name.0).exists()
        );
    }

    // Act
    let files = manager.collect_files(&test_dir, &["env".to_string()], &[".envrc".to_string()])?;

    // Print all collected files for debugging
    println!("\nCollected files:");
    for file in &files {
        println!("Found: {:?}", file.path());
    }

    // Assert
    assert_eq!(files.len(), 1, "Should only match exact case");
    assert_eq!(
        files[0].file_name().to_string_lossy(),
        "TEST2.env",
        "Should match correct case for extension"
    );

    Ok(())
}

#[rstest]
fn test_collect_files_symlinks(
    test_dir: PathBuf,
    confguard_config_file: PathBuf,
) -> ConfGuardResult<()> {
    // Arrange
    create_file_with_content(test_dir.join("original.env").to_str().unwrap(), "content")?;
    std::os::unix::fs::symlink(test_dir.join("original.env"), test_dir.join("link.env"))?;

    let manager = SopsManager::new(&confguard_config_file)?;

    // Act
    let files = manager.collect_files(&test_dir, &["env".to_string()], &[])?;

    // Assert
    assert_eq!(
        files.len(),
        2,
        "Should find both original and symlinked files"
    );
    let paths: Vec<String> = files
        .iter()
        .map(|f| f.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(paths.contains(&"original.env".to_string()));
    assert!(paths.contains(&"link.env".to_string()));

    Ok(())
}
