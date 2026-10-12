pub mod traits;
pub mod registry;
pub mod process;

pub use traits::{Plugin, PluginContext};
pub use registry::{PluginRegistry, TrailingWhitespacePlugin, WordCountPlugin};
pub use process::ProcessPluginRunner;
