use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use walkdir::WalkDir;
use chrono;
use std::path::PathBuf;

use crate::fs_utils::{get_home_dir, get_backup_dir};
use crate::backup::{backup_file, find_latest_backup, find_all_backup_versions};
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

pub fn status_dotfiles(verbose: bool) -> Result<()> {
    let config = read_config()?;
    let source_dir = Path::new(&config.source_dir);
    let home_dir = get_home_dir()?;

    if !source_dir.exists() {
        return Err(anyhow!("Source directory '{}' does not exist", source_dir.display()));
    }

    println!("{} {}", 
        colorize::header("Dotfiles Status"), 
        colorize::info(format!("(source: {})", source_dir.display()))
    );

    let mut total_count = 0;
    let mut installed_count = 0;
    let mut modified_count = 0;
    let mut missing_count = 0;

    for entry in fs::read_dir(source_dir)?
        .filter_map(|e| e.ok())
    {
        let source_path = entry.path();
        let relative_path = source_path.strip_prefix(source_dir)?;

        let should_skip = BLACKLIST.iter().any(|pattern| {
            relative_path.to_string_lossy().contains(pattern)
        });

        if should_skip {
            continue;
        }

        total_count += 1;
        let target_path = home_dir.join(relative_path);

        if source_path.is_file() {
            if !target_path.exists() {
                println!("  {} {} {}", 
                    colorize::error("✗"), 
                    colorize::path(relative_path.display()),
                    colorize::error("Not installed")
                );
                missing_count += 1;
            } else {
                let files_identical = match (fs::read(&source_path), fs::read(&target_path)) {
                    (Ok(source_content), Ok(target_content)) => source_content == target_content,
                    _ => false
                };

                if files_identical {
                    println!("  {} {} {}", 
                        colorize::success("✓"), 
                        colorize::path(relative_path.display()),
                        colorize::success("Installed")
                    );
                    installed_count += 1;
                } else {
                    modified_count += 1;
                    
                    println!("  {} {} {}", 
                        colorize::warning("!"), 
                        colorize::path(relative_path.display()),
                        colorize::warning("Modified")
                    );
                    
                    if verbose {
                        if let (Ok(source_content), Ok(target_content)) = (
                            fs::read_to_string(&source_path),
                            fs::read_to_string(&target_path)
                        ) {
                            let source_lines: Vec<&str> = source_content.lines().collect();
                            let target_lines: Vec<&str> = target_content.lines().collect();
                            
                            println!("    {} {} lines, {} {} lines", 
                                colorize::info("Source:"), 
                                source_lines.len(),
                                colorize::info("Target:"), 
                                target_lines.len()
                            );
                            
                            let mut diff_count = 0;
                            let max_diffs = 3;
                            let max_line_len = 60;
                            
                            for i in 0..std::cmp::min(source_lines.len(), target_lines.len()) {
                                if source_lines[i] != target_lines[i] && diff_count < max_diffs {
                                    diff_count += 1;
                                    
                                    let source_snippet = if source_lines[i].len() > max_line_len {
                                        format!("{}...", &source_lines[i][0..max_line_len])
                                    } else {
                                        source_lines[i].to_string()
                                    };
                                    
                                    let target_snippet = if target_lines[i].len() > max_line_len {
                                        format!("{}...", &target_lines[i][0..max_line_len])
                                    } else {
                                        target_lines[i].to_string()
                                    };
                                    
                                    println!("    Line {}: ", i + 1);
                                    println!("      Source: {}", source_snippet);
                                    println!("      Target: {}", target_snippet);
                                }
                            }
                        }
                        println!();
                    }
                }
            }
        } else if source_path.is_dir() {
             if target_path.exists() && target_path.is_dir() {
                 println!("  {} {} {}", 
                     colorize::success("✓"), 
                     colorize::path(relative_path.display()),
                     colorize::success("Installed")
                 );
                 installed_count += 1;
             } else {
                 println!("  {} {} {}", 
                     colorize::error("✗"), 
                     colorize::path(relative_path.display()),
                     colorize::error("Not installed")
                 );
                 missing_count += 1;
             }
        }
    }

    println!("\n{}", colorize::header("Summary:"));
    println!("  {} {}", colorize::info("Total files and directories:"), colorize::highlight(total_count));
    println!("  {} {}", colorize::success("Installed:"), colorize::highlight(installed_count));
    println!("  {} {}", colorize::warning("Modified:"), colorize::highlight(modified_count));
    println!("  {} {}", colorize::error("Not installed:"), colorize::highlight(missing_count));

    Ok(())
}

pub fn uninstall_dotfiles(dry_run: bool, force: bool, verbose: bool) -> Result<()> {
    let config = read_config()?;
    let source_dir = &config.source_dir;

    let home_dir = get_home_dir()?;
    let source_dir = Path::new(source_dir);
    let backup_dir = get_backup_dir()?;

    if !source_dir.exists() {
        return Err(anyhow!("Source directory '{}' does not exist", source_dir.display()));
    }

    if verbose {
        println!("{} {}",
            colorize::info("Uninstalling dotfiles from"),
            colorize::path(home_dir.strip_prefix(&get_home_dir()?)?.display()));
        if dry_run {
            println!("{}", colorize::dry_run("Dry run mode: no files will be modified"));
        }
    } else {
        println!("{}", colorize::header("Uninstalling dotfiles..."));
    }

    let mut success_count = 0;
    let mut restored_count = 0;
    let mut skipped_count = 0;

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
            skipped_count += 1;
            continue;
        }

        let target_path = home_dir.join(relative_path);
        let rel_path_str = relative_path.to_string_lossy();

        if verbose {
            println!("  {} {}", colorize::info("Processing:"), colorize::path(relative_path.display()));
        }

        if !target_path.exists() {
            if verbose {
                println!("  {} {}", 
                    colorize::warning("Target file does not exist:"), 
                    colorize::path(target_path.strip_prefix(&get_home_dir()?)?.display()));
            } else {
                println!("  {} {}", 
                    colorize::warning("Skipped:"), 
                    colorize::path(relative_path.display()));
            }
            skipped_count += 1;
            continue;
        }

        // Check if the target is identical to the source
        let files_identical = match (fs::read(&source_path), fs::read(&target_path)) {
            (Ok(source_content), Ok(target_content)) => source_content == target_content,
            _ => false
        };

        if !files_identical && !force {
            if verbose {
                println!("  {} {} (use --force to remove)",
                    colorize::warning("Target file is modified, skipping:"),
                    colorize::path(relative_path.display()));
            } else {
                println!("  {} {} (use --force to remove)",
                    colorize::warning("Skipped (modified):"),
                    colorize::path(relative_path.display()));
            }
            skipped_count += 1;
            continue;
        }

        // Try to find a backup to restore
        match find_latest_backup(&rel_path_str, &backup_dir) {
            Ok(backup_path) => {
                if verbose {
                    println!("  {} {} with backup",
                        colorize::info("Replacing"),
                        colorize::path(relative_path.display()));
                } else {
                    println!("  {} {} (restoring backup)",
                        colorize::info("Uninstalling:"),
                        colorize::path(relative_path.display()));
                }

                if !dry_run {
                    fs::copy(&backup_path, &target_path)
                        .with_context(|| format!("Failed to restore backup {} to {}", 
                            backup_path.display(), target_path.display()))?;
                    restored_count += 1;

                    fs::remove_file(&backup_path)
                        .with_context(|| format!("Failed to delete backup file {}", backup_path.display()))?;
                        
                    if verbose {
                        println!("  {}", colorize::success("Backup restored and cleaned up"));
                    }
                } else if verbose {
                    println!("  {} {}",
                        colorize::dry_run("[Dry run] Would restore from backup:"),
                        colorize::path(backup_path.strip_prefix(&backup_dir)?.display()));
                }
            },
            Err(_) => {
                if verbose {
                    println!("  {} {}", 
                        colorize::info("Removing"),
                        colorize::path(relative_path.display()));
                } else {
                    println!("  {} {}",
                        colorize::info("Uninstalling:"),
                        colorize::path(relative_path.display()));
                }

                if !dry_run {
                    fs::remove_file(&target_path)
                        .with_context(|| format!("Failed to remove file {}", target_path.display()))?;
                    success_count += 1;
                    
                    if verbose {
                        println!("  {}", colorize::success("Removed successfully"));
                    }
                } else if verbose {
                    println!("  {} {}",
                        colorize::dry_run("[Dry run] Would remove:"),
                        colorize::path(target_path.strip_prefix(&home_dir)?.display()));
                }
            }
        }
    }

    if dry_run {
        println!("{}", colorize::dry_run("Dry run - no files were actually modified"));
    } else {
        println!("\n{}", colorize::header("Summary:"));
        if restored_count > 0 {
            println!("  {} {}", 
                colorize::success("Files restored from backup:"), 
                colorize::highlight(restored_count));
        }
        if success_count > 0 {
            println!("  {} {}", 
                colorize::success("Files removed:"), 
                colorize::highlight(success_count));
        }
        if skipped_count > 0 {
            println!("  {} {}", 
                colorize::warning("Files skipped:"), 
                colorize::highlight(skipped_count));
        }
        println!("{}", colorize::success("Uninstallation complete."));
    }

    Ok(())
}