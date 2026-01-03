use crate::errors::{ConfGuardError, ConfGuardResult};
use std::path::{Path, PathBuf};

/// Function to replace the prefix of a path
pub fn replace_prefix(path: &Path, base: &Path, new_base: &Path) -> ConfGuardResult<PathBuf> {
    path.strip_prefix(base)
        .map(|stripped| new_base.join(stripped))
        .map_err(|_| ConfGuardError::PathPrefixMismatch(base.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_replace_prefix() {
        let base = PathBuf::from("/old_base");
        let new_base = PathBuf::from("/new_base");
        let path = PathBuf::from("/old_base/mydir/myfile.txt");

        let new_path = replace_prefix(&path, &base, &new_base).unwrap();

        assert_eq!(new_path, PathBuf::from("/new_base/mydir/myfile.txt"));
    }

    #[test]
    fn test_replace_prefix_directory() {
        let base = PathBuf::from("/old_base");
        let new_base = PathBuf::from("/new_base");
        let path = PathBuf::from("/old_base/mydir");

        let new_path = replace_prefix(&path, &base, &new_base).unwrap();

        assert_eq!(new_path, PathBuf::from("/new_base/mydir"));
    }

    #[test]
    fn test_replace_prefix_with_no_match() {
        let base = PathBuf::from("/not_a_prefix");
        let new_base = PathBuf::from("/new_base");
        let path = PathBuf::from("/old_base/mydir/myfile.txt");

        let result = replace_prefix(&path, &base, &new_base);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Path does not start with: {:?}", base)
        );
    }
}
