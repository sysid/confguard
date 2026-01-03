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
fn unguarded_project(test_dir: PathBuf) -> (PathBuf, ConfGuard) {
    // Create a simple .envrc
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(envrc_path.to_str().unwrap(), "export FOO=bar\n").unwrap();

    // Create guard with target_dir already set
    let guard = ConfGuard::new_for_guarding(test_dir.clone(), true).unwrap();

    (test_dir, guard)
}

#[rstest]
fn test_guard_success(unguarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, mut guard) = unguarded_project;
    let envrc_path = test_dir.join(".envrc");
    let initial_content = fs::read_to_string(&envrc_path)?;

    // Act
    guard.guard(false)?;

    // Assert
    // 1. Check .envrc is now a symlink
    assert!(
        envrc_path.symlink_metadata()?.file_type().is_symlink(),
        ".envrc should be a symlink"
    );

    // 2. Check target directory was created
    let target_dir = guard.target_dir.as_ref().unwrap();
    assert!(target_dir.exists(), "Target directory should exist");
    assert!(
        target_dir.starts_with(guard.confguard_base_dir.join("guarded")),
        "Target dir should be in guarded subdirectory"
    );

    // 3. Check dot.envrc exists in target dir
    let target_envrc = target_dir.join("dot.envrc");
    assert!(target_envrc.exists(), "target dot.envrc should exist");

    // 4. Check content was preserved and confguard section added
    let new_content = fs::read_to_string(&target_envrc)?;
    assert!(
        new_content.contains(&initial_content),
        "Original content should be preserved"
    );
    assert!(
        new_content.contains("confguard start"),
        "Should have confguard section"
    );
    assert!(
        new_content.contains(&guard.sentinel),
        "Should contain sentinel"
    );

    // 5. Check environments directory was created
    let env_dir = target_dir.join("environments");
    assert!(env_dir.exists(), "environments directory should exist");
    assert!(env_dir.join("local.env").exists(), "local.env should exist");

    Ok(())
}

#[rstest]
fn test_guard_absolute_links(unguarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, mut guard) = unguarded_project;
    let envrc_path = test_dir.join(".envrc");

    // Act
    guard.guard(true)?;

    // Assert
    let link_target = fs::read_link(&envrc_path)?;
    assert!(link_target.is_absolute(), "Link target should be absolute");

    // Check confguard section
    let target_envrc = guard.target_dir.unwrap().join("dot.envrc");
    let content = fs::read_to_string(&target_envrc)?;
    assert!(
        content.contains("config.relative = false"),
        "Should indicate absolute paths"
    );

    Ok(())
}

#[rstest]
fn test_guard_relative_links(unguarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, mut guard) = unguarded_project;
    let envrc_path = test_dir.join(".envrc");

    // Act
    guard.guard(false)?;

    // Assert
    let link_target = fs::read_link(&envrc_path)?;
    assert!(!link_target.is_absolute(), "Link target should be relative");

    // Check confguard section
    let target_envrc = guard.target_dir.unwrap().join("dot.envrc");
    let content = fs::read_to_string(&target_envrc)?;
    assert!(
        content.contains("config.relative = true"),
        "Should indicate relative paths"
    );

    Ok(())
}

#[rstest]
fn test_guard_missing_source_dir(test_dir: PathBuf) -> ConfGuardResult<()> {
    // Arrange
    let mut guard = ConfGuardBuilder::default()
        .version(2)
        .source_dir(test_dir.join("nonexistent"))
        .is_relative(true)
        .confguard_base_dir(test_dir.join("guarded"))
        .build()?;

    // Act
    let result = guard.guard(false);

    // Assert
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));

    Ok(())
}

#[rstest]
fn test_guard_preserve_envrc_permissions(
    unguarded_project: (PathBuf, ConfGuard),
) -> ConfGuardResult<()> {
    use std::os::unix::fs::PermissionsExt;

    // Arrange
    let (test_dir, mut guard) = unguarded_project;
    let envrc_path = test_dir.join(".envrc");

    // Set specific permissions
    let mode = 0o644;
    fs::set_permissions(&envrc_path, fs::Permissions::from_mode(mode))?;

    // Act
    guard.guard(false)?;

    // Assert
    let target_envrc = guard.target_dir.unwrap().join("dot.envrc");
    let perms = target_envrc.metadata()?.permissions();
    assert_eq!(
        perms.mode() & 0o777,
        mode,
        "Permissions should be preserved"
    );

    Ok(())
}

#[rstest]
fn test_guard_idempotency(unguarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (_, mut guard) = unguarded_project;

    // Act
    guard.guard(false)?;
    let result = guard.guard(false);

    // Assert
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already guarded"));

    Ok(())
}
