// src/sops/crypto.rs
use crate::errors::{ConfGuardError, ConfGuardResult};
use colored::Colorize;
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{io, thread};
use tracing::debug;

#[derive(Debug, Clone, Deserialize)]
pub struct SopsCrypto {
    gpg_key: String,
}

impl SopsCrypto {
    pub fn new(gpg_key: String) -> Self {
        Self { gpg_key }
    }

    pub fn encrypt_file(&self, input: &Path, output: &Path) -> ConfGuardResult<()> {
        debug!(
            "{:?} Encrypting file: {:?} -> {:?}",
            thread::current().id(),
            input,
            output
        );

        let output = Command::new("sops")
            .arg("-e")
            .arg("--pgp")
            .arg(&self.gpg_key)
            .arg("--output")
            .arg(output)
            .arg(input) // the input file is the last argument
            .output()?;

        if !output.status.success() {
            eprintln!(
                "{}",
                format!(
                    "{:?}: sops failed: {:?}. Code: {:?}",
                    thread::current().id(),
                    input,
                    output.status.code()
                )
                .red()
            );
        }

        // Print stdout and stderr
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;

        Ok(())
    }

    pub fn decrypt_file(&self, input: &Path, output: &Path) -> ConfGuardResult<()> {
        debug!(
            "{:?} Decrypting file: {:?} -> {:?}",
            thread::current().id(),
            input,
            output
        );

        let output = if output.extension().unwrap_or_default() == "env" {
            debug!("Decrypt dotenv {:?} -> {:?}", input, output);
            Command::new("sops")
                .arg("-d")
                .arg("--pgp")
                .arg(&self.gpg_key)
                .arg("--input-type")
                .arg("dotenv")
                .arg("--output-type")
                .arg("dotenv")
                .arg("--output")
                .arg(output)
                .arg(input) // the input file is the last argument
                .output()?
        } else {
            debug!("Decrypt dotenv {:?} -> {:?}", input, output);
            Command::new("sops")
                .arg("-d")
                .arg("--pgp")
                .arg(&self.gpg_key)
                .arg("--output")
                .arg(output)
                .arg(input) // the input file is the last argument
                .output()?
        };

        if !output.status.success() {
            eprintln!(
                "{}",
                format!(
                    "{:?}: sops failed: {:?}. Code: {:?}",
                    thread::current().id(),
                    input,
                    output.status.code()
                )
                .red()
            );
        }

        // Print stdout and stderr
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;

        Ok(())
    }
}
