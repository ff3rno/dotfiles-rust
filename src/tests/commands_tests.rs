use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use anyhow::Result;
use tempfile::{tempdir, TempDir};

use crate::commands::{install_dotfiles, list_backups, clear_backups, uninstall_dotfiles};
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

#[test]
fn test_uninstall_dotfiles() -> Result<()> {
    let (temp_dir, temp_home, backup_dir) = setup_test_env()?;
    
    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir)?;
    
    let config = Config {
        source_dir: source_dir.to_str().unwrap().to_string(),
    };
    write_config(&config)?;
    
    // Create source files
    create_test_file(&source_dir.join(".vimrc"), "vimrc content")?;
    create_test_file(&source_dir.join(".bashrc"), "bashrc content")?;
    create_test_file(&source_dir.join(".zshrc"), "zshrc content")?;
    create_test_file(&source_dir.join(".config/fish/config.fish"), "fish config content")?;
    
    // Install files to home
    install_dotfiles(false, false, true, false)?;
    
    // Verify files were installed
    assert!(temp_home.join(".vimrc").exists(), ".vimrc should be installed");
    assert!(temp_home.join(".bashrc").exists(), ".bashrc should be installed");
    assert!(temp_home.join(".zshrc").exists(), ".zshrc should be installed");
    assert!(temp_home.join(".config/fish/config.fish").exists(), "fish config should be installed");
    
    // Create backups for some files
    let backup_vimrc = backup_dir.join(".vimrc.1000000000");
    let backup_bashrc = backup_dir.join(".bashrc.1000000000");
    
    create_test_file(&backup_vimrc, "backup vimrc content")?;
    create_test_file(&backup_bashrc, "backup bashrc content")?;
    
    // Modify a home file to test the force flag
    create_test_file(&temp_home.join(".zshrc"), "modified zshrc content")?;
    
    // Uninstall dotfiles without force flag
    uninstall_dotfiles(false, false, false)?;
    
    // Check files with backups were replaced with backup content
    assert!(temp_home.join(".vimrc").exists(), ".vimrc should exist (replaced with backup)");
    assert!(temp_home.join(".bashrc").exists(), ".bashrc should exist (replaced with backup)");
    
    // Verify content was restored from backups
    let vimrc_content = fs::read_to_string(temp_home.join(".vimrc"))?;
    let bashrc_content = fs::read_to_string(temp_home.join(".bashrc"))?;
    assert_eq!(vimrc_content, "backup vimrc content", "Vimrc should be restored from backup");
    assert_eq!(bashrc_content, "backup bashrc content", "Bashrc should be restored from backup");
    
    // Check that the modified file was not removed
    assert!(temp_home.join(".zshrc").exists(), ".zshrc should not be removed (modified without force)");
    let zshrc_content = fs::read_to_string(temp_home.join(".zshrc"))?;
    assert_eq!(zshrc_content, "modified zshrc content", "Modified zshrc should not be changed");
    
    // Check that fish config was uninstalled (no backup)
    assert!(!temp_home.join(".config/fish/config.fish").exists(), "fish config should be removed (no backup)");
    
    // Check that backups were removed
    assert!(!backup_vimrc.exists(), "vimrc backup should be deleted");
    assert!(!backup_bashrc.exists(), "bashrc backup should be deleted");
    
    // Test uninstall with force flag
    
    // First reinstall everything
    install_dotfiles(false, true, true, false)?;
    
    // Create new backups with higher timestamps to ensure they're chosen as latest
    let new_backup_vimrc = backup_dir.join(".vimrc.2000000000");
    let new_backup_bashrc = backup_dir.join(".bashrc.2000000000");
    let new_backup_zshrc = backup_dir.join(".zshrc.2000000000");
    
    create_test_file(&new_backup_vimrc, "new backup vimrc content")?;
    create_test_file(&new_backup_bashrc, "new backup bashrc content")?;
    create_test_file(&new_backup_zshrc, "new backup zshrc content")?;
    
    // Modify zshrc again
    create_test_file(&temp_home.join(".zshrc"), "modified zshrc content again")?;
    
    // Uninstall with force flag
    uninstall_dotfiles(false, true, false)?;
    
    // Check that files with backups were restored from backup
    assert!(temp_home.join(".vimrc").exists(), ".vimrc should exist (replaced with backup)");
    assert!(temp_home.join(".bashrc").exists(), ".bashrc should exist (replaced with backup)");
    assert!(temp_home.join(".zshrc").exists(), ".zshrc should exist (replaced with backup)");
    
    // Verify content is from backups
    let vimrc_content = fs::read_to_string(temp_home.join(".vimrc"))?;
    let bashrc_content = fs::read_to_string(temp_home.join(".bashrc"))?;
    let zshrc_content = fs::read_to_string(temp_home.join(".zshrc"))?;
    
    assert_eq!(vimrc_content, "new backup vimrc content", "Vimrc should be restored from backup");
    assert_eq!(bashrc_content, "new backup bashrc content", "Bashrc should be restored from backup");
    assert_eq!(zshrc_content, "new backup zshrc content", "Zshrc should be restored from backup");
    
    // Check that fish config was uninstalled (no backup)
    assert!(!temp_home.join(".config/fish/config.fish").exists(), "fish config should be removed");
    
    // Test dry run
    
    // First reinstall everything
    install_dotfiles(false, true, false, false)?;
    
    // Create new backups - use timestamp 3000000000 to ensure it's selected as the latest
    let newest_backup_vimrc = backup_dir.join(".vimrc.3000000000");
    create_test_file(&newest_backup_vimrc, "newest backup vimrc content")?;
    
    // Uninstall with dry run
    uninstall_dotfiles(true, true, false)?;
    
    // Check that no files were actually removed or changed
    assert!(temp_home.join(".vimrc").exists(), ".vimrc should still exist after dry run");
    assert!(temp_home.join(".bashrc").exists(), ".bashrc should still exist after dry run");
    assert!(temp_home.join(".zshrc").exists(), ".zshrc should still exist after dry run");
    assert!(temp_home.join(".config/fish/config.fish").exists(), "fish config should still exist after dry run");
    
    // Check backup still exists
    assert!(newest_backup_vimrc.exists(), "vimrc backup should still exist after dry run");
    
    cleanup_test_env();
    
    Ok(())
}

#[test]
fn test_uninstall_with_blacklist() -> Result<()> {
    let (temp_dir, temp_home, _) = setup_test_env()?;
    
    let source_dir = temp_dir.path().join("source");
    fs::create_dir_all(&source_dir)?;
    
    let config = Config {
        source_dir: source_dir.to_str().unwrap().to_string(),
    };
    write_config(&config)?;
    
    // Create source files including blacklisted ones
    create_test_file(&source_dir.join(".vimrc"), "vimrc content")?;
    create_test_file(&source_dir.join(".git/config"), "[core]")?;
    create_test_file(&source_dir.join("node_modules/some_package/index.js"), "console.log('hello')")?;
    create_test_file(&source_dir.join(".DS_Store"), "binary data")?;
    
    // Manually create these in home dir (they shouldn't be uninstalled since they're blacklisted)
    create_test_file(&temp_home.join(".git/config"), "[core] repositoryformatversion = 0")?;
    create_test_file(&temp_home.join("node_modules/some_package/index.js"), "console.log('modified')")?;
    create_test_file(&temp_home.join(".DS_Store"), "modified binary data")?;
    
    // Install vimrc to home
    install_dotfiles(false, false, false, false)?;
    
    // Verify vimrc was installed
    assert!(temp_home.join(".vimrc").exists(), ".vimrc should be installed");
    
    // Run uninstall
    uninstall_dotfiles(false, true, false)?;
    
    // Check that vimrc was removed
    assert!(!temp_home.join(".vimrc").exists(), ".vimrc should be removed");
    
    // Check that blacklisted files were not removed
    assert!(temp_home.join(".git/config").exists(), ".git/config should not be removed (blacklisted)");
    assert!(temp_home.join("node_modules/some_package/index.js").exists(), "node_modules file should not be removed (blacklisted)");
    assert!(temp_home.join(".DS_Store").exists(), ".DS_Store should not be removed (blacklisted)");
    
    cleanup_test_env();
    
    Ok(())
} 