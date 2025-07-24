use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_vim_bindings")]
    pub vim_bindings: bool,
    
    #[serde(default = "default_tab_size")]
    pub tab_size: usize,
}

fn default_vim_bindings() -> bool {
    false
}

fn default_tab_size() -> usize {
    4
}

impl Default for Config {
    fn default() -> Self {
        Config {
            vim_bindings: default_vim_bindings(),
            tab_size: default_tab_size(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::config_path();
        
        if let Ok(contents) = fs::read_to_string(&config_path) {
            toml::from_str(&contents).unwrap_or_else(|e| {
                eprintln!("Error parsing config file: {}", e);
                Self::default()
            })
        } else {
            // Create default config file if it doesn't exist
            let default_config = Self::default();
            if let Err(e) = default_config.save() {
                eprintln!("Error creating default config file: {}", e);
            }
            default_config
        }
    }
    
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path();
        
        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let toml_string = toml::to_string_pretty(self)?;
        fs::write(&config_path, toml_string)?;
        
        Ok(())
    }
    
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("river");
        path.push("config.toml");
        path
    }
}