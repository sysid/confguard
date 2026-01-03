# ConfGuard

A highly opinionated configuration management tool for securing and managing sensitive environment
files across different deployment stages.

[![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD_3--Clause-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)

## Overview

ConfGuard helps manage sensitive configuration files by:
- Moving configuration files to a secure, centralized location
- Creating symbolic links to maintain project structure
- Supporting multiple environment configurations (local, test, integration, production)
- Integrating with SOPS for encryption/decryption workflows
- Providing IDE integration for development workflows

## Table of Contents

- [Features](#features)
- [Architecture](#architecture)
- [Getting Started](#getting-started)
- [Usage](#usage)
- [Configuration](#configuration)
- [Development](#development)
- [Testing](#testing)
- [License](#license)

## Features

### Core Functionality
- **Guard Projects**: Secure existing projects by moving `.envrc` files to a managed location
- **Multi-Environment Support**: Automatically creates environment files for different stages
- **Symbolic Link Management**: Maintains project structure while securing configurations
- **SOPS Integration**: Built-in support for encrypting/decrypting sensitive files (including binary files)
- **IDE Integration**: Creates IntelliJ/VSCode run configurations

### Environment Files
When guarding a project, ConfGuard automatically creates multiple environment files:

| File | Purpose |
|------|---------|
| `local.env` | Local development environment |
| `test.env` | Testing environment |
| `int.env` | Integration/staging environment |
| `prod.env` | Production environment |

Each file contains `export RUN_ENV="<environment>"` to identify the active environment.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              ConfGuard Architecture                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Your Project                          $CONFGUARD_BASE_DIR                  │
│  ├── .envrc ──────symlink────────────► ├── guarded/                         │
│  └── ...                               │   └── <project-uuid>/              │
│                                        │       ├── dot.envrc                │
│                                        │       └── environments/            │
│                                        │           ├── local.env            │
│                                        │           ├── test.env             │
│                                        │           ├── int.env              │
│                                        │           └── prod.env             │
│                                        └── confguard.toml                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Source Structure

```
confguard/src/
├── main.rs        # Entry point, CLI parsing, logging setup
├── lib.rs         # Library exports
├── errors.rs      # Error types with thiserror
├── cli/           # Command-line interface
│   ├── args.rs    # Command definitions (clap)
│   └── commands.rs# Command execution logic
├── core/          # Core guarding functionality
│   ├── guard.rs   # ConfGuard struct and operations
│   └── config.rs  # Configuration management
├── sops/          # SOPS encryption integration
│   ├── manager.rs # Encryption workflow orchestration
│   ├── crypto.rs  # SOPS binary invocation
│   └── gitignore.rs # Gitignore management
└── util/          # Utility functions
    ├── path/      # Path manipulation utilities
    └── helper/    # General helpers
```

## Getting Started

### Prerequisites

- **Rust**: 1.56.0 or later (2021 edition)
- **SOPS**: Required for encryption features ([install SOPS](https://github.com/getsops/sops))
- **GPG**: Required for PGP-based encryption
- **direnv**: Recommended for automatic environment loading

### Installation

**From source:**
```bash
git clone https://github.com/sysid/rs-cg.git
cd rs-cg
make install
```

**Or using cargo:**
```bash
cd confguard
cargo install --path .
```

**Verify installation:**
```bash
confguard --info
```

### Quick Start

1. **Initialize SOPS configuration:**
   ```bash
   confguard sops-init
   ```

2. **Guard an existing project:**
   ```bash
   confguard guard /path/to/your/project
   ```

3. **Verify the guard:**
   ```bash
   confguard show /path/to/your/project
   ```

## Usage

### Global Options

```bash
confguard [OPTIONS] [COMMAND]
```

| Option | Description |
|--------|-------------|
| `-c, --config <FILE>` | Custom config file path |
| `-d, --debug` | Increase verbosity (-d=INFO, -dd=DEBUG, -ddd=TRACE) |
| `--generate <SHELL>` | Generate shell completions (bash, zsh, fish) |
| `--info` | Display version and base directory |

### Linking Modes

ConfGuard supports two linking modes controlled by the `--absolute` flag:

| Mode | Flag | Symlink Content | Example |
|------|------|-----------------|---------|
| **Relative** (default) | none | `../../../xxx/rs-cg/guarded/proj-uuid/dot.envrc` | Portable if both dirs move together |
| **Absolute** | `--absolute` | `/Users/you/xxx/rs-cg/guarded/proj-uuid/dot.envrc` | Works regardless of project location |

**Relative linking** (default) calculates the path from the symlink location to the target. Use this when your home directory might move or be mounted differently.

---

### Core Commands

#### `guard` - Guard a Project Directory

Moves `.envrc` to a secure location and creates a symlink in its place.

```bash
confguard guard <source_dir> [--absolute]
```

| Parameter | Required | Description |
|-----------|----------|-------------|
| `source_dir` | Yes | Project directory containing `.envrc` |
| `--absolute` | No | Use absolute paths instead of relative (default: false) |

**What it creates:**

1. **Sentinel directory**: `$CONFGUARD_BASE_DIR/guarded/{project-name}-{uuid}/`
2. **Moved file**: `dot.envrc` (original `.envrc` content)
3. **Environment files**: `environments/{local,test,int,prod}.env`
4. **Symlink**: `.envrc` -> `dot.envrc` (relative or absolute based on flag)
5. **IDE config**: `.idea/runConfigurations/rsenv.sh`

**Metadata appended to dot.envrc:**
```bash
#------------------------------- confguard start --------------------------------
# config.relative = true
# config.version = 3
# state.sentinel = 'myproject-a1b2c3d4'
# state.timestamp = '2026-01-03T...'
# state.sourceDir = '~/dev/myproject'
export SOPS_PATH=$HOME/xxx/rs-cg/guarded/myproject-a1b2c3d4
dotenv $SOPS_PATH/environments/local.env
#-------------------------------- confguard end ---------------------------------
```

**Use case:** Secure a project's environment configuration while maintaining seamless access via the original `.envrc` path. No sensitive information ends up in version control.

---

#### `unguard` - Remove Guarding from a Project

Restores the original file structure by replacing the symlink with the actual content.

```bash
confguard unguard <source_dir>
```

| Parameter | Required | Description |
|-----------|----------|-------------|
| `source_dir` | Yes | Project directory with guarded `.envrc` |

**What gets restored:**
- `.envrc` symlink replaced with `dot.envrc` content
- Confguard metadata section removed from `.envrc`
- All symlinks pointing to this project's sentinel directory are replaced with their targets

**What is NOT removed:**
- Sentinel directory remains at `$CONFGUARD_BASE_DIR/guarded/{sentinel}/`
- Environment files and encrypted files are preserved
- IDE run configurations remain

**Use case:** Permanently disconnect a project from ConfGuard management, e.g., before archiving or transferring the project.

---

#### `guard-one` - Guard a Single File

Guards an additional file within an already-guarded project.

```bash
confguard guard-one <source_dir> <source_path>
```

| Parameter | Required | Description |
|-----------|----------|-------------|
| `source_dir` | Yes | Project directory (must already be guarded) |
| `source_path` | Yes | File to guard (must be within project) |

**Validation:**
- Project must already be guarded
- File must exist
- File path must start with `source_dir` (prevents guarding arbitrary system files)

**Behavior:**
- Preserves relative path structure: `/project/config/secrets.env` -> `{sentinel}/config/secrets.env`
- Always creates **absolute** symlinks (unlike `guard` which defaults to relative)

**Use case:** Add sensitive files (credentials, keys) to an existing guard setup without re-guarding the entire project.

---

#### `relink` - Recreate Symbolic Link

Recreates the `.envrc` symlink when it's been deleted or broken.

```bash
confguard relink <envrc_path>
```

| Parameter | Required | Description |
|-----------|----------|-------------|
| `envrc_path` | Yes | Path to the `dot.envrc` file in the guarded location |

**How it works:**
1. Reads the `state.sourceDir` from the confguard section in `dot.envrc`
2. Creates a new symlink at the original project location
3. Respects the original `config.relative` setting

**Use cases:**
- Symlink accidentally deleted
- Fresh clone where `.envrc` was gitignored
- Project directory moved but sentinel directory unchanged

---

#### `replace-link` - Replace Symlink with Target

Converts a symlink into a regular file containing the target's content.

```bash
confguard replace-link <link>
```

| Parameter | Required | Description |
|-----------|----------|-------------|
| `link` | Yes | Path to the symbolic link |

**Behavior:**
- Removes the symlink
- Moves the target file to the symlink's location
- Works for both files and directories

**Use case:** Inverse to `replace-link`, but works on ANY symlink, not just confguard-managed ones.

| Command      | Operation                                  |
|--------------|--------------------------------------------|
| guard-one    | file → sentinel dir, create symlink        |
| replace-link | symlink → move target back, remove symlink |


**`replace-link` vs `unguard`:**

| Aspect | `replace-link` | `unguard` |
|--------|----------------|-----------|
| **Scope** | Single symlink | Entire project |
| **Target** | Any symlink you specify | Only guarded projects |
| **Metadata** | Untouched | Removes confguard section |
| **Other symlinks** | Untouched | Replaces ALL symlinks pointing to sentinel |
| **Guard status** | Project remains guarded | Project no longer guarded |


replace-link /path/to/symlink
- Operates on ONE specific symlink
- Replaces it with the file it points to
- The project is still guarded (other symlinks, metadata intact)
- Use case: "freeze" one file while keeping the guard

unguard /path/to/project
- Operates on the ENTIRE project
- Replaces .envrc symlink AND removes confguard metadata section
- Walks the project and replaces ALL symlinks pointing to sentinel dir
- Use case: completely disconnect from ConfGuard
---

#### `init` - Initialize Project with .envrc

Creates a new `.envrc` file from a template.

```bash
confguard init <source_dir> [--template <path>]
```

| Parameter | Required | Description |
|-----------|----------|-------------|
| `source_dir` | Yes | Directory to initialize |
| `--template` | No | Custom template file (default: embedded template) |

**Validation:**
- Fails if `.envrc` already exists
- Fails if project is already guarded

**Note:** This command does NOT guard the project. Run `guard` separately after initialization.

---

#### `show` - Display Guard Status

Shows the current configuration of a guarded project.

```bash
confguard show <source_dir>
```

**Output fields:**

| Field | Description |
|-------|-------------|
| `version` | Configuration version (currently 3) |
| `source_dir` | Absolute path to project |
| `is_relative` | Whether symlinks use relative paths |
| `confguard_base_dir` | Base directory for all ConfGuard data |
| `target_dir` | Path to this project's guarded files |
| `sentinel` | Unique identifier (`{name}-{uuid}`) |

---

#### `settings` - Display Global Settings

Shows the current ConfGuard configuration.

```bash
confguard settings
```

**Output:**
```
Settings:
  Base: /Users/you/xxx/rs-cg
  Version: 3
```

---

#### `fix-run-config` - Update IDE Configuration

Repairs or updates the IntelliJ run configuration script.

```bash
confguard fix-run-config <source_dir>
```

**What it does:**
- Creates/updates `.idea/runConfigurations/rsenv.sh`
- Makes the script executable

**Use case:** Restore IDE integration after the run configuration was deleted or after updating ConfGuard.

---

### IDE Integration

ConfGuard creates `.idea/runConfigurations/rsenv.sh` during the guard operation:

```bash
#!/usr/bin/env bash
[[ -f "$SOPS_PATH/environments/${RUN_ENV:-local}.env" ]] && rsenv build "$SOPS_PATH/environments/${RUN_ENV:-local}.env"
```

**How it works with IntelliJ:**

1. Configure your Run Configuration to execute this script before launch
2. Set `RUN_ENV` to switch environments (`local`, `test`, `int`, `prod`)
3. The script calls `rsenv build` to process the environment file
4. Environment variables are injected into your application

**Requirements:**
- External `rsenv` tool must be installed
- IntelliJ EnvFile plugin (optional, for additional env file support)

---

### SOPS Commands

SOPS autodetects file formats, including binary files. Unknown extensions are automatically treated as binary data.

#### Security Model: Gitignore Protection

ConfGuard prevents accidental commits of plaintext secrets through automatic `.gitignore` management. When you run `sops-enc`, it:

1. **Encrypts** matching files → creates `.enc` versions
2. **Updates `.gitignore`** → adds patterns to exclude plaintext files
3. After `sops-clean` → only encrypted files remain

**What gets added to `.gitignore`:**

```
# ---------------------------------- confguard-start -----------------------------------
*.env  # sops-managed 2024-01-15 10:30:45
*.envrc  # sops-managed 2024-01-15 10:30:45
dot_pgpass  # sops-managed 2024-01-15 10:30:45
dot_pypirc  # sops-managed 2024-01-15 10:30:45
kube_config  # sops-managed 2024-01-15 10:30:45
# ---------------------------------- confguard-end -----------------------------------
```

Patterns come from `confguard.toml`:
- `file_extensions_enc` → `*.{ext}` patterns
- `file_names_enc` → exact filename patterns

The confguard markers allow updates without destroying your existing `.gitignore` entries.

**Typical workflow:**

```bash
# 1. Create/edit plaintext secrets
vim secrets.env

# 2. Encrypt (also updates .gitignore)
confguard sops-enc

# 3. Remove plaintext (safe - gitignore already protects)
confguard sops-clean

# 4. Commit only encrypted files
git add secrets.env.enc .gitignore
git commit -m "Add encrypted secrets"
```

#### `sops-init` - Initialize SOPS Configuration

```bash
confguard sops-init [--dir <path>] [--template <path>]
```

| Option | Description |
|--------|-------------|
| `--dir` | Target directory (default: `$CONFGUARD_BASE_DIR`) |
| `--template` | Custom template file |

Creates `confguard.toml` with encryption configuration.

#### `sops-enc` - Encrypt Files

```bash
confguard sops-enc [--dir <path>]
```

Encrypts all files matching `file_extensions_enc` and `file_names_enc` patterns. Output files have
`.enc` suffix. Also updates `.gitignore` to exclude plaintext files from version control (see
[Security Model](#security-model-gitignore-protection) above).

#### `sops-dec` - Decrypt Files

```bash
confguard sops-dec [--dir <path>]
```

Decrypts all `.enc` files (or files matching `file_extensions_dec`/`file_names_dec`).

#### `sops-clean` - Remove Plaintext Files

```bash
confguard sops-clean [--dir <path>]
```

Removes plaintext files that match encryption patterns (after encryption, to leave only `.enc` files).

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `CONFGUARD_BASE_DIR` | Base directory for confguard operations | `$HOME/xxx/rs-cg` |
| `CONFGUARD_VERSION` | Override configuration version | - |
| `RUST_LOG` | Logging level (DEBUG, INFO, WARN) | WARN |

### File Structure

When a project is guarded:
```
$CONFGUARD_BASE_DIR/
├── guarded/
│   └── <project-name-uuid>/
│       ├── dot.envrc              # Original .envrc content
│       └── environments/
│           ├── local.env          # export RUN_ENV="local"
│           ├── test.env           # export RUN_ENV="test"
│           ├── int.env            # export RUN_ENV="int"
│           └── prod.env           # export RUN_ENV="prod"
└── confguard.toml                 # SOPS configuration
```

### SOPS Configuration

Create `confguard.toml` in your base directory:

```toml
gpg_key = "your-gpg-key-fingerprint"

# Extensions to encrypt
file_extensions_enc = [
  "envrc",
  "env",
  "p12",       # binary - PKCS#12 certificates
  "keystore",  # binary - Java keystores
]

# Specific filenames to encrypt
file_names_enc = [
  "dot_pypirc",
  "dot_pgpass",
  "kube_config",
]

# Extensions to decrypt (encrypted files)
file_extensions_dec = ["enc"]

# Specific filenames to decrypt
file_names_dec = []
```

### SOPS Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           ENCRYPTION FLOW                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  confguard.toml                                                             │
│  ├── file_extensions_enc: ["envrc", "env", "p12", ...]                      │
│  └── file_names_enc: [...]                                                  │
│           │                                                                 │
│           ▼                                                                 │
│  collect_files()                                                            │
│  └── Filter by extension OR filename                                        │
│           │                                                                 │
│           ▼                                                                 │
│  encrypt_file() [8 parallel threads]                                        │
│  └── sops -e --pgp <key> --output <out> <in>                                │
│           │                                                                 │
│           ▼                                                                 │
│  SOPS auto-detects file type                                                │
│  ├── Known extension (.json, .yaml, .env) → structured format               │
│  └── Unknown extension → binary format (autodetected)                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Development

### Prerequisites

- Rust toolchain (rustup recommended)
- GNU Make

### Build Commands

```bash
make build          # Build release binary
make test           # Run tests
make style          # Format code (cargo fmt)
make lint           # Run clippy
make doc            # Generate documentation
```

### Installation

```bash
make install        # Install to ~/bin/
make uninstall      # Remove installation
```

### Test Environment

```bash
make init-env       # Create test environment
make show-env       # Show test environment structure
```

### Version Management

```bash
make bump-patch     # Bump patch version (x.y.Z)
make bump-minor     # Bump minor version (x.Y.z)
make bump-major     # Bump major version (X.y.z)
```

## Testing

Tests require special environment setup and must run single-threaded due to shared state:

```bash
# Run all tests
make test

# Or manually:
RUST_LOG=DEBUG cargo test -- --test-threads=1

# Run specific test module
cargo test core::guard -- --test-threads=1
```

### Test Structure

```
confguard/tests/
├── cli/           # CLI integration tests
├── core/          # Core functionality tests
├── sops/          # SOPS integration tests
├── util/          # Utility function tests
└── resources/     # Test fixtures
```

## Dependencies

Key dependencies:

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `thiserror` | Error handling |
| `tracing` | Logging infrastructure |
| `serde` / `toml` | Configuration parsing |
| `uuid` | Unique sentinel identifiers |
| `walkdir` | Directory traversal |
| `rayon` | Parallel file processing |

## License

BSD-3-Clause - See [LICENSE](LICENSE) for details.
