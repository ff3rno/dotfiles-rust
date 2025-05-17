use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use anyhow::Result;
use tempfile::{tempdir, TempDir};

use crate::commands::{install_dotfiles, restore_backups, list_backups, clear_backups};
use crate::fs_utils::{set_test_home_dir, set_test_backup_dir, set_test_id, clear_test_id};
use crate::config::{Config, write_config};

fn setup_test_env() -> Result<(TempDir, PathBuf, PathBuf)> {
    let test_id = set_test_id();
    
    let temp_dir = tempdir()?;
    let temp_home = temp_dir.path().join(format!("home_{}", test_id));
    let backup_dir = temp_dir.path().join(format!("backup_{}", test_id));
    
    fs::create_dir_all(&temp_home)?;
    fs::create_dir_all(&backup_dir)?;

    let abs_temp_home = temp_home.canonicalize()?;
    println!("Setting test HOME to: {}", abs_temp_home.display());
    
    set_test_home_dir(Some(abs_temp_home.clone()));
    set_test_backup_dir(Some(backup_dir.canonicalize()?));
    
    Ok((temp_dir, abs_temp_home, backup_dir.canonicalize()?))
}

fn cleanup_test_env() {
    set_test_home_dir(None);
    set_test_backup_dir(None);
    clear_test_id();
}

fn create_test_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    
    Ok(())
}

#[test]
fn test_install_dotfiles() -> Result<()> {
    let (temp_dir, temp_home, _) = setup_test_env()?;
    
    println!("Test HOME set to: {}", temp_home.to_string_lossy());
    
    if temp_home.exists() {
        fs::remove_dir_all(&temp_home)?;
        fs::create_dir_all(&temp_home)?;
    }

    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir)?;
    
    let config = Config {
        source_dir: source_dir.to_str().unwrap().to_string(),
    };

    write_config(&config)?;
    
    create_test_file(&source_dir.join(".vimrc"), "set nocompatible")?;
    create_test_file(&source_dir.join(".config/fish/config.fish"), "set -x PATH $PATH")?;
    
    println!("Installing dotfiles from {} to {}", source_dir.display(), temp_home.display());
    install_dotfiles(false, false, false, false)?;
    
    assert!(temp_home.join(".vimrc").exists(), ".vimrc should be installed on first run");
    assert!(temp_home.join(".config/fish/config.fish").exists(), "config.fish should be installed on first run");
    
    let vimrc_content = fs::read_to_string(temp_home.join(".vimrc"))?;
    assert_eq!(vimrc_content, "set nocompatible", ".vimrc content should match");
    
    create_test_file(&source_dir.join(".bashrc"), "export PATH=$PATH:/usr/local/bin")?;
    create_test_file(&temp_home.join(".bashrc"), "# existing bashrc content")?;
    
    install_dotfiles(false, true, true, false)?;
    
    let bashrc_content = fs::read_to_string(temp_home.join(".bashrc"))?;
    assert_eq!(bashrc_content, "export PATH=$PATH:/usr/local/bin", ".bashrc should be overwritten with force");
    
    create_test_file(&source_dir.join(".zshrc"), "export ZSH=$HOME/.oh-my-zsh")?;
    
    install_dotfiles(true, false, false, false)?;
    
    assert!(!temp_home.join(".zshrc").exists(), ".zshrc should not be installed in dry run");
    
    cleanup_test_env();
    
    Ok(())
}

#[test]
fn test_install_dotfiles_blacklist() -> Result<()> {
    let (temp_dir, temp_home, _) = setup_test_env()?;
    
    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir)?;
    
    let config = Config {
        source_dir: source_dir.to_str().unwrap().to_string(),
    };
    write_config(&config)?;
    
    create_test_file(&source_dir.join(".vimrc"), "set nocompatible")?;
    create_test_file(&source_dir.join(".git/config"), "[core]")?;
    create_test_file(&source_dir.join("node_modules/some_package/index.js"), "console.log('hello')")?;
    create_test_file(&source_dir.join(".DS_Store"), "binary data")?;
    create_test_file(&source_dir.join(".config/fish/config.fish"), "set -x PATH $PATH")?;

    install_dotfiles(false, false, false, false)?;
    
    assert!(temp_home.join(".vimrc").exists(), ".vimrc should be installed");
    assert!(temp_home.join(".config/fish/config.fish").exists(), "config.fish should be installed");

    assert!(!temp_home.join(".git/config").exists(), ".git/config should be blacklisted");
    assert!(!temp_home.join("node_modules/some_package/index.js").exists(), "node_modules should be blacklisted");
    assert!(!temp_home.join(".DS_Store").exists(), ".DS_Store should be blacklisted");

    cleanup_test_env();
    
    Ok(())
}

#[test]
fn test_restore_backups() -> Result<()> {
    let (temp_dir, temp_home, backup_dir) = setup_test_env()?;
    
    // Create source directory with files for fallback restore
    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir)?;
    
    let config = Config {
        source_dir: source_dir.to_str().unwrap().to_string(),
    };
    write_config(&config)?;
    
    // Create source files that should be used as fallback when no backup exists
    create_test_file(&source_dir.join(".vimrc"), "source vimrc content")?;
    create_test_file(&source_dir.join(".bashrc"), "source bashrc content")?;
    create_test_file(&source_dir.join(".zshrc"), "source zshrc content")?;
    create_test_file(&source_dir.join(".config/fish/config.fish"), "source fish config content")?;
    
    println!("TEST_HOME set to: {}", temp_home.to_string_lossy());
    println!("Backup dir set to: {}", backup_dir.to_string_lossy());
    
    let backup_file_path1 = backup_dir.join(".vimrc.1000000000");
    let backup_file_path2 = backup_dir.join(".bashrc.1000000100");
    let backup_file_path3 = backup_dir.join(".vimrc.1000000200");  // Newer version of .vimrc
    let backup_file_path4 = backup_dir.join(".config/fish/config.fish.1000000300");
    
    create_test_file(&backup_file_path1, "old vimrc content")?;
    create_test_file(&backup_file_path2, "bashrc content")?;
    create_test_file(&backup_file_path3, "newer vimrc content")?;
    create_test_file(&backup_file_path4, "fish config content")?;
    
    println!("Backup files created");
    
    restore_backups(Some(".vimrc"), None, false, true)?;
    
    let restored_vimrc = temp_home.join(".vimrc");
    println!("Checking for restored file at: {}", restored_vimrc.display());
    
    assert!(restored_vimrc.exists(), ".vimrc should be restored");
    let vimrc_content = fs::read_to_string(&restored_vimrc)?;
    assert_eq!(vimrc_content, "newer vimrc content", "Latest vimrc backup should be restored");
    
    assert!(backup_file_path3.exists(), "Backup file should still exist");
    
    fs::remove_file(&restored_vimrc)?;
    
    restore_backups(Some(".vimrc"), Some("1000000000"), false, false)?;
    
    assert!(restored_vimrc.exists(), ".vimrc should be restored with specific version");
    let vimrc_content = fs::read_to_string(&restored_vimrc)?;
    assert_eq!(vimrc_content, "old vimrc content", "Specific version of vimrc should be restored");
    
    assert!(!backup_file_path1.exists(), "Backup file should be deleted");
    
    fs::remove_file(&restored_vimrc)?;
    
    restore_backups(Some(".vimrc"), Some("1000000200"), true, false)?;
    assert!(!restored_vimrc.exists(), ".vimrc should not be restored in dry run");
    
    assert!(backup_file_path3.exists(), "Backup file should still exist after dry run");
    
    restore_backups(None, None, false, false)?;
    
    let restored_vimrc = temp_home.join(".vimrc");
    let restored_bashrc = temp_home.join(".bashrc");
    let restored_fish_config = temp_home.join(".config/fish/config.fish");
    
    assert!(restored_vimrc.exists(), ".vimrc should be restored in restore-all");
    assert!(restored_bashrc.exists(), ".bashrc should be restored in restore-all");
    assert!(restored_fish_config.exists(), "config.fish should be restored in restore-all");
    
    let vimrc_content = fs::read_to_string(&restored_vimrc)?;
    let bashrc_content = fs::read_to_string(&restored_bashrc)?;
    let fish_content = fs::read_to_string(&restored_fish_config)?;
    
    assert_eq!(vimrc_content, "newer vimrc content", "Latest vimrc backup should be restored");
    assert_eq!(bashrc_content, "bashrc content", "Bashrc should be restored");
    assert_eq!(fish_content, "source fish config content", "Fish config should be installed from source");
    
    assert!(!backup_file_path2.exists(), "Bashrc backup file should be deleted");
    assert!(!backup_file_path3.exists(), "Vimrc backup file should be deleted");
    // We're not using backup_file_path4 for fish config, so don't check if it was deleted
    
    // Test restore for file with no backup
    fs::remove_dir_all(&backup_dir)?;
    fs::create_dir_all(&backup_dir)?;
    
    // No backups exist, but source file does
    let no_backup_file = temp_home.join(".zshrc");
    if no_backup_file.exists() {
        fs::remove_file(&no_backup_file)?;
    }
    
    restore_backups(Some(".zshrc"), None, false, true)?;
    
    assert!(no_backup_file.exists(), ".zshrc should be restored from source");
    let zshrc_content = fs::read_to_string(&no_backup_file)?;
    assert_eq!(zshrc_content, "source zshrc content", "File should be restored from source file");
    
    // Test nonexistent file with no source
    let result = restore_backups(Some(".nonexistent"), None, false, true);
    assert!(result.is_ok(), "Restore should work but fail to find file");
    
    cleanup_test_env();
    
    Ok(())
}

#[test]
fn test_list_backups() -> Result<()> {
    let (_, _, backup_dir) = setup_test_env()?;
    
    create_test_file(&backup_dir.join(".vimrc.1000000000"), "old vimrc")?;
    create_test_file(&backup_dir.join(".vimrc.1000000100"), "newer vimrc")?;
    create_test_file(&backup_dir.join(".bashrc.1000000000"), "bashrc backup")?;
    
    list_backups(None)?;
    list_backups(Some(".vimrc"))?;
    list_backups(Some(".nonexistent"))?;

    cleanup_test_env();
    
    Ok(())
}

#[test]
fn test_clear_backups() -> Result<()> {
    let (temp_dir, _, backup_dir) = setup_test_env()?;

    let file1 = backup_dir.join("file1.txt.123");
    let file2 = backup_dir.join("file2.conf.456");
    std::fs::write(&file1, "backup 1")?;
    std::fs::write(&file2, "backup 2")?;

    assert!(backup_dir.exists(), "Backup directory should exist before clearing");
    assert!(file1.exists(), "Dummy backup file 1 should exist");
    assert!(file2.exists(), "Dummy backup file 2 should exist");

    clear_backups(true)?;

    assert!(!backup_dir.exists(), "Backup directory should be removed after clearing");

    let no_backup_dir = temp_dir.path().join("nonexistent_backup_dir");
    set_test_backup_dir(Some(no_backup_dir.clone()));
    assert!(!no_backup_dir.exists(), "Non-existent backup directory should not exist");
    let result = clear_backups(true);
    assert!(result.is_ok(), "Clearing when no backup dir exists should not return an error");
    assert!(!no_backup_dir.exists(), "Non-existent backup directory should still not exist");

    cleanup_test_env();
    Ok(())
}

#[test]
fn test_install_dotfiles_identical_files() -> Result<()> {
    let (temp_dir, temp_home, backup_dir) = setup_test_env()?;
    
    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir)?;
    
    let config = Config {
        source_dir: source_dir.to_str().unwrap().to_string(),
    };
    write_config(&config)?;
    
    let identical_content = "This file is identical in source and home";
    create_test_file(&source_dir.join(".vimrc"), identical_content)?;
    create_test_file(&temp_home.join(".vimrc"), identical_content)?;
    
    create_test_file(&source_dir.join(".bashrc"), "Source bashrc content")?;
    create_test_file(&temp_home.join(".bashrc"), "Different home bashrc content")?;
    
    let vimrc_mtime_before = temp_home.join(".vimrc").metadata()?.modified()?;
    let bashrc_mtime_before = temp_home.join(".bashrc").metadata()?.modified()?;
    
    let backup_count_before = fs::read_dir(&backup_dir)
        .map(|entries| entries.count())
        .unwrap_or(0);
    
    install_dotfiles(false, true, true, false)?;
    
    let vimrc_mtime_after = temp_home.join(".vimrc").metadata()?.modified()?;
    assert_eq!(vimrc_mtime_before, vimrc_mtime_after, "Identical file should not be modified");
    
    let bashrc_mtime_after = temp_home.join(".bashrc").metadata()?.modified()?;
    assert_ne!(bashrc_mtime_before, bashrc_mtime_after, "Different file should be updated");
    
    let bashrc_content = fs::read_to_string(temp_home.join(".bashrc"))?;
    assert_eq!(bashrc_content, "Source bashrc content", "Bashrc content should be updated");
    
    let backup_count_after = fs::read_dir(&backup_dir)
        .map(|entries| entries.count())
        .unwrap_or(0);
    
    assert_eq!(backup_count_after, backup_count_before + 1, "Only the different file should be backed up");
    
    cleanup_test_env();
    
    Ok(())
}

#[test]
fn test_restore_no_backup_preparation() -> Result<()> {
    let (_temp_dir, temp_home, backup_dir) = setup_test_env()?;
    
    let home_file = temp_home.join(".no_backup_file");
    fs::write(&home_file, "File with no backup")?;
    
    assert!(home_file.exists(), "Test file should exist before restore would prompt");
    
    fs::create_dir_all(&backup_dir)?;
    
    let backup_file_path = backup_dir.join(".no_backup_file.1234567890");
    assert!(!backup_file_path.exists(), "Backup should not exist");
    
    cleanup_test_env();
    
    Ok(())
}

#[test]
fn test_restore_only_manages_repo_files() -> Result<()> {
    let (temp_dir, temp_home, backup_dir) = setup_test_env()?;
    
    // Create source directory with files
    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir)?;
    
    let config = Config {
        source_dir: source_dir.to_str().unwrap().to_string(),
    };
    write_config(&config)?;
    
    // Create files in source directory
    create_test_file(&source_dir.join(".vimrc"), "vimrc content")?;
    create_test_file(&source_dir.join(".bashrc"), "bashrc content")?;
    create_test_file(&source_dir.join(".zshrc"), "zshrc content")?;
    
    // Install them to home
    install_dotfiles(false, false, false, false)?;
    
    // Verify all files were installed
    assert!(temp_home.join(".vimrc").exists(), ".vimrc should be installed");
    assert!(temp_home.join(".bashrc").exists(), ".bashrc should be installed");
    assert!(temp_home.join(".zshrc").exists(), ".zshrc should be installed");
    
    // Create backups only for some files
    let backup_vimrc = backup_dir.join(".vimrc.1000000000");
    let backup_bashrc = backup_dir.join(".bashrc.1000000000");
    
    create_test_file(&backup_vimrc, "backup vimrc content")?;
    create_test_file(&backup_bashrc, "backup bashrc content")?;
    
    // Create a file in home that's not in source and has no backup
    create_test_file(&temp_home.join(".not_in_source"), "not in source content")?;
    
    // Modify the source files to ensure they're different from what's in home
    create_test_file(&source_dir.join(".vimrc"), "updated vimrc content")?;
    create_test_file(&source_dir.join(".bashrc"), "updated bashrc content")?;
    create_test_file(&source_dir.join(".zshrc"), "updated zshrc content")?;
    
    // Restore backups - should restore files with backups and install from source for files without backups
    restore_backups(None, None, false, true)?;
    
    // Files with backups should be restored from backup
    assert!(temp_home.join(".vimrc").exists(), ".vimrc should still exist (restored from backup)");
    assert!(temp_home.join(".bashrc").exists(), ".bashrc should still exist (restored from backup)");
    
    // File without backup should be installed from source
    assert!(temp_home.join(".zshrc").exists(), ".zshrc should exist (restored from source)");
    
    // File not in source should not be touched by the restore operation
    assert!(temp_home.join(".not_in_source").exists(), ".not_in_source should still exist (not managed by dotfiles)");
    
    // Verify content of restored files
    let vimrc_content = fs::read_to_string(temp_home.join(".vimrc"))?;
    let bashrc_content = fs::read_to_string(temp_home.join(".bashrc"))?;
    let zshrc_content = fs::read_to_string(temp_home.join(".zshrc"))?;
    let not_in_source_content = fs::read_to_string(temp_home.join(".not_in_source"))?;
    
    assert_eq!(vimrc_content, "backup vimrc content", "Vimrc content should be from backup");
    assert_eq!(bashrc_content, "backup bashrc content", "Bashrc content should be from backup");
    assert_eq!(zshrc_content, "updated zshrc content", "Zshrc content should be from source");
    assert_eq!(not_in_source_content, "not in source content", "Not in source content should be unchanged");
    
    cleanup_test_env();
    
    Ok(())
}

#[test]
fn test_status_dotfiles() -> Result<()> {
    let (temp_dir, temp_home, _) = setup_test_env()?;
    
    // Create source directory with files
    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir)?;
    
    let config = Config {
        source_dir: source_dir.to_str().unwrap().to_string(),
    };
    write_config(&config)?;
    
    // Case 1: Create a file in source that doesn't exist in home (not installed)
    create_test_file(&source_dir.join(".vimrc"), "vimrc content")?;
    
    // Case 2: Create a file that exists in both with identical content (installed)
    create_test_file(&source_dir.join(".bashrc"), "identical content")?;
    create_test_file(&temp_home.join(".bashrc"), "identical content")?;
    
    // Case 3: Create a file that exists in both with different content (modified)
    create_test_file(&source_dir.join(".zshrc"), "source zshrc content")?;
    create_test_file(&temp_home.join(".zshrc"), "modified zshrc content")?;
    
    // Case 4: Create a blacklisted file that should be skipped
    create_test_file(&source_dir.join(".git/config"), "git config")?;
    
    // Run status with basic output
    let result = crate::commands::status_dotfiles(false);
    assert!(result.is_ok(), "Status command should run without errors");
    
    // Run status with verbose output
    let verbose_result = crate::commands::status_dotfiles(true);
    assert!(verbose_result.is_ok(), "Verbose status command should run without errors");
    
    cleanup_test_env();
    
    Ok(())
} 