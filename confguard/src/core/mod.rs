pub mod config;
mod guard;

pub use config::Settings;
pub use guard::ConfGuard;
pub use guard::ConfGuardBuilder;
pub use guard::ConfGuardBuilderError; // Export the builder error type
                                      // Replace SETTINGS with functions to access settings
pub use config::settings; // We'll create this function
