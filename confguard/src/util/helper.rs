use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::{env, fs};

use crate::errors::{ConfGuardError, ConfGuardResult};
use crate::util::RESOURCES_DIR;
use tracing::debug;

pub mod path;
pub mod testing;

pub fn file_contents(path: &Path) -> ConfGuardResult<String> {
    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Copies a file from the included resources directory
/// the directory is compiled into the binary at compile time
pub fn copy_file_from_resources(
    file_name: &str,
    destination: &str,
    executable: bool,
) -> ConfGuardResult<()> {
    let file = RESOURCES_DIR
        .get_file(file_name)
        .ok_or_else(|| ConfGuardError::ResourceNotFound(file_name.to_string()))?;
    debug!("{:?} {:?}", RESOURCES_DIR, file);
    // Create the destination file
    let destination_file = Path::new(destination);
    // Ensure the parent directory of the destination file exists
    if let Some(parent) = destination_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    // Write the file data to the destination file
    let mut file_writer = fs::File::create(destination_file)?;
    file_writer.write_all(file.contents())?;

    // If the executable flag is set to true, set the permissions accordingly
    if executable {
        let mut permissions = file_writer.metadata()?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(destination_file, permissions)?;
    }

    Ok(())
}

pub fn assert_path_does_not_include(path: &Path, forbidden: &[&str]) {
    // Convert the path to a string representation
    let path_str = path
        .to_str()
        .expect("test paths are valid UTF-8 on supported platforms");

    for forbidden_item in forbidden {
        assert!(
            !path_str.contains(forbidden_item),
            "The path '{}' contains a forbidden string or subpath: '{}'",
            path_str,
            forbidden_item
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_to_string;
    use tempfile::tempdir;
    #[test]
    fn test_assert_path_does_not_include() {
        let valid_path = Path::new("/home/user/projects/rust_project");
        let invalid_path = Path::new("/home/user/projects/secret_project");

        let forbidden = ["secret", "private"];

        // This should pass
        assert_path_does_not_include(valid_path, &forbidden);

        // This should panic because "secret" is in the path
        let result = std::panic::catch_unwind(|| {
            assert_path_does_not_include(invalid_path, &forbidden);
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_copy_file_from_resources() {
        // Setup temporary directory.
        let dir = tempdir().unwrap();
        let dest_path = dir.path().join("test_file.txt");
        // Copy the file from resources.
        let result = copy_file_from_resources(
            "asset_to_be_included.txt",
            dest_path.to_str().unwrap(),
            true,
        );
        assert!(result.is_ok());
        // Confirm that the destination file now exists.
        assert!(dest_path.exists());
        // Confirm that the destination file has the correct contents.
        let contents = read_to_string(&dest_path).unwrap();
        assert_eq!(contents, "Hello, World!\n");

        // Confirm that the destination file has the executable flag set.
        let metadata = dest_path.metadata().unwrap();
        let permissions = metadata.permissions();
        assert_eq!(
            permissions.mode() & 0o100,
            0o100,
            "File should be executable"
        );
    }

    #[test]
    fn test_copy_file_from_resources2() {
        // Setup temporary directory.
        let dir = tempdir().unwrap();
        let dest_path = dir.path().join(".envrc");
        // Copy the file from resources.
        let result = copy_file_from_resources("dot.envrc", dest_path.to_str().unwrap(), false);
        assert!(result.is_ok());
        // Confirm that the destination file now exists.
        assert!(dest_path.exists());
        // Confirm that the destination file has the correct contents.
        let _ = read_to_string(&dest_path).unwrap();
        // println!("{}", contents);
    }
}
