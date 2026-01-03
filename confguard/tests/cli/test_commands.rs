use confguard::cli::args::{Cli, Commands};
use confguard::cli::execute_command;
use confguard::core::ConfGuard;
use confguard::errors::ConfGuardResult;
#[allow(unused_imports)]
use confguard::util::helper::testing::{setup_test_dir, teardown_test_dir};
use confguard::util::testing::create_file_with_content;
use rstest::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[fixture]
fn test_dir() -> PathBuf {
    setup_test_dir()
}

#[fixture]
fn cli_with_dir(test_dir: PathBuf) -> (Cli, PathBuf) {
    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: None,
    };
    (cli, test_dir)
}

#[fixture]
fn guarded_project(test_dir: PathBuf) -> (PathBuf, ConfGuard) {
    // Create and guard a project
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(envrc_path.to_str().unwrap(), "export FOO=bar\n").unwrap();

    let mut guard = ConfGuard::new_for_guarding(test_dir.clone(), true).unwrap();
    guard.guard(false).unwrap();

    (test_dir, guard)
}

#[rstest]
fn test_init_command(cli_with_dir: (Cli, PathBuf)) -> ConfGuardResult<()> {
    // Arrange
    let (mut cli, test_dir) = cli_with_dir;

    // Ensure the test directory exists
    fs::create_dir_all(&test_dir)?;

    cli.command = Some(Commands::Init {
        source_dir: test_dir.to_str().unwrap().to_string(),
        template_path: None,
    });

    // Act
    let result = execute_command(&cli);

    // Assert
    // println!("{:?}", result);
    assert!(result.is_ok());
    assert!(test_dir.join(".envrc").exists());
    let content = fs::read_to_string(test_dir.join(".envrc"))?;
    assert!(!content.is_empty());

    Ok(())
}

#[rstest]
fn test_guard_command(cli_with_dir: (Cli, PathBuf)) -> ConfGuardResult<()> {
    // Arrange
    let (mut cli, test_dir) = cli_with_dir;

    // Create initial .envrc
    let envrc_path = test_dir.join(".envrc");
    create_file_with_content(envrc_path.to_str().unwrap(), "export FOO=bar\n")?;

    cli.command = Some(Commands::Guard {
        source_dir: test_dir.to_str().unwrap().to_string(),
        absolute: false,
    });

    // Act
    let result = execute_command(&cli);

    // Assert
    assert!(result.is_ok());
    assert!(envrc_path.symlink_metadata()?.file_type().is_symlink());
    assert!(test_dir.join(".idea/runConfigurations/rsenv.sh").exists());

    Ok(())
}

#[rstest]
fn test_unguard_command(guarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, _) = guarded_project;
    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::Unguard {
            source_dir: test_dir.to_str().unwrap().to_string(),
        }),
    };

    // Act
    let result = execute_command(&cli);

    // Assert
    assert!(result.is_ok());
    let envrc_path = test_dir.join(".envrc");
    assert!(!envrc_path.symlink_metadata()?.file_type().is_symlink());
    assert!(envrc_path.exists());

    Ok(())
}

#[rstest]
fn test_show_command(guarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, _) = guarded_project;
    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::Show {
            source_dir: test_dir.to_str().unwrap().to_string(),
        }),
    };

    // Act
    let result = execute_command(&cli);

    // Assert
    assert!(result.is_ok());

    Ok(())
}

#[rstest]
fn test_guard_one_command(guarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, _) = guarded_project;

    // Create a file to guard
    let test_file = test_dir.join("config.yml");
    create_file_with_content(test_file.to_str().unwrap(), "key: value\n")?;

    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::GuardOne {
            source_dir: test_dir.to_str().unwrap().to_string(),
            source_path: test_file.to_str().unwrap().to_string(),
        }),
    };

    // Act
    let result = execute_command(&cli);

    // Assert
    assert!(result.is_ok());
    assert!(test_file.symlink_metadata()?.file_type().is_symlink());

    Ok(())
}

#[rstest]
fn test_relink_command(guarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, guard) = guarded_project;
    let dot_envrc = guard.target_dir.unwrap().join("dot.envrc");

    // Remove original .envrc to simulate broken link
    fs::remove_file(test_dir.join(".envrc"))?;

    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::Relink {
            envrc_path: dot_envrc.to_str().unwrap().to_string(),
        }),
    };

    // Act
    let result = execute_command(&cli);

    // Assert
    assert!(result.is_ok());
    assert!(test_dir.join(".envrc").exists());
    assert!(test_dir
        .join(".envrc")
        .symlink_metadata()?
        .file_type()
        .is_symlink());

    Ok(())
}

#[rstest]
fn test_fix_run_config_command(guarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, _) = guarded_project;

    // Remove existing run config to test recreation
    let run_config = test_dir.join(".idea/runConfigurations/rsenv.sh");
    if run_config.exists() {
        fs::remove_file(&run_config)?;
    }

    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::FixRunConfig {
            source_dir: test_dir.to_str().unwrap().to_string(),
        }),
    };

    // Act
    let result = execute_command(&cli);

    // Assert
    assert!(result.is_ok());
    assert!(run_config.exists());
    assert!(run_config.metadata()?.permissions().mode() & 0o111 != 0); // Check executable

    Ok(())
}

#[rstest]
fn test_info_command() -> ConfGuardResult<()> {
    // Arrange
    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::Info),
    };

    // Act
    let result = execute_command(&cli);

    // Assert
    assert!(result.is_ok());

    Ok(())
}

#[rstest]
fn test_replace_link_command(guarded_project: (PathBuf, ConfGuard)) -> ConfGuardResult<()> {
    // Arrange
    let (test_dir, _) = guarded_project;
    let envrc_path = test_dir.join(".envrc");

    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::ReplaceLink {
            link: envrc_path.to_str().unwrap().to_string(),
        }),
    };

    // Act
    let result = execute_command(&cli);

    // Assert
    assert!(result.is_ok());
    assert!(!envrc_path.symlink_metadata()?.file_type().is_symlink());
    assert!(envrc_path.exists());

    Ok(())
}
#[rstest]
fn test_sops_init(test_dir: PathBuf) -> ConfGuardResult<()> {
    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::SopsInit {
            template_path: None,
        }),
    };

    // Execute command
    execute_command(&cli)?;

    // Verify config file was created
    let config_path = test_dir.join("confguard.toml");
    assert!(config_path.exists(), "Configuration file should exist");

    // Verify content
    let content = fs::read_to_string(&config_path)?;
    assert!(
        content.contains("file_extensions_enc"),
        "Should contain encryption configuration"
    );
    assert!(
        content.contains("gpg_key"),
        "Should contain GPG key configuration"
    );

    Ok(())
}
