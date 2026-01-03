use derive_builder::UninitializedFieldError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfGuardError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),

    #[error("Project already guarded: {0}")]
    AlreadyGuarded(PathBuf),

    #[error("Project not guarded: {0}")]
    NotGuarded(PathBuf),

    #[error("No .envrc file found in: {0}")]
    NoEnvrcFile(PathBuf),

    #[error("File already exists: {0}")]
    FileAlreadyExists(PathBuf),

    #[error("Target already exists: {0}")]
    TargetAlreadyExists(PathBuf),

    #[error("Configuration file not found: {0}")]
    ConfigFileNotFound(PathBuf),

    #[error("Configuration file already exists: {0}")]
    ConfigFileAlreadyExists(PathBuf),

    #[error("Source does not exist: {0}")]
    SourceNotFound(PathBuf),

    #[error("Source must be within project directory: {0}")]
    SourceNotInProject(PathBuf),

    #[error("Target directory not set")]
    TargetDirectoryNotSet,

    #[error("Link does not exist: {0}")]
    LinkNotFound(PathBuf),

    #[error("Not a symbolic link: {0}")]
    NotSymbolicLink(PathBuf),

    #[error("Path {0} is not a symbolic link")]
    PathNotSymbolicLink(PathBuf),

    #[error("Resource file not found: {0}")]
    ResourceNotFound(String),

    #[error("The provided path is not a directory")]
    NotDirectory,

    #[error("Path must be absolute: {0}")]
    PathNotAbsolute(PathBuf),

    #[error("Source path does not exist: {0}")]
    SourcePathNotFound(PathBuf),

    #[error("Source is already a symbolic link: {0}")]
    SourceIsSymlink(PathBuf),

    #[error("Source link does not exist: {0}")]
    SourceLinkNotFound(PathBuf),

    #[error("Source is not a symbolic link: {0}")]
    SourceNotSymlink(PathBuf),

    #[error("Path does not start with: {0:?}")]
    PathPrefixMismatch(PathBuf),

    #[error("Configuration file not found at: {path}. Run 'confguard sops-init' to create it.")]
    ConfigNotFoundWithHint { path: String },

    #[error("Project appears to be already guarded - .envrc is a symlink")]
    ProjectAlreadyGuardedSymlink,

    #[error("Not a guarded envrc file: {0}")]
    NotGuardedEnvrc(PathBuf),

    #[error("Failed to get parent directory of: {0}")]
    NoParentDirectory(PathBuf),

    #[error("Link already exists: {0} -> {1}")]
    LinkAlreadyExists(String, String),

    #[error("Link exists but points elsewhere: {0} -> {1}")]
    LinkPointsElsewhere(String, String),

    #[error("Source directory does not exist: {0}")]
    SourceDirectoryNotFound(PathBuf),

    #[error("Project is not guarded - .envrc is not a symlink")]
    ProjectNotGuardedNotSymlink,

    #[error("Sentinel already set")]
    SentinelAlreadySet,

    #[error("Invalid source directory path")]
    InvalidSourceDirectory,

    #[error("Project appears to be already guarded - sentinel exists: {0:?}")]
    SentinelExists(Vec<PathBuf>),

    #[error("Cannot determine home directory")]
    CannotDetermineHome,

    #[error("Cannot calculate relative path")]
    CannotCalculateRelativePath,

    #[error("Original path does not exist: {0}")]
    OriginalPathNotFound(String),

    #[error("Failed to create relative path")]
    FailedToCreateRelativePath,

    #[error("Path contains invalid UTF-8: {0}")]
    InvalidUtf8Path(PathBuf),

    #[error("File operation failed: {0}")]
    FileOperation(#[from] std::io::Error),

    #[error("Path canonicalization failed: {path}, reason: {reason}")]
    PathCanonicalization { path: PathBuf, reason: String },

    #[error("Directory change failed: {reason}")]
    DirectoryChange { reason: String },

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("Builder error: {0}")]
    BuilderError(String),

    #[error("File system extra error: {0}")]
    FsExtra(#[from] fs_extra::error::Error),

    #[error("Strip prefix error: {0}")]
    StripPrefixError(#[from] std::path::StripPrefixError),

    #[error("TOML deserialization error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Thread pool build error: {0}")]
    ThreadPoolBuildError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type ConfGuardResult<T> = Result<T, ConfGuardError>;

/// Extension trait to provide context functionality similar to anyhow
/// This allows us to add contextual information to errors while maintaining
/// the specific error types that thiserror provides
pub trait ConfGuardContext<T> {
    fn with_context<F>(self, f: F) -> ConfGuardResult<T>
    where
        F: FnOnce() -> String;

    fn context(self, msg: &str) -> ConfGuardResult<T>;
}

// Specific implementations for common error types to avoid conflicts
impl<T> ConfGuardContext<T> for Result<T, std::io::Error> {
    fn with_context<F>(self, f: F) -> ConfGuardResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", f(), e)))
    }

    fn context(self, msg: &str) -> ConfGuardResult<T> {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", msg, e)))
    }
}

impl<T> ConfGuardContext<T> for Result<T, regex::Error> {
    fn with_context<F>(self, f: F) -> ConfGuardResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", f(), e)))
    }

    fn context(self, msg: &str) -> ConfGuardResult<T> {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", msg, e)))
    }
}

impl<T> ConfGuardContext<T> for Result<T, fs_extra::error::Error> {
    fn with_context<F>(self, f: F) -> ConfGuardResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", f(), e)))
    }

    fn context(self, msg: &str) -> ConfGuardResult<T> {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", msg, e)))
    }
}

// For Result<(), ConfGuardError> and similar ConfGuardError results
impl<T> ConfGuardContext<T> for Result<T, ConfGuardError> {
    fn with_context<F>(self, f: F) -> ConfGuardResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", f(), e)))
    }

    fn context(self, msg: &str) -> ConfGuardResult<T> {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", msg, e)))
    }
}

// For TOML deserialization errors
impl<T> ConfGuardContext<T> for Result<T, toml::de::Error> {
    fn with_context<F>(self, f: F) -> ConfGuardResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", f(), e)))
    }

    fn context(self, msg: &str) -> ConfGuardResult<T> {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", msg, e)))
    }
}

// For builder errors
impl From<derive_builder::UninitializedFieldError> for ConfGuardError {
    fn from(err: derive_builder::UninitializedFieldError) -> Self {
        ConfGuardError::Internal(format!("Builder error: {}", err))
    }
}

// For ConfGuardBuilderError from derive_builder
impl From<crate::core::ConfGuardBuilderError> for ConfGuardError {
    fn from(err: crate::core::ConfGuardBuilderError) -> Self {
        ConfGuardError::Internal(format!("ConfGuard builder error: {}", err))
    }
}

// For thread pool build errors
impl From<rayon::ThreadPoolBuildError> for ConfGuardError {
    fn from(err: rayon::ThreadPoolBuildError) -> Self {
        ConfGuardError::ThreadPoolBuildError(err.to_string())
    }
}

// For StripPrefixError context
impl<T> ConfGuardContext<T> for Result<T, std::path::StripPrefixError> {
    fn with_context<F>(self, f: F) -> ConfGuardResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", f(), e)))
    }

    fn context(self, msg: &str) -> ConfGuardResult<T> {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", msg, e)))
    }
}

// For rayon thread pool build errors
impl<T> ConfGuardContext<T> for Result<T, rayon::ThreadPoolBuildError> {
    fn with_context<F>(self, f: F) -> ConfGuardResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", f(), e)))
    }

    fn context(self, msg: &str) -> ConfGuardResult<T> {
        self.map_err(|e| ConfGuardError::Internal(format!("{}: {}", msg, e)))
    }
}
