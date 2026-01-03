use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::errors::{ConfGuardContext, ConfGuardError, ConfGuardResult};
use fs_extra::dir::CopyOptions;
use pathdiff::diff_paths;
use regex::Regex;
use tracing::debug;
use walkdir::{DirEntry, WalkDir};

pub fn find_file_pattern(directory: &Path, pattern: &str) -> ConfGuardResult<Vec<PathBuf>> {
    debug!("directory={:?}, pattern={:?}", directory, pattern);
    let directory_path = PathBuf::from(directory);
    if !directory_path.is_dir() {
        return Err(ConfGuardError::NotDirectory);
    }

    let regex = Regex::new(pattern)?;
    let file_paths = WalkDir::new(directory_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_dir() && regex.is_match(e.file_name().to_string_lossy().as_ref())
        })
        .map(DirEntry::into_path)
        .collect();

    Ok(file_paths)
}

/// Converts a path to absolute, resolving it against the current working directory if relative
pub fn to_absolute_path(path: impl AsRef<Path>) -> ConfGuardResult<PathBuf> {
    let path = path.as_ref();
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .context("get current directory")?
            .join(path)
            .canonicalize()
            .with_context(|| format!("canonicalize path: {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use tempfile::tempdir;

    #[test]
    fn test_find_file_pattern() {
        let temp_dir = tempdir().expect("temp directory creation is a basic OS operation");
        let x = temp_dir.path();

        let confguard_dir1 = temp_dir.path().join("sops/rs-cg-1899fc23");
        let confguard_dir2 = temp_dir.path().join("sops/rs-cg-11111111");
        fs::create_dir_all(&confguard_dir1)
            .unwrap_or_else(|_| panic!("Failed to create directory {:?}", confguard_dir1));
        fs::create_dir_all(&confguard_dir2)
            .unwrap_or_else(|_| panic!("Failed to create directory {:?}", confguard_dir2));

        // let result = find_file_pattern(x, "rs-cg-........").unwrap();
        let result = find_file_pattern(x, format!("{}-[a-z0-9]{{8}}$", "rs-cg").as_str()).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_invalid_directory() {
        let result = find_file_pattern(Path::new("non_existent_dir"), ".*");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_pattern() {
        let result = find_file_pattern(Path::new("."), "["); // "[" is an invalid regex
        assert!(result.is_err());
    }
}
