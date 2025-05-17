use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};
use tempfile::tempdir;

use crate::config::{Config, read_config, write_config, get_config_path, initialize_config};
use crate::fs_utils::{set_test_home_dir, set_test_id, clear_test_id};

fn setup_test_env() -> Result<(tempfile::TempDir, PathBuf)> {
    let test_id = set_test_id();
    
    let temp_dir = tempdir()?;
    let home_path = temp_dir.path().join(format!("home_{}", test_id));
    
    fs::create_dir_all(&home_path)?;
    set_test_home_dir(Some(home_path.clone()));
    
    Ok((temp_dir, home_path))
}

fn cleanup_test_env() {
    set_test_home_dir(None);
    clear_test_id();
}

#[test]
fn test_config_read_write() -> Result<()> {
    let (_, _) = setup_test_env()?;
    
    let config = Config {
        source_dir: String::from("/path/to/dotfiles"),
    };
    
    write_config(&config)?;
    
    let read_config = read_config()?;
    assert_eq!(read_config.source_dir, "/path/to/dotfiles");
    
    // Verify it was written as YAML
    let config_path = get_config_path()?;
    let content = fs::read_to_string(config_path)?;
    assert!(content.contains("source_dir:"));
    
    cleanup_test_env();
    Ok(())
}

#[test]
fn test_initialize_config() -> Result<()> {
    let (_, _) = setup_test_env()?;
    
    initialize_config("/custom/dotfiles/path")?;
    
    let config_path = get_config_path()?;
    assert!(config_path.exists());
    
    let content = fs::read_to_string(config_path)?;
    assert!(content.contains("/custom/dotfiles/path"));
    
    cleanup_test_env();
    Ok(())
}

#[test]
fn test_default_config() -> Result<()> {
    let (_, _) = setup_test_env()?;
    
    // Don't create a config file, so read_config should return default values
    let config = read_config()?;
    
    assert_eq!(config.source_dir, ".");
    
    cleanup_test_env();
    Ok(())
}

#[test]
fn test_get_config_path() -> Result<()> {
    let (_, home_path) = setup_test_env()?;
    
    let config_path = get_config_path()?;
    let expected_path = home_path.join(".dotfiles-rustrc.yaml");
    
    assert_eq!(config_path, expected_path);
    
    cleanup_test_env();
    Ok(())
}

#[test]
fn test_migrate_json_to_yaml() -> Result<()> {
    let (_, home_path) = setup_test_env()?;
    
    println!("Test home path: {}", home_path.display());
    
    // Make sure the home directory exists
    fs::create_dir_all(&home_path)
        .with_context(|| format!("Failed to create test home directory at {}", home_path.display()))?;
    
    // Create an old-style JSON config file
    let old_config_path = home_path.join(".dotfiles-rustrc");
    let json_content = r#"{"source_dir":"/old/json/config/path"}"#;
    
    println!("Writing JSON config to: {}", old_config_path.display());
    fs::write(&old_config_path, json_content)
        .with_context(|| format!("Failed to write test JSON config to {}", old_config_path.display()))?;
    
    // Verify the old JSON file exists
    assert!(old_config_path.exists(), "Old JSON config file should exist");
    println!("Verified old JSON config exists at: {}", old_config_path.display());
    
    // Reading the config should trigger migration
    println!("Now reading config to trigger migration...");
    let config = read_config().with_context(|| "Failed to read config during migration test")?;
    
    // Verify the content was migrated correctly
    assert_eq!(config.source_dir, "/old/json/config/path", "Config source_dir should match the migrated value");
    println!("Verified migrated config has correct content");
    
    // Verify the new YAML file exists
    let new_config_path = get_config_path()?;
    println!("New config path should be: {}", new_config_path.display());
    
    if !new_config_path.exists() {
        return Err(anyhow::anyhow!("New YAML config file does not exist at {}", new_config_path.display()));
    }
    println!("Verified new YAML config exists at: {}", new_config_path.display());
    
    // Verify the old JSON file was removed
    if old_config_path.exists() {
        return Err(anyhow::anyhow!("Old JSON file still exists at {}", old_config_path.display()));
    }
    println!("Verified old JSON config was removed");
    
    // Verify the content of the new file has YAML format
    let content = fs::read_to_string(&new_config_path)
        .with_context(|| format!("Failed to read new YAML config at {}", new_config_path.display()))?;
    println!("New YAML content: {}", content);
    
    assert!(content.contains("source_dir: /old/json/config/path"), 
            "New config file should contain YAML formatted content: {}", content);
    println!("Verified new config file has YAML format");
    
    cleanup_test_env();
    Ok(())
} 