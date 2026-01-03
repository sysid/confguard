pub mod helper;
pub mod path;

pub use helper::*;
pub use path::*;

use include_dir::{include_dir, Dir};

pub const RESOURCES_DIR: Dir = include_dir!("./resources");
