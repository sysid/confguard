use crate::errors::{ConfGuardContext, ConfGuardError, ConfGuardResult};
use crate::sops::crypto::SopsCrypto;
use crate::sops::gitignore::GitignoreManager;
use crate::util::copy_file_from_resources;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::ThreadPoolBuilder;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, io};
use tracing::{debug, instrument};
use walkdir::{DirEntry, WalkDir};

const NUM_THREADS: usize = 8;

#[derive(Debug, Clone, Deserialize)]
pub struct SopsManager {
    pub config: SopsConfig,
    pub base_path: PathBuf,
    crypto: SopsCrypto,
    gitignore: GitignoreManager,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SopsConfig {
    #[serde(default)]
    pub file_extensions_enc: Vec<String>,
    #[serde(default)]
    pub file_names_enc: Vec<String>,
    #[serde(default)]
    pub file_extensions_dec: Vec<String>,
    #[serde(default)]
    pub file_names_dec: Vec<String>,
    #[serde(default = "default_gpg_key")]
    pub gpg_key: String,
}

fn default_gpg_key() -> String {
    "60A4127E82E218297532FAB6D750B66AE08F3B90".to_string()
}

impl SopsManager {
    pub fn new(toml_path: &Path) -> ConfGuardResult<Self> {
        if !toml_path.exists() {
            return Err(ConfGuardError::ConfigFileNotFound(toml_path.to_path_buf()));
        }

        // Derive base_path from config file location (its parent directory)
        let base_path = toml_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let sops_config = read_config(toml_path)?;
        let crypto = SopsCrypto::new(sops_config.gpg_key.clone());
        let gitignore = GitignoreManager::new(&base_path);

        Ok(Self {
            config: sops_config,
            base_path,
            crypto,
            gitignore,
        })
    }

    #[instrument]
    pub fn encrypt_files(&self, base_dir: Option<&Path>) -> ConfGuardResult<()> {
        // TODO: Check Update gitignore only when operating on default base path
        if base_dir.is_none() {
            self.gitignore.update_entries(
                &self.config.file_extensions_enc,
                &self.config.file_names_enc,
            )?;
        }

        let files = self.collect_files(
            base_dir.unwrap_or(&self.base_path),
            &self.config.file_extensions_enc,
            &self.config.file_names_enc,
        )?;

        let pool = ThreadPoolBuilder::new()
            .num_threads(NUM_THREADS)
            .build()
            .with_context(|| format!("create thread pool with {} threads", NUM_THREADS))?;

        pool.install(|| {
            files
                .into_par_iter()
                .try_for_each(|entry| -> ConfGuardResult<()> {
                    let path = entry.path();
                    let enc_path = PathBuf::from(format!("{}.enc", path.display()));
                    self.crypto.encrypt_file(path, &enc_path)?;
                    println!("Encrypted: {:?} -> {:?}", path, enc_path);
                    Ok(())
                })
        })?;

        Ok(())
    }

    #[instrument]
    pub fn decrypt_files(&self, base_dir: Option<&Path>) -> ConfGuardResult<()> {
        let files = self.collect_files(
            base_dir.unwrap_or(&self.base_path),
            &self.config.file_extensions_dec,
            &self.config.file_names_dec,
        )?;

        let pool = ThreadPoolBuilder::new()
            .num_threads(NUM_THREADS)
            .build()
            .with_context(|| format!("create thread pool with {} threads", NUM_THREADS))?;

        pool.install(|| {
            files
                .into_par_iter()
                .try_for_each(|entry| -> ConfGuardResult<()> {
                    let path = entry.path();
                    //     PathBuf::from(path.display().to_string().strip_suffix(".enc").ok_or_else(
                    //         || anyhow!("File doesn't end with .enc extension: {:?}", path),
                    //     )?);
                    let dec_path = path.with_extension("");

                    self.crypto.decrypt_file(path, &dec_path)?;
                    println!("Decrypted: {:?} -> {:?}", path, dec_path);
                    Ok(())
                })
        })?;

        Ok(())
    }

    #[instrument]
    pub fn clean_files(&self, base_dir: Option<&Path>) -> ConfGuardResult<()> {
        let files = self.collect_files(
            base_dir.unwrap_or(&self.base_path),
            &self.config.file_extensions_enc,
            &self.config.file_names_enc,
        )?;

        for entry in files {
            let path = entry.path();
            if path.exists() {
                fs::remove_file(path).with_context(|| format!("remove file: {:?}", path))?;
                debug!("Cleaned: {:?}", path);
            }
        }

        Ok(())
    }

    #[instrument]
    pub fn collect_files(
        &self,
        base_dir: &Path,
        extensions: &[String],
        filenames: &[String],
    ) -> ConfGuardResult<Vec<DirEntry>> {
        let accepted_extensions: HashSet<String> = extensions.iter().cloned().collect();
        let accepted_filenames: HashSet<String> = filenames.iter().cloned().collect();

        let files = WalkDir::new(base_dir)
            .into_iter()
            .filter_map(|result| {
                result
                    .map_err(|e| {
                        ConfGuardError::Internal(format!(
                            "Failed to walk directory {:?}: {}",
                            self.base_path, e
                        ))
                    })
                    .ok()
            })
            .filter(|entry| {
                let path = entry.path();
                path.extension().is_some_and(|ext| {
                    accepted_extensions.contains(&ext.to_string_lossy().to_string())
                }) || path.file_name().is_some_and(|name| {
                    accepted_filenames.contains(&name.to_string_lossy().to_string())
                })
            })
            .collect();

        Ok(files)
    }
}

#[instrument]
pub fn read_config(toml_path: &Path) -> ConfGuardResult<SopsConfig> {
    let toml_str = fs::read_to_string(toml_path)
        .with_context(|| format!("read TOML config file: {}", toml_path.display()))?;

    toml::from_str(&toml_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        .with_context(|| format!("parse TOML config from: {}", toml_path.display()))
}
