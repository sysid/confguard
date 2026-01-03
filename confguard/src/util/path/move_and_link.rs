// src/util/path/move_and_link.rs
use crate::errors::{ConfGuardContext, ConfGuardError, ConfGuardResult};
use crate::util::link::create_link;
use derive_builder::Builder;
use derive_more::Display;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Builder, Display, Default, Clone)]
#[display("MoveAndLink {{ source: {}, destination: {} }}", source_path.display(), destination_path.display())]
pub struct MoveAndLink {
    source_path: PathBuf,
    destination_path: PathBuf,
}

impl MoveAndLink {
    /// Creates a new MoveAndLink instance with canonicalized absolute paths
    ///
    /// # Requirements
    /// - Source path must exist
    /// - Source path must not be a symlink
    pub fn new(
        source_path: impl AsRef<Path>,
        destination_path: impl AsRef<Path>,
    ) -> ConfGuardResult<Self> {
        let current_dir = std::env::current_dir()?;
        let source_ref = source_path.as_ref();

        // Get absolute source path
        let source_abs = if source_ref.is_absolute() {
            source_ref.to_path_buf()
        } else {
            current_dir.join(source_ref)
        };

        // First check existence
        if !source_abs.exists() {
            return Err(ConfGuardError::SourcePathNotFound(source_abs));
        }

        // Check if it's a symlink before canonicalization
        if source_abs.symlink_metadata()?.file_type().is_symlink() {
            return Err(ConfGuardError::SourceIsSymlink(source_abs));
        }

        // Then canonicalize
        let source_canonical = source_abs
            .canonicalize()
            .with_context(|| format!("canonicalize source path: {:?}", source_abs))?;

        let dest_ref = destination_path.as_ref();
        let dest_abs = if dest_ref.is_absolute() {
            dest_ref.to_path_buf()
        } else {
            current_dir.join(dest_ref)
        };

        Ok(Self {
            source_path: source_canonical,
            destination_path: dest_abs,
        })
    }

    /// Moves the source file to the destination and creates a symbolic link at the source location
    pub fn move_and_link(&self, relative: bool) -> ConfGuardResult<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = self.destination_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Move the file
        fs::rename(&self.source_path, &self.destination_path).with_context(|| {
            format!("move {:?} to {:?}", self.source_path, self.destination_path)
        })?;

        // Create symbolic link
        create_link(&self.destination_path, &self.source_path, relative)?;

        Ok(())
    }

    /// Reverses the move_and_link operation by replacing the symlink with the original file
    pub fn revert(&self) -> ConfGuardResult<()> {
        // Ensure source exists and is a symlink
        if !self.source_path.exists() {
            return Err(ConfGuardError::SourceLinkNotFound(self.source_path.clone()));
        }
        if !self
            .source_path
            .symlink_metadata()?
            .file_type()
            .is_symlink()
        {
            return Err(ConfGuardError::SourceNotSymlink(self.source_path.clone()));
        }

        // Remove the symlink
        fs::remove_file(&self.source_path)?;

        // Move the file back
        fs::rename(&self.destination_path, &self.source_path).with_context(|| {
            format!(
                "move {:?} back to {:?}",
                self.destination_path, self.source_path
            )
        })?;

        Ok(())
    }
}
