use crate::errors::{ConfGuardContext, ConfGuardError, ConfGuardResult};
use pathdiff::diff_paths;
use std::path::{Path, PathBuf};
use std::{env, fs};

use fs_extra::dir;
use fs_extra::dir::CopyOptions;
#[cfg(unix)]
use std::os::unix::fs as platform_fs;
#[cfg(windows)]
use std::os::windows::fs as platform_fs;
use tracing::{debug, info, instrument};

/// Creates a symbolic link at `link_path` pointing to `original_path`.
///
/// # Arguments
/// * `original_path` - The target path that the link will point to
/// * `link_path` - The path where the symbolic link will be created
/// * `relative` - If true, creates a relative symlink; if false, creates an absolute symlink
///
/// # Returns
/// * `Result<()>` - Ok(()) on success, or an error if the operation fails
/// ```
#[instrument]
pub fn create_link(original_path: &Path, link_path: &Path, relative: bool) -> ConfGuardResult<()> {
    // Ensure the original path exists
    if !original_path.exists() {
        return Err(ConfGuardError::OriginalPathNotFound(
            original_path.display().to_string(),
        ));
    }

    // Create parent directories for the link if they don't exist
    if let Some(parent) = link_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Calculate the target path based on whether we want relative or absolute links
    let target_path = if relative {
        // Get relative path from link's parent directory to the original
        if let Some(link_parent) = link_path.parent() {
            diff_paths(original_path, link_parent)
                .ok_or(ConfGuardError::FailedToCreateRelativePath)?
        } else {
            original_path.to_path_buf()
        }
    } else {
        original_path.to_path_buf()
    };

    #[cfg(unix)]
    {
        platform_fs::symlink(target_path, link_path)?;
    }

    #[cfg(windows)]
    {
        if original_path.is_dir() {
            platform_fs::symlink_dir(target_path, link_path)?;
        } else {
            platform_fs::symlink_file(target_path, link_path)?;
        }
    }

    Ok(())
}

/// Replace a symbolic link with its target.
///
/// If `link_path` is not a symbolic link, this function returns an error.
#[instrument]
pub fn replace_link_with_target(link_path: &Path) -> ConfGuardResult<()> {
    // Validate UTF-8 in path
    validate_utf8_path(link_path)?;
    if !link_path.exists() {
        return Err(ConfGuardError::LinkNotFound(link_path.to_path_buf()));
    }

    // Ensure the path is a symbolic link.
    debug!("metadata: {:?}", std::fs::symlink_metadata(link_path)?);
    if !link_path.symlink_metadata()?.file_type().is_symlink() {
        return Err(ConfGuardError::PathNotSymbolicLink(link_path.to_path_buf()));
    }

    let link_target = std::fs::read_link(link_path).context("read link")?;
    let link_path_abs = to_absolute_path(validate_utf8_path(link_path)?)?;
    debug!(
        "link_path_abs: {:?}, link_target: {:?}, CWD: {:?}",
        link_path_abs,
        link_target,
        std::env::current_dir()?
    );

    // Save the original current directory, to restore it later
    let original_dir = env::current_dir()?;
    // Change the current directory to handle relative links correctly
    env::set_current_dir(link_path_abs.parent().unwrap())?;

    // For relative links the CWD must be where the link is
    let link_target_metadata = fs::metadata(&link_target).context("read link metadata")?;
    debug!("link_target_metadata: {:?}", link_target_metadata);

    fs::remove_file(&link_path_abs).with_context(|| {
        format!(
            "remove file: {:?}, CWD: {:?}",
            link_path_abs,
            env::current_dir().unwrap_or_else(|_| PathBuf::from("unknown"))
        )
    })?;

    if link_target_metadata.is_file() {
        fs::rename(&link_target, &link_path_abs)?;
    } else if link_target_metadata.is_dir() {
        let options = CopyOptions {
            copy_inside: true,
            ..Default::default()
        };
        dir::move_dir(&link_target, &link_path_abs, &options)?;
    }

    info!(
        "Replaced link {:?} with target {:?}",
        link_path, link_target
    );
    env::set_current_dir(original_dir)?;
    Ok(())
}

fn validate_utf8_path(path: &Path) -> ConfGuardResult<&str> {
    path.to_str()
        .ok_or_else(|| ConfGuardError::InvalidUtf8Path(path.to_path_buf()))
}

fn to_absolute_path(relative_path: &str) -> ConfGuardResult<PathBuf> {
    let current_directory = env::current_dir()?;
    let absolute_path = current_directory.join(relative_path);

    // Validate UTF-8
    validate_utf8_path(&absolute_path)?;

    Ok(absolute_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::helper::testing;
    use crate::util::testing::{create_file_with_content, print_active_env_vars, TEST_ENV_VARS};
    use std::fs::File;
    use std::io::{Read, Write};

    #[test]
    fn test_create_relative_link_file() -> ConfGuardResult<()> {
        let base_dir = testing::setup_test_dir();
        print_active_env_vars(TEST_ENV_VARS);

        // Create test directory structure
        let original_file = base_dir.join("original/nested/file.txt");
        create_file_with_content(original_file.to_str().unwrap(), "Hello world")?;

        let link_path = base_dir.join("links/link");

        // Create the link
        create_link(&original_file, &link_path, true)?;

        // Verify the link was created and points to the correct location
        assert!(link_path.exists());
        assert!(link_path.symlink_metadata()?.file_type().is_symlink());

        // read content of link and verify
        let mut file = File::open(&link_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        assert_eq!(content, "Hello world");

        // testing::teardown_test_dir(&base_dir);
        Ok(())
    }

    #[test]
    fn test_create_relative_link() -> ConfGuardResult<()> {
        let base_dir = testing::setup_test_dir();
        print_active_env_vars(TEST_ENV_VARS);

        // Create test directory structure
        let original_dir = base_dir.join("original");
        fs::create_dir_all(&original_dir)?;

        let link_parent = base_dir.join("links");
        let link_path = link_parent.join("link");

        // Create the link
        create_link(&original_dir, &link_path, true)?;

        // Verify the link was created and points to the correct location
        assert!(link_path.exists());
        assert!(link_path.symlink_metadata()?.file_type().is_symlink());

        // testing::teardown_test_dir(&base_dir);
        Ok(())
    }
}
