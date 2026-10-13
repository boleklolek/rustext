pub mod traits;
pub mod registry;
pub mod process;
pub mod manager;

pub use traits::{Plugin, PluginContext};
pub use registry::{PluginRegistry, TrailingWhitespacePlugin, WordCountPlugin};
pub use process::ProcessPluginRunner;
pub use manager::{ExtensionCommand, ExtensionInfo, ExtensionManager};

