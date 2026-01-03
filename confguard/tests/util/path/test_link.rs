use confguard::errors::ConfGuardResult;
use confguard::util::helper::testing::{setup_test_dir, teardown_test_dir};
use confguard::util::link::{create_link, replace_link_with_target};
use confguard::util::testing::{print_active_env_vars, TEST_ENV_VARS};
use rstest::*;
use std::fs::{self, File};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs as platform_fs;
#[cfg(windows)]
use std::os::windows::fs as platform_fs;
use std::path::PathBuf;
use tracing::debug;

#[fixture]
fn test_dir() -> PathBuf {
    setup_test_dir()
}

#[fixture]
fn test_file(test_dir: PathBuf) -> (PathBuf, PathBuf) {
    let file_path = test_dir.join("test.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "test content").unwrap();

    let link_path = test_dir.join("test_link.txt");
    (file_path, link_path)
}

#[fixture]
fn test_dir_structure(test_dir: PathBuf) -> (PathBuf, PathBuf) {
    let original_dir = test_dir.join("original");
    fs::create_dir(&original_dir).unwrap();

    let link_dir = test_dir.join("link");
    (original_dir, link_dir)
}

#[rstest]
#[case(true)] // relative link
              // #[case(false)]  // relative link
fn test_create_link_file(
    #[case] relative: bool,
    test_file: (PathBuf, PathBuf),
) -> ConfGuardResult<()> {
    let (original_path, link_path) = test_file;

    // Act
    create_link(&original_path, &link_path, relative)?;

    // Assert
    assert!(link_path.exists());
    assert!(link_path.symlink_metadata()?.file_type().is_symlink());

    // Verify link target
    let target = fs::read_link(&link_path)?;
    if relative {
        assert!(target.is_relative());
    } else {
        assert!(target.is_absolute());
    }

    // Verify content through link
    let content = fs::read_to_string(&link_path)?;
    assert_eq!(content.trim(), "test content");

    // Cleanup
    teardown_test_dir(original_path.parent().unwrap());
    Ok(())
}

#[rstest]
#[case(true)] // relative link
#[case(false)] // absolute link
fn test_create_link_directory(
    #[case] relative: bool,
    test_dir_structure: (PathBuf, PathBuf),
) -> ConfGuardResult<()> {
    let (original_dir, link_dir) = test_dir_structure;

    // Create a file in the original directory
    let test_file = original_dir.join("test.txt");
    let mut file = File::create(&test_file)?;
    writeln!(file, "test content")?;

    // Act
    create_link(&original_dir, &link_dir, relative)?;

    // Assert
    assert!(link_dir.exists());
    assert!(link_dir.symlink_metadata()?.file_type().is_symlink());

    // Verify link target
    let target = fs::read_link(&link_dir)?;
    if relative {
        assert!(target.is_relative());
    } else {
        assert!(target.is_absolute());
    }

    // Verify content through link
    let linked_file = link_dir.join("test.txt");
    let content = fs::read_to_string(&linked_file)?;
    assert_eq!(content.trim(), "test content");

    // Cleanup
    teardown_test_dir(original_dir.parent().unwrap());
    Ok(())
}

#[rstest]
fn test_create_link_nonexistent_original() -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();
    let nonexistent = test_dir.join("nonexistent");
    let link_path = test_dir.join("link");

    // Act & Assert
    let result = create_link(&nonexistent, &link_path, true);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
    assert!(!link_path.exists());

    // Cleanup
    teardown_test_dir(&test_dir);
    Ok(())
}

#[rstest]
fn test_create_link_with_nonexistent_parent_dirs() -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();
    let original = test_dir.join("test.txt");
    File::create(&original)?.write_all(b"test content")?;

    let deep_link = test_dir.join("a/b/c/link.txt");

    // Act
    create_link(&original, &deep_link, true)?;

    // Assert
    assert!(deep_link.exists());
    assert!(deep_link.symlink_metadata()?.file_type().is_symlink());
    let content = fs::read_to_string(&deep_link)?;
    assert_eq!(content.trim(), "test content");

    // Cleanup
    teardown_test_dir(&test_dir);
    Ok(())
}

#[rstest]
#[case(true)]
#[case(false)]
fn test_create_link_target_exists(#[case] relative: bool) -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();
    let original = test_dir.join("test.txt");
    let link_path = test_dir.join("link.txt");

    // Create both files
    File::create(&original)?.write_all(b"original content")?;
    File::create(&link_path)?.write_all(b"existing content")?;

    // Act & Assert
    let result = create_link(&original, &link_path, relative);
    assert!(result.is_err());

    // Cleanup
    teardown_test_dir(&test_dir);
    Ok(())
}

#[test]
fn test_replace_link_with_target() -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();
    print_active_env_vars(TEST_ENV_VARS);

    // Create a target file with content
    let target_path = test_dir.join("target.txt");
    let mut target_file = File::create(&target_path)?;
    writeln!(target_file, "test content")?;

    // Create a symlink to the target
    let link_path = test_dir.join("link.txt");
    platform_fs::symlink(&target_path, &link_path)?;

    // Verify initial state
    assert!(link_path.exists());
    assert!(link_path.symlink_metadata()?.file_type().is_symlink());

    // Replace the link
    replace_link_with_target(&link_path)?;

    // Verify the replacement
    assert!(link_path.exists());
    assert!(!link_path.symlink_metadata()?.file_type().is_symlink());

    // Verify content was preserved
    let content = fs::read_to_string(&link_path)?;
    assert_eq!(content.trim(), "test content");

    teardown_test_dir(&test_dir);
    Ok(())
}

#[test]
fn test_replace_nonexistent_link() {
    let test_dir = setup_test_dir();
    let nonexistent = test_dir.join("nonexistent.txt");

    let result = replace_link_with_target(&nonexistent);
    assert!(result.is_err());

    teardown_test_dir(&test_dir);
}

#[test]
fn test_replace_non_link() -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();
    let regular_file = test_dir.join("regular.txt");
    File::create(&regular_file)?;

    let result = replace_link_with_target(&regular_file);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not a symbolic link"));

    teardown_test_dir(&test_dir);
    Ok(())
}

#[test]
fn test_replace_broken_link() -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();

    // Create a symlink to a nonexistent target
    let target_path = test_dir.join("nonexistent_target.txt");
    let link_path = test_dir.join("broken_link.txt");
    platform_fs::symlink(&target_path, &link_path)?;

    let result = replace_link_with_target(&link_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));

    // teardown_test_dir(&test_dir);
    Ok(())
}

#[test]
fn test_replace_dir_link() -> ConfGuardResult<()> {
    let test_dir = setup_test_dir();
    print_active_env_vars(TEST_ENV_VARS);

    debug!("Test directory: {:?}", test_dir);

    // Create a target directory with some content
    let target_dir = test_dir.join("target_dir");
    fs::create_dir_all(&target_dir)?;
    debug!("Created target directory: {:?}", target_dir);

    // Create some files in the target directory
    let file1_path = target_dir.join("file1.txt");
    let mut file1 = File::create(&file1_path)?;
    writeln!(file1, "content 1")?;
    debug!("Created file1: {:?}", file1_path);

    let file2_path = target_dir.join("file2.txt");
    let mut file2 = File::create(&file2_path)?;
    writeln!(file2, "content 2")?;
    debug!("Created file2: {:?}", file2_path);

    // Create a subdirectory with content
    let subdir_path = target_dir.join("subdir");
    fs::create_dir_all(&subdir_path)?;
    let file3_path = subdir_path.join("file3.txt");
    let mut file3 = File::create(&file3_path)?;
    writeln!(file3, "content 3")?;
    debug!("Created file3: {:?}", file3_path);

    // Create a symlink to the directory
    let link_path = test_dir.join("dir_link");
    debug!("Creating symlink: {:?} -> {:?}", link_path, target_dir);

    #[cfg(unix)]
    platform_fs::symlink(&target_dir, &link_path)?;
    #[cfg(windows)]
    platform_fs::symlink_dir(&target_dir, &link_path)?;

    // Verify initial state
    assert!(link_path.exists(), "Link path does not exist");
    assert!(
        link_path.symlink_metadata()?.file_type().is_symlink(),
        "Path is not a symlink"
    );

    // Replace the link
    replace_link_with_target(&link_path)?;

    // Verify the replacement
    assert!(
        link_path.exists(),
        "Link path does not exist after replacement"
    );
    assert!(
        !link_path.symlink_metadata()?.file_type().is_symlink(),
        "Path is still a symlink after replacement"
    );
    assert!(
        link_path.is_dir(),
        "Path is not a directory after replacement"
    );

    // Verify directory structure and content was preserved
    let new_file1_content = fs::read_to_string(link_path.join("file1.txt"))?;
    assert_eq!(
        new_file1_content.trim(),
        "content 1",
        "File1 content mismatch"
    );

    let new_file2_content = fs::read_to_string(link_path.join("file2.txt"))?;
    assert_eq!(
        new_file2_content.trim(),
        "content 2",
        "File2 content mismatch"
    );

    let new_file3_content = fs::read_to_string(link_path.join("subdir/file3.txt"))?;
    assert_eq!(
        new_file3_content.trim(),
        "content 3",
        "File3 content mismatch"
    );

    // teardown_test_dir(&test_dir);
    Ok(())
}
