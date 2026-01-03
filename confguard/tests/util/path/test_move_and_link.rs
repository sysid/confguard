use std::env;
// tests/utils/test_move_and_link.rs
use confguard::errors::ConfGuardResult;
use confguard::util::helper::testing::setup_test_dir;
use confguard::util::path::move_and_link::MoveAndLink;
use std::fs::{self, File};
use std::io::Write;

#[test]
fn test_move_and_link_basic() -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();

    // Create source file
    let source_path = test_dir.join("source.txt");
    let mut file = File::create(&source_path)?;
    writeln!(file, "test content")?;

    let dest_path = test_dir.join("dest/source.txt");

    // Create and execute MoveAndLink
    let move_link = MoveAndLink::new(&source_path, &dest_path)?;
    move_link.move_and_link(true)?;

    // Verify
    assert!(source_path.exists());
    assert!(source_path.symlink_metadata()?.file_type().is_symlink());
    assert!(dest_path.exists());
    assert!(!dest_path.symlink_metadata()?.file_type().is_symlink());

    // Verify content
    let content = fs::read_to_string(&source_path)?;
    assert_eq!(content.trim(), "test content");

    // teardown_test_dir(&test_dir);
    Ok(())
}

#[test]
fn test_move_and_link_revert() -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();

    // Create source file
    let source_path = test_dir.join("source.txt");
    let mut file = File::create(&source_path)?;
    writeln!(file, "test content")?;
    let _original_metadata = fs::metadata(&source_path)?;

    let dest_path = test_dir.join("dest/source.txt");

    // Execute move_and_link
    let move_link = MoveAndLink::new(&source_path, &dest_path)?;
    move_link.move_and_link(true)?;

    // Revert the operation
    move_link.revert()?;

    // Verify
    assert!(source_path.exists());
    assert!(!source_path.symlink_metadata()?.file_type().is_symlink());
    assert!(!dest_path.exists());

    // Verify content remains intact
    let content = fs::read_to_string(&source_path)?;
    assert_eq!(content.trim(), "test content");

    // teardown_test_dir(&test_dir);
    Ok(())
}

#[test]
fn test_move_and_link_relative_vs_absolute() -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();

    // Test with relative paths
    let source_path = test_dir.join("rel_source.txt");
    let mut file = File::create(&source_path)?;
    writeln!(file, "test content")?;

    let dest_path = test_dir.join("dest/rel_source.txt");

    // Save the original current directory, to restore it later
    let original_dir = env::current_dir()?;
    env::set_current_dir(&test_dir)?;

    // Should work with relative paths
    let move_link = MoveAndLink::new("rel_source.txt", "dest/rel_source.txt")?;
    move_link.move_and_link(true)?;

    // Verify
    assert!(source_path.exists());
    assert!(dest_path.exists());

    // After executing your code, restore the original current directory
    env::set_current_dir(original_dir)?;
    // teardown_test_dir(&test_dir);
    Ok(())
}

#[test]
fn test_move_and_link_error_non_existing_source() -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();

    // Save the original current directory, to restore it later
    let original_dir = env::current_dir()?;
    env::set_current_dir(&test_dir)?;

    // Test nonexistent source
    let result = MoveAndLink::new(test_dir.join("nonexistent.txt"), test_dir.join("dest.txt"));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));

    env::set_current_dir(original_dir)?;
    // teardown_test_dir(&test_dir);
    Ok(())
}
#[test]
fn test_move_and_link_errors_source_is_link() -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();

    // Save the original current directory, to restore it later
    let original_dir = env::current_dir()?;
    env::set_current_dir(&test_dir)?;

    // Test with existing symlink as source
    let source_path = test_dir.join("symlink_source.txt");
    let link_path = test_dir.join("existing_link.txt");
    let mut file = File::create(&source_path)?;
    writeln!(file, "test content")?;
    std::os::unix::fs::symlink(&source_path, &link_path)?;

    let result = MoveAndLink::new(&link_path, test_dir.join("dest.txt"));
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("already a symbolic link"));

    env::set_current_dir(original_dir)?;
    // teardown_test_dir(&test_dir);
    Ok(())
}

#[test]
fn test_move_and_link_directory_with_special_files() -> ConfGuardResult<()> {
    // Arrange
    let test_dir = setup_test_dir();

    // Create source directory with nested structure
    let source_dir = test_dir.join("source_dir");
    fs::create_dir_all(source_dir.join("subdir"))?;

    // Create regular file
    let mut file1 = File::create(source_dir.join("file1.txt"))?;
    writeln!(file1, "content 1")?;

    // Create symlink within directory
    let target_file = source_dir.join("target.txt");
    let mut file2 = File::create(&target_file)?;
    writeln!(file2, "target content")?;
    let symlink = source_dir.join("link.txt");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target_file, &symlink)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&target_file, &symlink)?;

    let dest_dir = test_dir.join("dest/source_dir");

    // Act
    let move_link = MoveAndLink::new(&source_dir, &dest_dir)?;
    move_link.move_and_link(true)?;

    // Assert
    // Verify internal symlink still works
    let linked_symlink = source_dir.join("link.txt");
    assert!(linked_symlink.exists());
    let content = fs::read_to_string(&linked_symlink)?;
    assert_eq!(content.trim(), "target content");

    // Verify revert
    move_link.revert()?;
    assert!(source_dir.join("link.txt").exists());
    let content = fs::read_to_string(source_dir.join("link.txt"))?;
    assert_eq!(content.trim(), "target content");

    Ok(())
}
