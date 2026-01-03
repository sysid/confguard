// src/sops/gitignore.rs
use crate::errors::{ConfGuardError, ConfGuardResult};
use chrono::Utc;
use serde::Deserialize;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};

#[derive(Debug, Clone, Deserialize)]
pub struct GitignoreManager {
    pub gitignore_path: PathBuf,
    pub section_start: String,
    pub section_end: String,
}

impl GitignoreManager {
    pub fn new(project_path: &Path) -> Self {
        Self {
            gitignore_path: project_path.join(".gitignore"),
            section_start: "# ---------------------------------- confguard-start -----------------------------------".to_string(),
            section_end: "# ---------------------------------- confguard-end -----------------------------------".to_string(),
        }
    }

    #[instrument(skip(self))]
    pub fn update_entries(
        &self,
        extensions: &[String],
        filenames: &[String],
    ) -> ConfGuardResult<()> {
        debug!("Updating gitignore entries at {:?}", self.gitignore_path);

        // Read existing content
        let content = if self.gitignore_path.exists() {
            fs::read_to_string(&self.gitignore_path)?
        } else {
            String::new()
        };

        let (pre_section, _, post_section) = self.split_sections(&content);

        // Prepare new entries
        let mut entries = Vec::new();
        for ext in extensions {
            entries.push(format!("*.{}", ext));
        }
        entries.extend(filenames.iter().cloned());
        entries.sort();

        // Create new section content
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
        let mut new_section = vec![self.section_start.clone()];
        new_section.extend(
            entries
                .iter()
                .map(|e| format!("{}  # sops-managed {}", e, timestamp)),
        );
        new_section.push(self.section_end.clone());

        // Write updated content
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.gitignore_path)?;

        if !pre_section.is_empty() {
            writeln!(file, "{}", pre_section.join("\n"))?;
        }
        writeln!(file, "{}", new_section.join("\n"))?;
        if !post_section.is_empty() {
            writeln!(file, "{}", post_section.join("\n"))?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn split_sections(&self, content: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut pre_section = Vec::new();
        let mut section = Vec::new();
        let mut post_section = Vec::new();
        let mut in_section = false;

        for line in content.lines() {
            if line == self.section_start {
                in_section = true;
            } else if line == self.section_end {
                in_section = false;
            } else if in_section {
                section.push(line.to_string());
            } else if !in_section && section.is_empty() {
                pre_section.push(line.to_string());
            } else {
                post_section.push(line.to_string());
            }
        }

        (pre_section, section, post_section)
    }

    #[instrument(skip(self))]
    pub fn clean_entries(&self) -> ConfGuardResult<()> {
        debug!("Cleaning gitignore entries at {:?}", self.gitignore_path);

        if !self.gitignore_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.gitignore_path)?;
        let (pre_section, _, post_section) = self.split_sections(&content);

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.gitignore_path)?;

        if !pre_section.is_empty() {
            writeln!(file, "{}", pre_section.join("\n"))?;
        }
        if !post_section.is_empty() {
            writeln!(file, "{}", post_section.join("\n"))?;
        }

        Ok(())
    }
}
