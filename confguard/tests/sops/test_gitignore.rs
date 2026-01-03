use confguard::errors::ConfGuardResult;
use confguard::sops::gitignore::*;
use confguard::util::testing::setup_test_dir;
use rstest::*;
use std::fs;
use std::path::PathBuf;

#[fixture]
fn test_dir() -> PathBuf {
    setup_test_dir()
}

#[rstest]
fn test_gitignore_management(test_dir: PathBuf) -> ConfGuardResult<()> {
    let manager = GitignoreManager::new(&test_dir);

    // Test adding entries
    let extensions = vec!["env".to_string(), "yaml".to_string()];
    let filenames = vec![".env".to_string(), ".envrc".to_string()];
    manager.update_entries(&extensions, &filenames)?;

    // Verify content
    let content = fs::read_to_string(&manager.gitignore_path)?;
    assert!(content.contains("*.env"));
    assert!(content.contains("*.yaml"));
    assert!(content.contains(".envrc"));
    assert!(content.contains("confguard-start"));

    // Test cleaning entries
    manager.clean_entries()?;
    let content = fs::read_to_string(&manager.gitignore_path)?;
    assert!(!content.contains("*.env"));
    assert!(!content.contains("*.yaml"));
    assert!(!content.contains(".envrc"));
    assert!(!content.contains("confguard-start"));

    // teardown_test_dir(&test_dir);
    Ok(())
}

#[rstest]
fn test_split_sections(test_dir: PathBuf) -> ConfGuardResult<()> {
    let manager = GitignoreManager::new(&test_dir);

    // Create initial content
    let content = format!(
        "before\n{}\nmiddle\n{}\nafter",
        manager.section_start, manager.section_end
    );

    // Test section splitting
    let (pre, section, post) = manager.split_sections(content.as_str());
    assert_eq!(pre, vec!["before"]);
    assert_eq!(section, vec!["middle"]);
    assert_eq!(post, vec!["after"]);

    // teardown_test_dir(&test_dir);
    Ok(())
}

#[rstest]
fn test_update_entries_preserves_structure(test_dir: PathBuf) -> ConfGuardResult<()> {
    let manager = GitignoreManager::new(&test_dir);

    // Create initial content with existing sections
    let initial_content = "# Project ignores\n*.log\n\n# Dependencies\n/target/\n";
    fs::write(&manager.gitignore_path, initial_content)?;

    // Update with new entries
    let extensions = vec!["env".to_string()];
    let filenames = vec![".env".to_string()];
    manager.update_entries(&extensions, &filenames)?;

    // Verify structure preservation
    let content = fs::read_to_string(&manager.gitignore_path)?;
    assert!(content.starts_with("# Project ignores"));
    assert!(content.contains("*.log"));
    assert!(content.contains("/target/"));
    assert!(content.contains("*.env"));
    assert!(content.contains(".env"));

    // teardown_test_dir(&test_dir);
    Ok(())
}
