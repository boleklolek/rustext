//! Plugin registry and built-in plugins.
//! Zero third-party dependencies, purely standard library Rust.

use std::path::PathBuf;

use super::traits::{Plugin, PluginContext};

/// Manages registration and lifecycle hooks of active plugins.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Registers a new plugin instance.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    /// Triggers the initialization hook on all plugins.
    pub fn trigger_init(&mut self, ctx: &mut PluginContext) {
        for plugin in &mut self.plugins {
            plugin.on_init(ctx);
        }
    }

    /// Triggers buffer open hook.
    pub fn trigger_open(&mut self, ctx: &mut PluginContext, path: &Option<PathBuf>) {
        for plugin in &mut self.plugins {
            plugin.on_open(ctx, path);
        }
    }

    /// Triggers before save hook.
    pub fn trigger_before_save(&mut self, ctx: &mut PluginContext, content: &mut String) {
        for plugin in &mut self.plugins {
            plugin.on_before_save(ctx, content);
        }
    }

    /// Triggers after save hook.
    pub fn trigger_after_save(&mut self, ctx: &mut PluginContext) {
        for plugin in &mut self.plugins {
            plugin.on_after_save(ctx);
        }
    }

    /// Dispatches a command to all plugins. Stops when the first plugin handles it.
    pub fn trigger_command(&mut self, ctx: &mut PluginContext, command: &str) -> bool {
        for plugin in &mut self.plugins {
            if plugin.on_command(ctx, command) {
                return true;
            }
        }
        false
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------- Built-in Sample Plugins ----------------

/// Plugin that cleans trailing whitespace on save.
pub struct TrailingWhitespacePlugin;

impl Plugin for TrailingWhitespacePlugin {
    fn name(&self) -> &str {
        "trailing_whitespace_cleaner"
    }

    fn on_before_save(&mut self, ctx: &mut PluginContext, content: &mut String) {
        let mut cleaned = String::with_capacity(content.len());
        for line in content.lines() {
            cleaned.push_str(line.trim_end());
            cleaned.push('\n');
        }
        *content = cleaned;
        ctx.set_status("Cleaned trailing whitespace before save");
    }
}

/// Plugin that counts words and characters on demand.
pub struct WordCountPlugin;

impl Plugin for WordCountPlugin {
    fn name(&self) -> &str {
        "word_counter"
    }

    fn on_command(&mut self, ctx: &mut PluginContext, command: &str) -> bool {
        if command == "stats" || command == "word_count" {
            ctx.set_status(format!(
                "Document stats: {} chars, {} lines, {} cursors",
                ctx.buffer_length, ctx.line_count, ctx.cursor_count
            ));
            true
        } else {
            false
        }
    }
}
