use std::fs;
use std::path::Path;
use std::collections::HashMap;
use anyhow::{anyhow, Context, Result};
use walkdir::WalkDir;
use chrono;
use std::path::PathBuf;
use std::collections::HashSet;

use crate::fs_utils::{get_home_dir, get_backup_dir, ensure_parent_dirs};
use crate::backup::{backup_file, find_backup_by_version, find_latest_backup, find_all_backup_versions};
use crate::config::read_config;
use crate::colorize;

const BLACKLIST: &[&str] = &[".git", ".dotfiles-rustrc.yaml", "README.md", "node_modules", ".DS_Store"];

pub fn install_dotfiles(dry_run: bool, force: bool, backup: bool, verbose: bool) -> Result<()> {
    let config = read_config()?;
    let source_dir = &config.source_dir;

    let home_dir = get_home_dir()?;
    let source_dir = Path::new(source_dir);
    let backup_dir = get_backup_dir()?;

    if !source_dir.exists() {
        return Err(anyhow!("Source directory '{}' does not exist", source_dir.display()));
    }

    if verbose {
        println!("{} {} to {}",
            colorize::info("Installing dotfiles from"),
            colorize::path(source_dir.display()),
            colorize::path(home_dir.strip_prefix(&get_home_dir()?)?.display()));
        if dry_run {
            println!("{}", colorize::dry_run("Dry run mode: no files will be copied"));
        }
    } else {
        println!("{}", colorize::header("Installing dotfiles..."));
    }

    for entry in WalkDir::new(source_dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let source_path = entry.path();

        if !source_path.is_file() {
            continue;
        }

        let relative_path = source_path.strip_prefix(source_dir)?;

        let should_skip = BLACKLIST.iter().any(|pattern| {
            relative_path.to_string_lossy().contains(pattern)
        });

        if should_skip {
            if verbose {
                println!("  {} {}", colorize::warning("Skipping blacklisted path:"), colorize::path(relative_path.display()));
            }
            continue;
        }

        let target_path = home_dir.join(relative_path);

        if verbose {
            println!("  {} {}", colorize::info("Processing:"), colorize::path(source_path.display()));
            println!("    {} {}", colorize::info("Relative path:"), colorize::path(relative_path.display()));
            println!("    {} {}", colorize::info("Target path:"), colorize::path(target_path.strip_prefix(&get_home_dir()?)?.display()));
        }

        if let Some(parent) = target_path.parent() {
            if !parent.exists() && !dry_run {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory {}", parent.display()))?;
            }
        }

        if verbose {
            println!("{} => {}",
                colorize::path(source_path.display()),
                colorize::path(target_path.strip_prefix(&get_home_dir()?)?.display()));
        }

        if target_path.exists() {
            // Check if the files have the same content
            let files_identical = match (fs::read(&source_path), fs::read(&target_path)) {
                (Ok(source_content), Ok(target_content)) => source_content == target_content,
                _ => false
            };

            if files_identical {
                if verbose {
                    println!("  {}", colorize::info("Skipping (files are identical)"));
                    println!("  {} {}", colorize::info("Unchanged:"), colorize::path(relative_path.display()));
                }
                continue;
            }

            if !force {
                if verbose {
                    println!("  {}", colorize::warning("Skipping (already exists but different, use --force to overwrite)"));
                } else {
                    println!("  {} {} (already exists, use --force to overwrite)",
                        colorize::warning("Skipped:"),
                        colorize::path(relative_path.display()));
                }
                continue;
            } else if backup {
                backup_file(&target_path, &backup_dir, dry_run)?;
            }
        }

        if !dry_run {
            fs::copy(source_path, &target_path)
                .with_context(|| format!("Failed to copy {} to {}", source_path.display(), target_path.display()))?;
            if verbose {
                println!("  {}", colorize::success("Copied successfully"));
            } else {
                println!("  {} {}", colorize::success("Copied:"), colorize::path(relative_path.display()));
            }
        } else {
            if verbose {
                println!("  {} {}",
                    colorize::dry_run("[Dry run] Would copy to"),
                    colorize::path(target_path.strip_prefix(&get_home_dir()?)?.display()));
            } else {
                println!("  {} {}",
                    colorize::dry_run("[Dry run] Would copy:"),
                    colorize::path(relative_path.display()));
            }
        }
    }

    if verbose {
        println!("{}", colorize::success("Dotfiles installation complete!"));
        println!("{}", colorize::info("You can now run 'restore' to revert to original files at any time."));
    } else {
        println!("{}", colorize::success("Installation complete."));
        println!("{}", colorize::info("You can now run 'restore' to revert to original files at any time."));
    }
    Ok(())
}

pub fn restore_backups(file: Option<&str>, version: Option<&str>, dry_run: bool, keep_backups: bool) -> Result<()> {
    let home_dir = get_home_dir()?;
    let backup_dir = get_backup_dir()?;
    let config = read_config()?;
    let source_dir = Path::new(&config.source_dir);

    if !source_dir.exists() {
        return Err(anyhow!("Source directory '{}' does not exist", source_dir.display()));
    }

    // Create backup directory if it doesn't exist
    if !backup_dir.exists() && !dry_run {
        fs::create_dir_all(&backup_dir)
            .with_context(|| format!("Failed to create backup directory {}", backup_dir.display()))?;
        println!("{} {}", 
            colorize::info("Created backup directory:"), 
            colorize::path(backup_dir.display()));
    }

    if let Some(file_path) = file {
        let home_file = home_dir.join(file_path);
        let source_file_path = source_dir.join(file_path);

        if let Some(ver) = version {
            // Restoring specific version
            match find_backup_by_version(file_path, ver, &backup_dir) {
                Ok(backup_file) => {
                    println!("{} {} ({} {}) to {}",
                        colorize::info("Restoring"),
                        colorize::path(backup_file.strip_prefix(&get_backup_dir()?)?.display()),
                        colorize::info("version"),
                        colorize::version(ver),
                        colorize::path(home_file.strip_prefix(&get_home_dir()?)?.display()));

                    if !dry_run {
                        ensure_parent_dirs(&home_file, dry_run)?;
                        fs::copy(&backup_file, &home_file)?;
                        println!("  {}", colorize::success("Restored successfully"));

                        if !keep_backups {
                            fs::remove_file(&backup_file)
                                .with_context(|| format!("Failed to delete backup file {}", backup_file.display()))?;
                            println!("  {}", colorize::success("Backup deleted"));
                        }
                    }
                },
                Err(_) => {
                    if source_file_path.exists() {
                        println!("{} {} {} {}", 
                            colorize::warning("No backup version"),
                            colorize::version(ver),
                            colorize::warning("found for"),
                            colorize::path(file_path));
                        println!("{} {}", 
                            colorize::info("Using source file from"),
                            colorize::path(source_file_path.display()));
                            
                        if !dry_run {
                            // Backup the existing file if it exists
                            if home_file.exists() {
                                backup_file(&home_file, &backup_dir, dry_run)?;
                                println!("  {} {}", 
                                    colorize::info("Created backup of existing file at"), 
                                    colorize::path(home_file.strip_prefix(&get_home_dir()?)?.display()));
                            }
                            
                            ensure_parent_dirs(&home_file, dry_run)?;
                            fs::copy(&source_file_path, &home_file)?;
                            println!("  {}", colorize::success("Installed from source"));
                        }
                    } else if home_file.exists() {
                        println!("{} {} {} {}",
                            colorize::error("No backup version"),
                            colorize::version(ver),
                            colorize::error("found for"),
                            colorize::path(file_path));
                        println!("{} {}? (yes/no)",
                            colorize::warning("Do you want to delete the existing file at"),
                            colorize::path(home_file.strip_prefix(&get_home_dir()?)?.display()));

                        let mut confirmation = String::new();
                        std::io::stdin().read_line(&mut confirmation)?;

                        if confirmation.trim().to_lowercase() == "yes" {
                            if !dry_run {
                                fs::remove_file(&home_file)
                                    .with_context(|| format!("Failed to delete file {}", home_file.display()))?;
                                println!("  {}", colorize::success("File deleted"));
                            } else {
                                println!("  {} {}",
                                    colorize::dry_run("[Dry run] Would delete file"),
                                    colorize::path(home_file.strip_prefix(&get_home_dir()?)?.display()));
                            }
                        } else {
                            println!("  {}", colorize::warning("Deletion canceled"));
                        }
                    } else {
                        println!("{} {} {} {}",
                            colorize::error("No backup version"),
                            colorize::version(ver),
                            colorize::error("found for"),
                            colorize::path(file_path));
                        println!("{} {}", 
                            colorize::error("Source file not found at"), 
                            colorize::path(source_file_path.display()));
                        println!("{}", colorize::info("Nothing to restore"));
                    }
                }
            }
        } else {
            // Restoring latest version
            match find_latest_backup(file_path, &backup_dir) {
                Ok(latest) => {
                    println!("{} {} from {}",
                        colorize::info("Restoring latest backup of"),
                        colorize::path(file_path),
                        colorize::path(latest.strip_prefix(&get_backup_dir()?)?.display()));

                    if !dry_run {
                        ensure_parent_dirs(&home_file, dry_run)?;
                        fs::copy(&latest, &home_file)?;
                        println!("  {}", colorize::success("Restored successfully"));

                        if !keep_backups {
                            fs::remove_file(&latest)
                                .with_context(|| format!("Failed to delete backup file {}", latest.display()))?;
                            println!("  {}", colorize::success("Backup deleted"));
                        }
                    }
                },
                Err(_) => {
                    if source_file_path.exists() {
                        println!("{} {}", 
                            colorize::warning("No backups found for"), 
                            colorize::path(file_path));
                        println!("{} {}", 
                            colorize::info("Using source file from"),
                            colorize::path(source_file_path.display()));
                            
                        if !dry_run {
                            // Backup the existing file if it exists
                            if home_file.exists() {
                                backup_file(&home_file, &backup_dir, dry_run)?;
                                println!("  {} {}", 
                                    colorize::info("Created backup of existing file at"), 
                                    colorize::path(home_file.strip_prefix(&get_home_dir()?)?.display()));
                            }
                            
                            ensure_parent_dirs(&home_file, dry_run)?;
                            fs::copy(&source_file_path, &home_file)?;
                            println!("  {}", colorize::success("Installed from source"));
                        }
                    } else if home_file.exists() {
                        println!("{} {}", colorize::error("No backups found for"), colorize::path(file_path));
                        println!("{} {}? (yes/no)",
                            colorize::warning("Do you want to delete the existing file at"),
                            colorize::path(home_file.strip_prefix(&get_home_dir()?)?.display()));

                        let mut confirmation = String::new();
                        std::io::stdin().read_line(&mut confirmation)?;

                        if confirmation.trim().to_lowercase() == "yes" {
                            if !dry_run {
                                fs::remove_file(&home_file)
                                    .with_context(|| format!("Failed to delete file {}", home_file.display()))?;
                                println!("  {}", colorize::success("File deleted"));
                            } else {
                                println!("  {} {}",
                                    colorize::dry_run("[Dry run] Would delete file"),
                                    colorize::path(home_file.strip_prefix(&get_home_dir()?)?.display()));
                            }
                        } else {
                            println!("  {}", colorize::warning("Deletion canceled"));
                        }
                    } else {
                        println!("{} {}", 
                            colorize::warning("No backups found for"), 
                            colorize::path(file_path));
                        println!("{} {}", 
                            colorize::warning("Source file not found at"), 
                            colorize::path(source_file_path.display()));
                        println!("{}", colorize::info("Nothing to restore"));
                    }
                }
            }
        }
    } else {
        restore_all_latest_backups(&backup_dir, &home_dir, source_dir, dry_run, keep_backups)?;
    }

    Ok(())
}

fn restore_all_latest_backups(backup_dir: &Path, home_dir: &Path, source_dir: &Path, dry_run: bool, keep_backups: bool) -> Result<()> {
    let mut file_map = HashMap::new();
    let mut have_backups = false;

    // First check if backup directory exists
    if backup_dir.exists() {
        for entry in fs::read_dir(backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if let Some(backup_name) = path.file_name() {
                let backup_name = backup_name.to_string_lossy();

                if let Some(pos) = backup_name.rfind('.') {
                    let (filename, ver_str) = backup_name.split_at(pos);
                    let ver = &ver_str[1..];

                    if let Ok(timestamp) = ver.parse::<u64>() {
                        let entry = file_map.entry(filename.to_string()).or_insert(Vec::new());
                        entry.push((timestamp, path.clone()));
                    }
                }
            }
        }

        if !file_map.is_empty() {
            have_backups = true;
        }
    }

    if have_backups {
        println!("{}", colorize::header("Restoring the latest backup for all files:"));
        let mut restored_count = 0;
        let mut deleted_count = 0;

        // Create a copy of file_map keys for checking later
        let backup_files: HashSet<String> = file_map.keys().cloned().collect();

        for (filename, versions) in &file_map {
            if let Some((timestamp, backup_path)) = versions.iter().max_by_key(|(ts, _)| *ts) {
                let home_file: PathBuf = home_dir.join(filename);
                let date_time: String = chrono::DateTime::<chrono::Utc>::from_timestamp(*timestamp as i64, 0)
                    .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| timestamp.to_string());

                println!("  {} {} ({} {}) to {}",
                         colorize::info("Restoring"),
                         colorize::path(backup_path.file_name().unwrap_or_default().to_string_lossy()),
                         colorize::info("from"),
                         colorize::version(date_time),
                         colorize::path(home_file.strip_prefix(home_dir).unwrap_or(&home_file).display()));

                if !dry_run {
                    ensure_parent_dirs(&home_file, dry_run)?;
                    fs::copy(backup_path, &home_file)
                        .with_context(|| format!("Failed to restore {} to {}",
                                                 backup_path.display(),
                                                 home_file.display()))?;
                    restored_count += 1;

                    if !keep_backups {
                        fs::remove_file(backup_path)
                            .with_context(|| format!("Failed to delete backup file {}", backup_path.display()))?;
                        deleted_count += 1;
                    }
                }
            }
        }

        if dry_run {
            println!("{}", colorize::dry_run("Dry run - no files were actually restored"));
        } else {
            println!("{} {} {}",
                colorize::success("Successfully restored"),
                colorize::highlight(restored_count),
                colorize::success("files from backups"));
            if !keep_backups && deleted_count > 0 {
                println!("{} {} {}",
                    colorize::info("Deleted"),
                    colorize::highlight(deleted_count),
                    colorize::info("backup files"));
            }
        }
    } else {
        println!("{}", colorize::warning("No backups found to restore"));
        
        if source_dir.exists() {
            println!("{}", colorize::info("Installing from source files instead..."));
            let mut installed_count = 0;
            
            for entry in WalkDir::new(source_dir)
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let source_path = entry.path();

                if !source_path.is_file() {
                    continue;
                }

                let relative_path = source_path.strip_prefix(source_dir)?;

                let should_skip = BLACKLIST.iter().any(|pattern| {
                    relative_path.to_string_lossy().contains(pattern)
                });

                if should_skip {
                    continue;
                }

                let target_path = home_dir.join(relative_path);

                println!("  {} {} => {}",
                    colorize::info("Installing"),
                    colorize::path(source_path.display()),
                    colorize::path(target_path.strip_prefix(home_dir).unwrap_or(&target_path).display()));

                if !dry_run {
                    // Backup the existing file if it exists
                    if target_path.exists() {
                        backup_file(&target_path, backup_dir, dry_run)?;
                        println!("    {} {}", 
                            colorize::info("Created backup of existing file at"), 
                            colorize::path(target_path.strip_prefix(home_dir).unwrap_or(&target_path).display()));
                    }
                    
                    ensure_parent_dirs(&target_path, dry_run)?;
                    fs::copy(source_path, &target_path)
                        .with_context(|| format!("Failed to copy {} to {}", 
                            source_path.display(), 
                            target_path.display()))?;
                    installed_count += 1;
                }
            }
            
            if dry_run {
                println!("{}", colorize::dry_run("Dry run - no files were actually installed"));
            } else {
                println!("{} {} {}",
                    colorize::success("Successfully installed"),
                    colorize::highlight(installed_count),
                    colorize::success("files from source"));
            }
        } else {
            println!("{} {}", 
                colorize::error("Source directory not found at"), 
                colorize::path(source_dir.display()));
            println!("{}", colorize::error("Nothing to restore or install"));
        }
        
        return Ok(());
    }

    // Find and remove files in home directory that were installed but have no backups
    if source_dir.exists() {
        let mut removed_count = 0;
        let backup_files: HashSet<String> = file_map.keys().cloned().collect();

        println!("{}", colorize::header("Removing files that were installed but have no backups:"));

        for entry in WalkDir::new(source_dir)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let source_path = entry.path();

            if !source_path.is_file() {
                continue;
            }

            let relative_path = source_path.strip_prefix(source_dir)?;

            let should_skip = BLACKLIST.iter().any(|pattern| {
                relative_path.to_string_lossy().contains(pattern)
            });

            if should_skip {
                continue;
            }

            let target_path = home_dir.join(relative_path);
            let target_filename = if let Some(filename) = relative_path.to_str() {
                filename.to_string()
            } else {
                continue;
            };

            // Check if there's no backup for this file
            if !backup_files.contains(&target_filename) && target_path.exists() {
                println!("  {} {} (no backup found)",
                         colorize::warning("Removing"),
                         colorize::path(target_path.strip_prefix(home_dir).unwrap_or(&target_path).display()));

                if !dry_run {
                    fs::remove_file(&target_path)
                        .with_context(|| format!("Failed to remove file {}", target_path.display()))?;
                    removed_count += 1;
                }
            }
        }

        if removed_count > 0 {
            println!("{} {} {}",
                colorize::info("Removed"),
                colorize::highlight(removed_count),
                colorize::info("files with no backups"));
        } else if !dry_run {
            println!("{}", colorize::info("No files needed to be removed"));
        }
    }

    Ok(())
}

pub fn list_backups(file: Option<&str>) -> Result<()> {
    let backup_dir: PathBuf = get_backup_dir()?;

    if !backup_dir.exists() {
        println!("{}", colorize::warning("No backups found"));
        return Ok(());
    }

    if let Some(file_path) = file {
        let versions = find_all_backup_versions(file_path, &backup_dir)?;

        if versions.is_empty() {
            println!("{} {}", colorize::warning("No backups found for"), colorize::path(file_path));
        } else {
            println!("{} {}:", colorize::header("Backup versions for"), colorize::path(file_path));
            for (version, path) in versions {
                let date_time = chrono::DateTime::<chrono::Utc>::from_timestamp(version as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| version.to_string());

                println!("  {} - {} ({})",
                    colorize::version(version),
                    colorize::path(path.strip_prefix(&get_backup_dir()?)?.display()),
                    colorize::info(date_time));
            }
        }
    } else {
        println!("{}", colorize::header("All backup files:"));
        let mut found = false;

        for entry in WalkDir::new(&backup_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Some(_) = path.file_name() {
                println!("  {}", colorize::path(path.strip_prefix(&backup_dir).unwrap_or(path).display()));
                found = true;
            }
        }

        if !found {
            println!("{}", colorize::warning("No backups found"));
        }
    }

    Ok(())
}

pub fn clear_backups(force: bool) -> Result<()> {
    let backup_dir = get_backup_dir()?;
    let home_dir = get_home_dir()?;

    if !backup_dir.exists() {
        let display_path = if backup_dir.starts_with(&home_dir) {
            format!("~/{}", backup_dir.strip_prefix(&home_dir).unwrap_or(&backup_dir).display())
        } else {
            backup_dir.display().to_string()
        };

        println!("{} {}", colorize::warning("No backups directory found at"), colorize::path(display_path));
        return Ok(());
    }

    if !force {
        let display_path = if backup_dir.starts_with(&home_dir) {
            format!("~/{}", backup_dir.strip_prefix(&home_dir).unwrap_or(&backup_dir).display())
        } else {
            backup_dir.display().to_string()
        };

        println!("{} {}",
            colorize::warning("Warning: This will permanently delete all backup files in"),
            colorize::path(display_path));
        println!("{}", colorize::warning("Are you sure you want to continue? (yes/no)"));

        let mut confirmation = String::new();
        std::io::stdin().read_line(&mut confirmation)?;
        let confirmation = confirmation.trim().to_lowercase();

        if confirmation != "yes" {
            println!("{}", colorize::warning("Backup clearing cancelled."));
            return Ok(());
        }
    }

    let display_path = if backup_dir.starts_with(&home_dir) {
        format!("~/{}", backup_dir.strip_prefix(&home_dir).unwrap_or(&backup_dir).display())
    } else {
        backup_dir.display().to_string()
    };

    println!("{} {}...", colorize::info("Clearing backups in"), colorize::path(display_path));
    fs::remove_dir_all(&backup_dir)
        .with_context(|| format!("Failed to remove backup directory {}", backup_dir.display()))?;

    println!("{}", colorize::success("All backups cleared."));

    Ok(())
}