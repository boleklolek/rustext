//! Dynamic extension and plugin manager for Rustext.
//! Discovers, loads, toggles, and executes external extensions and manifest-based tools.
//! Zero third-party dependencies, purely standard library Rust.

use std::fs;
use std::path::{Path, PathBuf};

use crate::editor::keymap::EditorAction;
use crate::ui::prompt::CommandItem;
use super::process::ProcessPluginRunner;

/// Command exposed by an extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionCommand {
    pub id: String,
    pub label: String,
    pub shortcut: String,
}

/// Metadata and runtime state for an extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub enabled: bool,
    pub commands: Vec<ExtensionCommand>,
    pub exec: Option<String>,
    pub args: Vec<String>,
    pub on_save: bool,
    pub dir_path: PathBuf,
}

impl ExtensionInfo {
    /// Parses an extension manifest (`extension.conf`) from key-value pairs.
    pub fn from_conf_str(content: &str, dir_path: PathBuf) -> Self {
        let mut id = String::new();
        let mut name = String::new();
        let mut version = "0.1.0".to_string();
        let mut author = "Autor desconocido".to_string();
        let mut description = String::new();
        let mut enabled = true;
        let mut commands = Vec::new();
        let mut exec = None;
        let mut args = Vec::new();
        let mut on_save = false;

        let mut current_section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = trimmed[1..trimmed.len() - 1].trim().to_lowercase();
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                let key = k.trim();
                let val = v.trim();

                if current_section == "commands" {
                    let (label, shortcut) = if let Some((lbl, sc)) = val.split_once('|') {
                        (lbl.trim().to_string(), sc.trim().to_string())
                    } else {
                        (val.to_string(), String::new())
                    };
                    commands.push(ExtensionCommand {
                        id: key.to_string(),
                        label,
                        shortcut,
                    });
                    continue;
                }

                match key.to_lowercase().as_str() {
                    "id" => id = val.to_string(),
                    "name" => name = val.to_string(),
                    "version" => version = val.to_string(),
                    "author" => author = val.to_string(),
                    "description" => description = val.to_string(),
                    "enabled" => enabled = val.eq_ignore_ascii_case("true") || val == "1",
                    "exec" => {
                        if !val.is_empty() {
                            exec = Some(val.to_string());
                        }
                    }
                    "args" => {
                        args = val.split_whitespace().map(|s| s.to_string()).collect();
                    }
                    "on_save" => on_save = val.eq_ignore_ascii_case("true") || val == "1",
                    "commands" => {
                        for item in val.split(',') {
                            let item_trim = item.trim();
                            if item_trim.is_empty() {
                                continue;
                            }
                            if let Some((cid, clabel)) = item_trim.split_once(':') {
                                commands.push(ExtensionCommand {
                                    id: cid.trim().to_string(),
                                    label: clabel.trim().to_string(),
                                    shortcut: String::new(),
                                });
                            } else {
                                commands.push(ExtensionCommand {
                                    id: item_trim.to_string(),
                                    label: item_trim.to_string(),
                                    shortcut: String::new(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if id.is_empty() {
            id = dir_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unnamed_ext".to_string());
        }
        if name.is_empty() {
            name = id.clone();
        }

        Self {
            id,
            name,
            version,
            author,
            description,
            enabled,
            commands,
            exec,
            args,
            on_save,
            dir_path,
        }
    }

    /// Persists the enabled/disabled state to disk in `extension.conf`.
    pub fn save_enabled_state(&self) -> std::io::Result<()> {
        let conf_path = self.dir_path.join("extension.conf");
        if conf_path.exists() {
            if let Ok(content) = fs::read_to_string(&conf_path) {
                let mut lines = Vec::new();
                let mut found_enabled = false;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.to_lowercase().starts_with("enabled") {
                        lines.push(format!("enabled = {}", self.enabled));
                        found_enabled = true;
                    } else {
                        lines.push(line.to_string());
                    }
                }
                if !found_enabled {
                    lines.push(format!("enabled = {}", self.enabled));
                }
                fs::write(&conf_path, lines.join("\n") + "\n")?;
            }
        }
        Ok(())
    }

    /// Executes the extension logic on `input_text`.
    pub fn execute(&self, _cmd_id: &str, input_text: &str) -> Result<String, String> {
        if !self.enabled {
            return Err("La extensión está desactivada".to_string());
        }

        // 1. If explicit exec command is provided
        if let Some(cmd) = &self.exec {
            let arg_refs: Vec<&str> = self.args.iter().map(|s| s.as_str()).collect();
            return ProcessPluginRunner::pipe_through_command(cmd, &arg_refs, input_text);
        }

        // 2. Check for script files in the extension folder
        let script_candidates = ["run.sh", "run.py", "main.py", "run.bat", "run.cmd", "extension.sh"];
        for candidate in &script_candidates {
            let script_path = self.dir_path.join(candidate);
            if script_path.exists() {
                let (cmd, args) = if candidate.ends_with(".py") {
                    ("python3", vec![script_path.to_str().unwrap()])
                } else if candidate.ends_with(".sh") {
                    ("sh", vec![script_path.to_str().unwrap()])
                } else {
                    (script_path.to_str().unwrap(), vec![])
                };
                return ProcessPluginRunner::pipe_through_command(cmd, &args, input_text);
            }
        }

        Err(format!(
            "La extensión '{}' no define un comando ni script ejecutable",
            self.name
        ))
    }
}

/// Central manager for discovering, tracking, and triggering extensions.
#[derive(Debug, Clone)]
pub struct ExtensionManager {
    pub extensions: Vec<ExtensionInfo>,
    pub base_dirs: Vec<PathBuf>,
}

impl ExtensionManager {
    /// Creates a new extension manager and detects extension directories.
    pub fn new() -> Self {
        let mut dirs = Vec::new();

        // 1. User config directory: ~/.rustext/extensions or %USERPROFILE%\.rustext\extensions
        let home_dir = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from);

        if let Some(home) = home_dir {
            dirs.push(home.join(".rustext").join("extensions"));
        }

        // 2. Local workspace extensions: ./.rustext/extensions
        if let Ok(cur) = std::env::current_dir() {
            let local_dir = cur.join(".rustext").join("extensions");
            if !dirs.contains(&local_dir) {
                dirs.push(local_dir);
            }
        }

        let mut manager = Self {
            extensions: Vec::new(),
            base_dirs: dirs,
        };

        manager.load_extensions();
        manager
    }

    /// Primary directory where user extensions are stored.
    pub fn default_extensions_dir(&self) -> PathBuf {
        self.base_dirs
            .first()
            .cloned()
            .unwrap_or_else(|| PathBuf::from("./.rustext/extensions"))
    }

    /// Loads all extensions from all configured base directories.
    pub fn load_extensions(&mut self) {
        self.extensions.clear();

        let default_dir = self.default_extensions_dir();
        // Ensure default extensions directory exists
        if !default_dir.exists() {
            let _ = fs::create_dir_all(&default_dir);
        }

        // Check if default directory is empty; if so, create initial sample extensions
        if let Ok(entries) = fs::read_dir(&default_dir) {
            if entries.count() == 0 {
                let _ = self.scaffold_default_samples(&default_dir);
            }
        }

        for base_dir in &self.base_dirs {
            if !base_dir.exists() || !base_dir.is_dir() {
                continue;
            }

            if let Ok(read_dir) = fs::read_dir(base_dir) {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let conf_file = path.join("extension.conf");
                        if conf_file.exists() {
                            if let Ok(content) = fs::read_to_string(&conf_file) {
                                let info = ExtensionInfo::from_conf_str(&content, path);
                                // Prevent duplicate IDs
                                if !self.extensions.iter().any(|e| e.id == info.id) {
                                    self.extensions.push(info);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort alphabetically by name
        self.extensions
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    /// Reloads all extensions from disk.
    pub fn reload(&mut self) {
        self.load_extensions();
    }

    /// Toggles the enabled state of an extension by index and saves it.
    pub fn toggle_extension(&mut self, index: usize) -> bool {
        if index < self.extensions.len() {
            self.extensions[index].enabled = !self.extensions[index].enabled;
            let _ = self.extensions[index].save_enabled_state();
            self.extensions[index].enabled
        } else {
            false
        }
    }

    /// Returns all commands from currently enabled extensions for Command Palette integration.
    pub fn get_active_commands(&self) -> Vec<CommandItem> {
        let mut commands = Vec::new();

        for ext in &self.extensions {
            if !ext.enabled {
                continue;
            }

            for cmd in &ext.commands {
                // Prepend extension badge for clarity in Command Palette
                let full_name = format!("[Ext] {}", cmd.label);
                // Leaked static string to satisfy CommandItem lifetime requirements
                let leaked_name: &'static str = Box::leak(full_name.into_boxed_str());
                let leaked_cmd_id: &'static str = Box::leak(cmd.id.clone().into_boxed_str());

                commands.push(CommandItem {
                    name: leaked_name,
                    shortcut: "",
                    action: EditorAction::ExecuteExtensionCommand(leaked_cmd_id.to_string()),
                });
            }
        }

        commands
    }

    /// Executes an extension command on `input_text`.
    pub fn execute_command(&self, cmd_id: &str, input_text: &str) -> Option<Result<String, String>> {
        for ext in &self.extensions {
            if !ext.enabled {
                continue;
            }
            if ext.commands.iter().any(|c| c.id == cmd_id) {
                return Some(ext.execute(cmd_id, input_text));
            }
        }
        None
    }

    /// Executes all enabled extensions configured to run `on_save`.
    pub fn trigger_on_save(&self, content: &str) -> Option<String> {
        let mut current_content = content.to_string();
        let mut modified = false;

        for ext in &self.extensions {
            if ext.enabled && ext.on_save {
                if let Ok(new_content) = ext.execute("on_save", &current_content) {
                    if new_content != current_content {
                        current_content = new_content;
                        modified = true;
                    }
                }
            }
        }

        if modified {
            Some(current_content)
        } else {
            None
        }
    }

    /// Scaffolds a user-requested sample extension in the default extensions folder.
    pub fn create_sample_extension(&mut self, folder_name: &str) -> Result<PathBuf, String> {
        let target_dir = self.default_extensions_dir().join(folder_name);
        if target_dir.exists() {
            return Err("Ya existe una extensión con ese nombre".to_string());
        }

        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Error creando carpeta: {}", e))?;

        let conf_content = format!(
            r#"# Manifiesto de extensión para Rustext
id = {id}
name = {name}
version = 1.0.0
author = Usuario
description = Extensión personalizada para transformar texto
enabled = true
commands = {id}_run:Ejecutar {name}
exec = tr
args = [:lower:] [:upper:]
on_save = false
"#,
            id = folder_name,
            name = folder_name.replace('_', " ")
        );

        fs::write(target_dir.join("extension.conf"), conf_content)
            .map_err(|e| format!("Error escribiendo extension.conf: {}", e))?;

        self.reload();
        Ok(target_dir)
    }

    /// Creates initial built-in sample extensions if user directory is empty.
    fn scaffold_default_samples(&self, base_dir: &Path) -> std::io::Result<()> {
        // 1. Uppercase converter
        let dir_upper = base_dir.join("mayusculas");
        fs::create_dir_all(&dir_upper)?;
        let conf_upper = r#"id = mayusculas
name = Convertir a Mayúsculas
version = 1.0.0
author = Rustext
description = Convierte el texto seleccionado a mayúsculas
enabled = true
commands = mayusculas:Convertir a Mayúsculas
exec = tr
args = [:lower:] [:upper:]
on_save = false
"#;
        fs::write(dir_upper.join("extension.conf"), conf_upper)?;

        // 2. Whitespace cleaner
        let dir_trim = base_dir.join("limpiar_espacios");
        fs::create_dir_all(&dir_trim)?;
        let conf_trim = r#"id = limpiar_espacios
name = Limpiador de Espacios
version = 1.0.0
author = Rustext
description = Limpia espacios en blanco redundantes
enabled = true
commands = limpiar_espacios:Limpiar Espacios al Final
exec = sed
args = s/[[:space:]]*$//
on_save = false
"#;
        fs::write(dir_trim.join("extension.conf"), conf_trim)?;

        Ok(())
    }
}
