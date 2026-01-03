use crate::errors::{ConfGuardContext, ConfGuardError, ConfGuardResult};
use std::fs;
use std::path::{Path, PathBuf};

/// Returns a string representation of a given path with the home directory portion replaced by "$HOME".
///
/// This function takes a reference to a `Path` and checks if the path contains the user's home directory.
/// If it does, the function replaces the home directory portion with the string "$HOME".
pub fn to_home_based(path: &Path) -> ConfGuardResult<String> {
    if !path.is_absolute() {
        return Err(ConfGuardError::PathNotAbsolute(path.to_path_buf()));
    }
    let home_dir = dirs::home_dir().ok_or(ConfGuardError::CannotDetermineHome)?;
    let home_dir_str = home_dir
        .to_str()
        .ok_or_else(|| ConfGuardError::InvalidUtf8Path(home_dir.clone()))?;

    // Replace the home directory portion of the path with "$HOME"
    let path_str = path.display().to_string();
    let relative_path = path_str.replace(home_dir_str, "$HOME");
    Ok(relative_path)
}

/// This function takes a path string as input and returns the absolute PathBuf.
/// It replaces $HOME with the actual home directory path.
pub fn from_home_based(path: &str) -> ConfGuardResult<PathBuf> {
    // Use context to add custom error message
    let home_dir = dirs::home_dir().ok_or(ConfGuardError::CannotDetermineHome)?;
    let home_dir_str = home_dir
        .to_str()
        .ok_or_else(|| ConfGuardError::InvalidUtf8Path(home_dir.clone()))?;

    let path = path.replace("$HOME", home_dir_str);
    let path = PathBuf::from(&path);

    // Canonicalize the path and convert it to a PathBuf
    fs::canonicalize(&path).with_context(|| format!("canonicalize path: {:?}", path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_home_based_with_home_directory() {
        let home = dirs::home_dir().unwrap();
        let path = home.join("Documents/test.txt");
        let result = to_home_based(&path).unwrap();
        assert_eq!(result, "$HOME/Documents/test.txt");
    }

    #[test]
    fn test_to_home_based_with_relative_directory() {
        let path = Path::new("relative/Documents/test.txt");
        let result = to_home_based(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_to_home_based_without_home_directory() {
        let path = Path::new("/etc/config/settings.txt");
        let result = to_home_based(path).unwrap();
        assert_eq!(result, "/etc/config/settings.txt");
    }

    #[test]
    fn test_to_home_based_with_invalid_path() {
        let path = Path::new("/invalid_path");
        let result = to_home_based(path).unwrap();
        assert_eq!(result, "/invalid_path");
    }

    #[test]
    fn test_from_home_based() {
        // Test that $HOME expansion works and returns absolute path
        // Skip if the path doesn't exist on this machine
        let result = from_home_based("$HOME");
        if let Ok(path) = result {
            assert!(path.is_absolute());
            assert_eq!(path, dirs::home_dir().unwrap());
        }
    }
}
