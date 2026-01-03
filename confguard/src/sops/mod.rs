// src/sops/mod.rs
pub mod crypto;
pub mod gitignore;
pub mod manager;

use crate::errors::{ConfGuardError, ConfGuardResult};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
