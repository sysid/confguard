#![allow(unused_imports)]

pub mod cli;
pub mod core;
pub mod errors;
pub mod sops;
pub mod util;

// Re-export for convenience
pub use errors::{ConfGuardContext, ConfGuardError, ConfGuardResult};

#[cfg(test)]
mod tests {
    use crate::util::helper::testing;
    use tracing::debug;

    #[test]
    fn test_dlog_macro() {
        testing::init_test_setup(); // Ensure logging is configured
        let test_var = vec![1, 2, 3];
        debug!("Test variable: {:?}, {:?}", &test_var, "string");
    }
}
