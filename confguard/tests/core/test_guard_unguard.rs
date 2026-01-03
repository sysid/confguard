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
fn guarded_project(test_dir: PathBuf) -> (PathBuf, ConfGuard) {
    // Create and guard a project
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(
        envrc_path.to_str().unwrap(),
        "export FOO=bar\n# Original content\n",
    )
    .unwrap();

    let mut guard = ConfGuard::new_for_guarding(test_dir.clone(), true).unwrap();
    guard.guard(false).unwrap();

    (test_dir, guard)
}

#[rstest]
fn test_unguard_success(guarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, mut guard) = guarded_project;
    let envrc_path = test_dir.join(".envrc");

    // Verify setup
    assert!(
        envrc_path.symlink_metadata()?.file_type().is_symlink(),
        "Setup verification: .envrc should be a symlink initially"
    );
    assert!(
        guard
            .target_dir
            .as_ref()
            .unwrap()
            .starts_with(guard.confguard_base_dir.join("guarded")),
        "Setup verification: Target dir should be in guarded subdirectory"
    );

    // Act
    guard.unguard()?;

    // Assert
    // 1. Check .envrc is no longer a symlink
    assert!(
        !envrc_path.symlink_metadata()?.file_type().is_symlink(),
        ".envrc should not be a symlink after unguard"
    );

    // 2. Check content is preserved
    let content = fs::read_to_string(&envrc_path)?;
    assert!(
        content.contains("Original content"),
        "Original content should be preserved"
    );
    assert!(
        !content.contains("confguard start"),
        "Confguard section should be removed"
    );
    assert!(
        !content.contains(&guard.sentinel),
        "Sentinel should be removed"
    );

    Ok(())
}

#[rstest]
fn test_unguard_multiple_links(guarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, mut guard) = guarded_project;

    // Create additional linked files
    let source_file = test_dir.join("source.txt");
    let target_file = guard.target_dir.as_ref().unwrap().join("source.txt");
    create_file_with_content(target_file.to_str().unwrap(), "linked content")?;
    std::os::unix::fs::symlink(&target_file, &source_file)?;

    // Add a nested link
    let nested_dir = test_dir.join("nested");
    fs::create_dir(&nested_dir)?;
    let nested_source = nested_dir.join("nested.txt");
    let nested_target = guard.target_dir.as_ref().unwrap().join("nested.txt");
    create_file_with_content(nested_target.to_str().unwrap(), "nested content")?;
    std::os::unix::fs::symlink(&nested_target, &nested_source)?;

    // Act
    guard.unguard()?;

    // Assert
    // Check all symlinks are replaced with actual files
    assert!(
        !source_file.symlink_metadata()?.file_type().is_symlink(),
        "source.txt should not be a symlink"
    );
    assert!(
        !nested_source.symlink_metadata()?.file_type().is_symlink(),
        "nested.txt should not be a symlink"
    );

    // Verify content is preserved
    let content = fs::read_to_string(&source_file)?;
    assert_eq!(content, "linked content", "Content should be preserved");
    let nested_content = fs::read_to_string(&nested_source)?;
    assert_eq!(
        nested_content, "nested content",
        "Nested content should be preserved"
    );

    Ok(())
}

#[rstest]
fn test_unguard_non_symlink_envrc(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let mut guard = ConfGuardBuilder::default()
        .version(2)
        .source_dir(test_dir.clone())
        .is_relative(true)
        .confguard_base_dir(test_dir.join("guarded"))
        .build()?;

    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(envrc_path.to_str().unwrap(), "export FOO=bar\n")?;

    // Act
    let result = guard.unguard();

    // Assert
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("not a symlink"),
        "Should fail when .envrc is not a symlink"
    );

    Ok(())
}

#[rstest]
fn test_unguard_missing_envrc(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let mut guard = ConfGuardBuilder::default()
        .version(2)
        .source_dir(test_dir.clone())
        .is_relative(true)
        .confguard_base_dir(test_dir.join("guarded"))
        .build()?;

    // Act
    let result = guard.unguard();

    // Assert
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No .envrc file found"),
        "Should fail when .envrc is missing"
    );

    Ok(())
}

#[rstest]
fn test_unguard_permission_preservation(
    guarded_project: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    use std::os::unix::fs::PermissionsExt;

    // Arrange
    let (test_dir, mut guard) = guarded_project;
    let envrc_path = test_dir.join(".envrc");
    let target_envrc = guard.target_dir.as_ref().unwrap().join("dot.envrc");

    // Set specific permissions on target
    let mode = 0o644;
    fs::set_permissions(&target_envrc, fs::Permissions::from_mode(mode))?;

    // Act
    guard.unguard()?;

    // Assert
    let perms = envrc_path.metadata()?.permissions();
    assert_eq!(
        perms.mode() & 0o777,
        mode,
        "Permissions should be preserved"
    );

    Ok(())
}

#[rstest]
fn test_unguard_idempotency(guarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (_, mut guard) = guarded_project;

    // Act
    guard.unguard()?;
    let result = guard.unguard();

    // Assert
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("not a symlink"),
        "Second unguard should fail"
    );

    Ok(())
}

// In test_guard_unguard.rs, add this test:

#[rstest]
fn test_unguard_only_managed_links(guarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, mut guard) = guarded_project;

    // Create a managed link (points to sentinel directory)
    let managed_source = test_dir.join("managed.txt");
    let managed_target = guard.target_dir.as_ref().unwrap().join("managed.txt");
    create_file_with_content(managed_target.to_str().unwrap(), "managed content")?;
    std::os::unix::fs::symlink(&managed_target, &managed_source)?;

    // Create an unmanaged link (points elsewhere)
    let unmanaged_target = test_dir.join("unmanaged_target.txt");
    let unmanaged_source = test_dir.join("unmanaged.txt");
    create_file_with_content(unmanaged_target.to_str().unwrap(), "unmanaged content")?;
    std::os::unix::fs::symlink(&unmanaged_target, &unmanaged_source)?;

    // Act
    guard.unguard()?;

    // Assert
    // Managed link should be replaced with content
    assert!(
        !managed_source.symlink_metadata()?.file_type().is_symlink(),
        "Managed link should be replaced"
    );
    assert_eq!(
        fs::read_to_string(&managed_source)?,
        "managed content",
        "Managed content should be preserved"
    );

    // Unmanaged link should remain a symlink
    assert!(
        unmanaged_source
            .symlink_metadata()?
            .file_type()
            .is_symlink(),
        "Unmanaged link should remain a symlink"
    );
    assert_eq!(
        fs::read_link(&unmanaged_source)?,
        unmanaged_target,
        "Unmanaged link should point to original target"
    );

    Ok(())
}
