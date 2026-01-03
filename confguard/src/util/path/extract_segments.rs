use crate::errors::{ConfGuardError, ConfGuardResult};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::warn;

/// Function to extract segments from a path
#[allow(dead_code)]
pub fn extract_segments(
    path: &Path,
    num_segments: usize,
    separator: &str,
    prefix: &str,
    suffix: &str,
) -> String {
    let components: Vec<_> = path
        .components()
        .map(|comp| comp.as_os_str().to_string_lossy())
        .filter(|s| !s.is_empty()) // Filter out empty components
        .collect();
    let end = components.len();

    // If requesting more segments than available, take all components with a single leading '/'
    if num_segments > end {
        return format!("{}/{}{}", prefix, components.join(separator), suffix);
    }

    // Otherwise take the requested number of segments from the end
    let start = end.saturating_sub(num_segments);
    let extracted_segments = components[start..end].join(separator);
    format!("{}{}{}", prefix, extracted_segments, suffix)
}

/// Function to create a file with segments extracted from a path
#[allow(dead_code)]
pub fn create_file_with_segments(
    path: &Path,
    num_segments: usize,
    separator: &str,
    prefix: &str,
    suffix: &str,
) -> ConfGuardResult<String> {
    let components: Vec<_> = path
        .components()
        .map(|comp| comp.as_os_str().to_string_lossy())
        .collect();
    let end = components.len();
    let start = end.saturating_sub(num_segments);
    let extracted_segments = components[start..end].join(separator);
    let file_name = format!("{}{}{}", prefix, extracted_segments, suffix);

    let mut file = File::create(&file_name)?;

    if num_segments >= 3 {
        writeln!(file, "# rsenv: {}.env", components[end - 2])?;
        writeln!(file, "export AWS_REGION={}", components[end - 3])?;
        writeln!(file, "export ENVIRONMENT={}", components[end - 2])?;
        writeln!(file, "export FUNCTIONAL_RESOURCE={}", components[end - 1])?;
    } else {
        warn!("Cannot create rsenv file: path must have at least 3 segments");
    }

    Ok(file_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::replace_base::replace_prefix;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

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

    #[test]
    fn test_extract_segments() {
        let path1 = Path::new("/a/b/c/d/e");
        let path2 = Path::new("/1/2/3");

        assert_eq!(extract_segments(path1, 2, "-", "{", "}"), "{d-e}");

        assert_eq!(extract_segments(path1, 5, "/", "[", "]"), "[a/b/c/d/e]");

        // When requesting more segments than available, it includes the root "/"
        // which appears as double slash due to how Path components work
        assert_eq!(extract_segments(path1, 6, "/", "[", "]"), "[//a/b/c/d/e]");

        assert_eq!(extract_segments(path1, 0, "/", "[", "]"), "[]");

        assert_eq!(extract_segments(path2, 2, "::", "<", ">"), "<2::3>");
    }

    #[test]
    fn test_path_components_behavior() {
        let path = Path::new("/a/b/c");
        let components: Vec<_> = path
            .components()
            .map(|comp| comp.as_os_str().to_string_lossy())
            .collect();

        // First component is "/" due to the root
        assert_eq!(components[0].to_string(), "/");
    }

    #[test]
    fn test_create_file_with_segments() -> ConfGuardResult<()> {
        let temp_dir = tempdir()?;
        let test_path = Path::new("/region/env/resource");
        let current_dir = temp_dir.path();
        std::env::set_current_dir(current_dir)?;

        let file_name = create_file_with_segments(test_path, 3, ".", "", ".env")?;

        // Verify file exists
        let file_path = current_dir.join(&file_name);
        assert!(file_path.exists());

        // Verify content
        let content = fs::read_to_string(&file_path)?;
        let expected = "# rsenv: env.env\n\
             export AWS_REGION=region\n\
             export ENVIRONMENT=env\n\
             export FUNCTIONAL_RESOURCE=resource\n"
            .to_string();
        assert_eq!(content, expected);

        Ok(())
    }

    #[test]
    fn test_create_file_with_insufficient_segments() -> ConfGuardResult<()> {
        let temp_dir = tempdir()?;
        let test_path = Path::new("/region/env");
        let current_dir = temp_dir.path();
        std::env::set_current_dir(current_dir)?;

        let file_name = create_file_with_segments(test_path, 2, ".", "", ".env")?;

        // Verify file exists but is empty
        let file_path = current_dir.join(&file_name);
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path)?;
        assert!(content.is_empty());

        Ok(())
    }
}
