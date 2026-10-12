//! Plugin traits and context definitions.
//! Zero third-party dependencies, purely standard library Rust.

use std::path::PathBuf;

/// Context passed to plugins containing editor state information.
#[derive(Debug, Default)]
pub struct PluginContext {
    pub file_path: Option<PathBuf>,
    pub buffer_length: usize,
    pub line_count: usize,
    pub cursor_count: usize,
    pub status_message: Option<String>,
}

impl PluginContext {
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }
}

/// Lifecycle trait for editor plugins and extensions.
pub trait Plugin {
    /// Returns the unique name of this plugin.
    fn name(&self) -> &str;

    /// Invoked once when the editor initializes.
    fn on_init(&mut self, _ctx: &mut PluginContext) {}

    /// Invoked when a buffer is opened.
    fn on_open(&mut self, _ctx: &mut PluginContext, _path: &Option<PathBuf>) {}

    /// Invoked before saving a buffer, allowing in-place text transformation (e.g. formatters).
    fn on_before_save(&mut self, _ctx: &mut PluginContext, _content: &mut String) {}

    /// Invoked immediately after a buffer is saved.
    fn on_after_save(&mut self, _ctx: &mut PluginContext) {}

    /// Invoked when a custom command is executed via the Command Palette or keybinding.
    fn on_command(&mut self, _ctx: &mut PluginContext, _command: &str) -> bool {
        false
    }
}
