use clap::{Args, Parser, Subcommand, ValueHint};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser, Debug, PartialEq)]
#[command(author, version, about = "A security guard for your config files")]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Override base directory (default: CONFGUARD_BASE_DIR or $HOME/xxx/rs-cg)
    #[arg(long = "base-dir", value_hint = ValueHint::DirPath)]
    pub base_dir: Option<PathBuf>,

    /// Turn debugging information on (multiple -d flags increase verbosity)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    /// Generate shell completions
    #[arg(long = "generate", value_enum)]
    pub generator: Option<Shell>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Show program information and configuration details
    Info,

    /// Show current configuration of a guarded project
    Show {
        /// Project directory containing .envrc
        #[arg(value_hint = ValueHint::DirPath)]
        source_dir: String,
    },

    /// Guard a project directory
    Guard {
        /// Project directory containing .envrc
        #[arg(value_hint = ValueHint::DirPath)]
        source_dir: String,

        /// Use absolute paths instead of relative ones
        #[arg(long = "absolute")]
        absolute: bool,
    },

    /// Remove guarding from a project
    Unguard {
        /// Project directory containing .envrc
        #[arg(value_hint = ValueHint::DirPath)]
        source_dir: String,
    },

    /// Guard a single file within a guarded project
    GuardOne {
        /// Project directory containing .envrc
        #[arg(value_hint = ValueHint::DirPath)]
        source_dir: String,

        /// File to be guarded
        #[arg(value_hint = ValueHint::FilePath)]
        source_path: String,
    },

    /// Recreate symbolic link to dot.envrc
    Relink {
        /// Path to dot.envrc file
        #[arg(value_hint = ValueHint::FilePath)]
        envrc_path: String,
    },

    /// Replace a symbolic link with its target
    ReplaceLink {
        /// Path to symbolic link
        #[arg(value_hint = ValueHint::FilePath)]
        link: String,
    },

    /// Fix/update IDE run configuration
    FixRunConfig {
        /// Project directory containing .envrc
        #[arg(value_hint = ValueHint::DirPath)]
        source_dir: String,
    },

    /// Initialize a directory with .envrc
    Init {
        /// Directory to initialize
        #[arg(value_hint = ValueHint::DirPath)]
        source_dir: String,

        /// Optional custom template file
        #[arg(long = "template", value_hint = ValueHint::FilePath)]
        template_path: Option<String>,
    },

    /// Encrypt files using SOPS
    SopsEnc {
        /// Directory to scan for files (default: base directory)
        #[arg(long = "dir", value_hint = ValueHint::DirPath)]
        dir: Option<String>,
    },

    /// Decrypt SOPS encrypted files
    SopsDec {
        /// Directory to scan for files (default: base directory)
        #[arg(long = "dir", value_hint = ValueHint::DirPath)]
        dir: Option<String>,
    },

    /// Clean plaintext files that have encrypted versions
    SopsClean {
        /// Directory to scan for files (default: base directory)
        #[arg(long = "dir", value_hint = ValueHint::DirPath)]
        dir: Option<String>,
    },

    /// Create initial confguard.toml for SOPS encryption
    SopsInit {
        /// Custom confguard.toml template file to copy from
        #[arg(long = "template", value_hint = ValueHint::FilePath)]
        template_path: Option<String>,
    },
}
