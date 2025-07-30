// This module handles configuration management for the River editor
// It demonstrates several important Rust concepts:
// - Serialization/Deserialization with Serde
// - Default trait implementation
// - Error handling with Result
// - File I/O operations

use serde::{Deserialize, Serialize}; // Traits for automatic serialization
use std::fs; // File system operations
use std::path::PathBuf; // Owned path type (like String vs &str)

// Configuration struct that maps to TOML file format
// 'pub' makes this struct visible outside the module
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    // #[serde(default = "function")] specifies a function to call
    // when this field is missing during deserialization
    #[serde(default = "default_vim_bindings")]
    pub vim_bindings: bool,
    
    #[serde(default = "default_tab_size")]
    pub tab_size: usize, // Platform-specific pointer size
    
    #[serde(default = "default_daily_notes_dir")]
    pub daily_notes_dir: String, // Heap-allocated string
    
    #[serde(default = "default_typing_timeout_seconds")]
    pub typing_timeout_seconds: u64, // 64-bit unsigned integer
    
    #[serde(default = "default_show_prompts")]
    pub show_prompts: bool,
    
    #[serde(default = "default_prompt_style")]
    pub prompt_style: String, // "ghost" or "none" or "command_only"
    
    #[serde(default = "default_use_ai_prompts")]
    pub use_ai_prompts: bool,
}

// These functions provide default values for config fields
// They're called by serde when a field is missing
fn default_vim_bindings() -> bool {
    false // Rust doesn't require 'return' for last expression
}

fn default_tab_size() -> usize {
    4
}

fn default_daily_notes_dir() -> String {
    // 'if let' combines pattern matching with conditional logic
    // It's like: "if this matches Some(value), bind it to 'home'"
    if let Some(home) = dirs::home_dir() {
        // PathBuf::join creates a new path
        // to_string_lossy() converts Path to String, replacing invalid UTF-8
        // Final .to_string() converts from Cow<str> to owned String
        home.join("Documents/DailyNotes").to_string_lossy().to_string()
    } else {
        // String literal with .to_string() creates an owned String
        "./DailyNotes".to_string()
    }
}

fn default_typing_timeout_seconds() -> u64 {
    180 // 3 minutes - integer literal
}

fn default_show_prompts() -> bool {
    true
}

fn default_prompt_style() -> String {
    "ghost".to_string()
}

fn default_use_ai_prompts() -> bool {
    true
}

// Implementing the Default trait allows Config::default() to be called
// This is useful for creating instances with sensible defaults
impl Default for Config {
    fn default() -> Self {
        // Struct literal syntax - field names match variable names
        Config {
            vim_bindings: default_vim_bindings(),
            tab_size: default_tab_size(),
            daily_notes_dir: default_daily_notes_dir(),
            typing_timeout_seconds: default_typing_timeout_seconds(),
            show_prompts: default_show_prompts(),
            prompt_style: default_prompt_style(),
            use_ai_prompts: default_use_ai_prompts(),
        }
    }
}

// Methods specific to Config (not from a trait)
impl Config {
    // Associated function (no self parameter) - called as Config::load()
    pub fn load() -> Self {
        // Self::config_path() calls another associated function
        let config_path = Self::config_path();
        
        // Try to read the config file
        // Ok(contents) means success, Err(_) means failure
        if let Ok(contents) = fs::read_to_string(&config_path) {
            // Parse TOML into Config struct
            // unwrap_or_else takes a closure |e| { ... } that runs on error
            // Closures are anonymous functions that can capture variables
            let mut config: Config = toml::from_str(&contents).unwrap_or_else(|e| {
                // eprintln! prints to stderr (error output)
                eprintln!("Error parsing config file: {}", e);
                Self::default() // Return default config on parse error
            });
            
            // Expand tilde (~) to home directory path
            // This is a common Unix convention
            if config.daily_notes_dir.starts_with("~") {
                if let Some(home) = dirs::home_dir() {
                    // replacen replaces first N occurrences (1 in this case)
                    // & borrows the string instead of moving it
                    config.daily_notes_dir = config.daily_notes_dir.replacen("~", &home.to_string_lossy(), 1);
                }
            }
            
            config
        } else {
            // Create default config file if it doesn't exist
            let default_config = Self::default();
            // Pattern match on Result - we only care about errors here
            if let Err(e) = default_config.save() {
                eprintln!("Error creating default config file: {}", e);
            }
            default_config // Return the config (moved ownership)
        }
    }
    
    // Save config to file
    // &self - immutable borrow (we only read the config)
    // Result<(), Box<dyn Error>> - can return any error type
    // Box<dyn Error> is a trait object - dynamic dispatch
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path();
        
        // Create config directory if it doesn't exist
        // Option::parent() returns Some(parent_path) or None
        if let Some(parent) = config_path.parent() {
            // ? operator converts the error type and returns early on error
            fs::create_dir_all(parent)?;
        }
        
        // Serialize self to pretty-printed TOML
        let toml_string = toml::to_string_pretty(self)?;
        // Write to file - takes a reference to path and content
        fs::write(&config_path, toml_string)?;
        
        Ok(()) // Success - return unit type wrapped in Ok
    }
    
    // Private associated function (no 'pub')
    // Returns the platform-specific config file path
    fn config_path() -> PathBuf {
        // dirs::config_dir() returns:
        // - Linux: ~/.config
        // - macOS: ~/Library/Application Support
        // - Windows: %APPDATA%
        // || PathBuf::from(".") is a closure that returns current dir as fallback
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("river");      // Add subdirectory
        path.push("config.toml"); // Add filename
        path // Return the PathBuf (implicit return)
    }
}