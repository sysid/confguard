# ConfGuard

[![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD_3--Clause-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)

A highly opinionated configuration management tool for securing and managing sensitive environment
files across different deployment stages.

ConfGuard helps manage sensitive configuration and data by:
- Moving configuration and files to a secure, centralized location
- Creating symbolic links to maintain project structure
- Integrating with SOPS to secure the centralized sensitive data store

### The Problem

`.envrc` files (used by [direnv](https://direnv.net/)) contain sensitive data:
- Database credentials
- API keys and secrets
- Cloud provider tokens

Projects may have other sensitive.

These files live in your project directory. One wrong `git add .` and your
secrets are in version control history forever.

### The Solution

ConfGuard creates a **sentinel directory** — a secure vault for each project's sensitive files:

```
your-project/                         $CONFGUARD_BASE_DIR/guarded/project-uuid/
├── .envrc ──────── symlink ────────► ├── dot.envrc         (your config)
├── certs/key.pem ── symlink ───────► ├── certs/key.pem     (your certs)
└── src/                              └── environments/
                                          ├── local.env
                                          ├── test.env
                                          └── prod.env
```

**The core idea:**

1. **`.envrc` is the entry point** — When guarded, it creates your sentinel directory and a
   bidirectional link
2. **Sentinel directory is the secure vault** — Lives outside any git repo, holds all sensitive files for this project
3. **Symlinks replace originals** — Your project structure stays intact, but files are just pointers (safe to commit)
4. **Add more files anytime** — Use `guard-one` to move additional sensitive files (certificates, keys, configs) into the vault
5. **Protection (encryption)** via builtin SOPS integration

The linking is via a dedicated confguard section which will be added to `.envrc`.

## Table of Contents

- [Architecture](#architecture)
- [Getting Started](#getting-started)
- [Usage](#usage)
- [Configuration](#configuration)
- [Development](#development)
- [Testing](#testing)
- [License](#license)

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

### Environment Files
When guarding a project, ConfGuard automatically creates multiple environment files:

| File | Purpose |
|------|---------|
| `local.env` | Local development environment |
| `test.env` | Testing environment |
| `int.env` | Integration/staging environment |
| `prod.env` | Production environment |

Each file contains `export RUN_ENV="<environment>"` to identify the active environment.

**Why multiple environments?**

Keep environment-specific variables (database URLs, API keys, feature flags) in separate files, then switch between them by changing one variable rather than maintaining multiple `.envrc` files or manually editing values.

**Switching environments:**

The `RUN_ENV` variable determines which environment is active:

```bash
rsenv build "$SOPS_PATH/environments/${RUN_ENV:-local}.env"
```

Setting `RUN_ENV=int` loads `int.env` instead of the default `local.env`.

This is best used in combination with [rsenv](https://github.com/sysid/rs-env).

## Getting Started

### Prerequisites

- **Rust**: 1.56.0 or later (2021 edition)
- **SOPS**: Required for encryption features ([install SOPS](https://github.com/getsops/sops))
- **GPG**: Required for PGP-based encryption
- **direnv**: Recommended for automatic environment loading

### Installation

```bash
```bash
cargo install confguard
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

Creates the sentinal directory, moves `.envrc` to it and creates bidirectional linking via symlink
and `.envrc` confguard section (SOPS_PATH).

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

**Metadata appended to .envrc:**
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
