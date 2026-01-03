//! Integration tests for the library

mod cli;
mod core;
mod sops;

use confguard::util::helper::testing;

mod util {
    pub mod path {
        mod test_link;
        mod test_move_and_link;
    }
}

// Re-export testing utilities for use in other test modules
pub use testing::{print_active_env_vars, setup_test_dir, teardown_test_dir, TEST_ENV_VARS};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lib_initialization() {
        print_active_env_vars(TEST_ENV_VARS);
        let test_dir = setup_test_dir();
        assert!(test_dir.exists());
        teardown_test_dir(&test_dir);
    }
}
