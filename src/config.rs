use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::fs_utils::get_home_dir;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub source_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source_dir: String::from("."),
        }
    }
}

pub fn get_config_path() -> Result<PathBuf> {
    let home_dir = get_home_dir()?;
    Ok(home_dir.join(".dotfiles-rustrc.yaml"))
}

pub fn read_config() -> Result<Config> {
    let config_path = get_config_path()?;
    
    if !config_path.exists() {
        // For backward compatibility, try reading the old JSON config file
        let old_config_path = get_home_dir()?.join(".dotfiles-rustrc");
        
        if old_config_path.exists() {
            let config_content = fs::read_to_string(&old_config_path)
                .with_context(|| format!("Failed to read old config file at {}", old_config_path.display()))?;
            
            match serde_json::from_str::<Config>(&config_content) {
                Ok(config) => {
                    println!("Converting old JSON config to YAML format...");
                    // Write the config in the new YAML format
                    write_config(&config)?;
                    println!("Old config file has been converted to YAML format at {}", config_path.display());
                    // Remove the old config file
                    let _ = fs::remove_file(&old_config_path);
                    return Ok(config);
                },
                Err(_) => {
                    // If we can't parse the old config, just use the default
                    return Ok(Config::default());
                }
            }
        }
        
        return Ok(Config::default());
    }
    
    let config_content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file at {}", config_path.display()))?;
    
    let config: Config = serde_yaml::from_str(&config_content)
        .with_context(|| format!("Failed to parse YAML config file at {}", config_path.display()))?;
    
    Ok(config)
}

pub fn write_config(config: &Config) -> Result<()> {
    let config_path = get_config_path()?;
    
    // Ensure the parent directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    
    let config_yaml = serde_yaml::to_string(config)
        .with_context(|| "Failed to serialize config to YAML")?;
    
    fs::write(&config_path, config_yaml)
        .with_context(|| format!("Failed to write config file at {}", config_path.display()))?;
    
    Ok(())
}

pub fn initialize_config(source_dir: &str) -> Result<()> {
    let config = Config {
        source_dir: source_dir.to_string(),
    };
    
    write_config(&config)
} 