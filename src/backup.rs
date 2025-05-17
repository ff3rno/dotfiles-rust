use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{anyhow, Context, Result};
use crate::colorize;

pub fn backup_file(file_path: &Path, backup_dir: &Path, dry_run: bool) -> Result<()> {
    if !backup_dir.exists() && !dry_run {
        return Err(anyhow!("Backup directory {} does not exist", backup_dir.display()));
    }
    
    let filename = file_path.file_name()
        .ok_or_else(|| anyhow!("Could not get filename"))?
        .to_string_lossy();
    
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let backup_filename = format!("{}.{}", filename, timestamp);
    let backup_path = backup_dir.join(&backup_filename);
    
    if !dry_run {
        if !file_path.exists() {
            return Err(anyhow!("Source file {} does not exist", file_path.display()));
        }
        
        fs::copy(file_path, &backup_path)
            .with_context(|| format!("Failed to create backup at {}", backup_path.display()))?;
            
    } else {
        println!("  {} {}", 
            colorize::dry_run("[Dry run] Would create backup at"), 
            colorize::path(backup_path.display()));
    }
    
    Ok(())
}

pub fn find_backup_by_version(file_path: &str, version: &str, backup_dir: &Path) -> Result<PathBuf> {
    let filename = Path::new(file_path).file_name()
        .ok_or_else(|| anyhow!("Invalid file path"))?
        .to_string_lossy();
    
    let backup_path = backup_dir.join(format!("{}.{}", filename, version));
    
    if backup_path.exists() {
        Ok(backup_path)
    } else {
        Err(anyhow!("Backup version {} not found for {}", version, file_path))
    }
}

pub fn find_latest_backup(file_path: &str, backup_dir: &Path) -> Result<PathBuf> {
    let versions = find_all_backup_versions(file_path, backup_dir)?;
    
    if versions.is_empty() {
        return Err(anyhow!("No backups found for {}", file_path));
    }
    
    let (_, latest_path) = versions.into_iter()
        .max_by_key(|(ver, _)| *ver)
        .unwrap();
    
    Ok(latest_path)
}

pub fn find_all_backup_versions(file_path: &str, backup_dir: &Path) -> Result<Vec<(u64, PathBuf)>> {
    let filename = Path::new(file_path).file_name()
        .ok_or_else(|| anyhow!("Invalid file path"))?
        .to_string_lossy();
    
    let mut versions = Vec::new();
    
    if !backup_dir.exists() {
        return Ok(versions);
    }
    
    for entry in fs::read_dir(backup_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if !path.is_file() {
            continue;
        }
        
        if let Some(backup_name) = path.file_name() {
            let backup_name = backup_name.to_string_lossy();
            
            if let Some(pos) = backup_name.rfind('.') {
                let (name, ver) = backup_name.split_at(pos);
                
                if name == filename {
                    let ver = &ver[1..];  
                    if let Ok(timestamp) = ver.parse::<u64>() {
                        versions.push((timestamp, path));
                    }
                }
            }
        }
    }
    
    versions.sort_by_key(|(timestamp, _)| *timestamp);
    
    Ok(versions)
} 