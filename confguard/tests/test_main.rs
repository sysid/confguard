use confguard::cli::args::{Cli, Commands};
use confguard::cli::execute_command;
use confguard::errors::ConfGuardResult;
use rstest::*;
use std::fs;
use std::path::PathBuf;

#[fixture]
fn test_dir() -> PathBuf {
    let dir = tempfile::tempdir().unwrap().keep();
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[fixture]
fn unguarded_project(test_dir: PathBuf) -> PathBuf {
    // Create a simple .envrc file
    let envrc_path = test_dir.join(".envrc");
    fs::write(&envrc_path, "export FOO=bar\n").unwrap();
    test_dir
}

#[fixture]
fn guarded_project(test_dir: PathBuf) -> PathBuf {
    // First create unguarded project
    let test_dir = unguarded_project(test_dir);

    // Then guard it
    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::Guard {
            source_dir: test_dir.to_str().unwrap().to_string(),
            absolute: false,
        }),
    };
    execute_command(&cli).unwrap();
    test_dir
}

#[rstest]
fn test_smoke_info(_test_dir: PathBuf) -> ConfGuardResult<()> {
    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::Info),
    };
    execute_command(&cli)
}

#[rstest]
fn test_smoke_init(test_dir: PathBuf) -> ConfGuardResult<()> {
    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::Init {
            source_dir: test_dir.to_str().unwrap().to_string(),
            template_path: None,
        }),
    };
    execute_command(&cli)?;
    assert!(test_dir.join(".envrc").exists());
    Ok(())
}

#[rstest]
fn test_smoke_guard(unguarded_project: PathBuf) -> ConfGuardResult<()> {
    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::Guard {
            source_dir: unguarded_project.to_str().unwrap().to_string(),
            absolute: false,
        }),
    };
    execute_command(&cli)?;
    assert!(unguarded_project
        .join(".envrc")
        .symlink_metadata()?
        .file_type()
        .is_symlink());
    Ok(())
}

#[rstest]
fn test_smoke_show(guarded_project: PathBuf) -> ConfGuardResult<()> {
    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::Show {
            source_dir: guarded_project.to_str().unwrap().to_string(),
        }),
    };
    execute_command(&cli)
}

#[rstest]
fn test_smoke_unguard(guarded_project: PathBuf) -> ConfGuardResult<()> {
    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::Unguard {
            source_dir: guarded_project.to_str().unwrap().to_string(),
        }),
    };
    execute_command(&cli)?;
    assert!(!guarded_project
        .join(".envrc")
        .symlink_metadata()?
        .file_type()
        .is_symlink());
    Ok(())
}

#[rstest]
fn test_smoke_guard_one(guarded_project: PathBuf) -> ConfGuardResult<()> {
    // Create a test file to guard
    let test_file = guarded_project.join("test.yml");
    fs::write(&test_file, "key: value")?;

    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::GuardOne {
            source_dir: guarded_project.to_str().unwrap().to_string(),
            source_path: test_file.to_str().unwrap().to_string(),
        }),
    };
    execute_command(&cli)?;
    assert!(test_file.symlink_metadata()?.file_type().is_symlink());
    Ok(())
}

#[rstest]
fn test_smoke_relink(guarded_project: PathBuf) -> ConfGuardResult<()> {
    // Find the target envrc
    let envrc_path = fs::read_link(guarded_project.join(".envrc"))?;

    // Remove the original link
    fs::remove_file(guarded_project.join(".envrc"))?;

    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::Relink {
            envrc_path: envrc_path.to_str().unwrap().to_string(),
        }),
    };
    execute_command(&cli)?;
    assert!(guarded_project
        .join(".envrc")
        .symlink_metadata()?
        .file_type()
        .is_symlink());
    Ok(())
}

#[rstest]
fn test_smoke_replace_link(guarded_project: PathBuf) -> ConfGuardResult<()> {
    let envrc = guarded_project.join(".envrc");

    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::ReplaceLink {
            link: envrc.to_str().unwrap().to_string(),
        }),
    };
    execute_command(&cli)?;
    assert!(!envrc.symlink_metadata()?.file_type().is_symlink());
    Ok(())
}

#[rstest]
fn test_smoke_fix_run_config(guarded_project: PathBuf) -> ConfGuardResult<()> {
    let cli = Cli {
        base_dir: None,
        debug: 0,
        generator: None,
        command: Some(Commands::FixRunConfig {
            source_dir: guarded_project.to_str().unwrap().to_string(),
        }),
    };
    execute_command(&cli)?;
    assert!(guarded_project
        .join(".idea/runConfigurations/rsenv.sh")
        .exists());
    Ok(())
}
