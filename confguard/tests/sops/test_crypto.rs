use confguard::errors::ConfGuardResult;
use confguard::sops::crypto::*;
use confguard::util::testing::setup_test_dir;
use rstest::{fixture, rstest};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[fixture]
fn test_dir() -> PathBuf {
    setup_test_dir()
}

#[ignore = "must be run from terminal due to password input"]
#[rstest]
fn test_encrypt_decrypt_cycle(test_dir: PathBuf) -> ConfGuardResult<()> {
    let crypto = SopsCrypto::new("60A4127E82E218297532FAB6D750B66AE08F3B90".to_string());

    // Create test file
    let test_file = test_dir.join("test.env");
    let mut file = File::create(&test_file)?;
    writeln!(file, "SECRET=test")?;

    // Test encryption
    let encrypted_file = test_dir.join("test.env.enc");
    crypto.encrypt_file(&test_file, &encrypted_file)?;
    assert!(encrypted_file.exists());

    // Test decryption
    let decrypted_file = test_dir.join("test.decrypted.env");
    crypto.decrypt_file(&encrypted_file, &decrypted_file)?;
    assert!(decrypted_file.exists());

    // Verify decrypted content matches original
    let decrypted_content = fs::read_to_string(&decrypted_file)?;
    assert_eq!(decrypted_content.trim(), "SECRET=test");

    // teardown_test_dir(&test_dir);
    Ok(())
}
