use std::path::PathBuf;
use tempfile::tempdir;

use crate::fs_utils::{get_home_dir, get_backup_dir, ensure_parent_dirs, set_test_home_dir, set_test_backup_dir, set_test_id, clear_test_id};

// Set up a test environment with unique test ID
fn setup_test_dirs() -> (tempfile::TempDir, PathBuf, PathBuf) {
    // Generate a unique test ID
    let test_id = set_test_id();
    
    let temp_dir = tempdir().unwrap();
    let home_path = temp_dir.path().join(format!("home_{}", test_id));
    let backup_path = temp_dir.path().join(format!("backup_{}", test_id));
    
    std::fs::create_dir_all(&home_path).unwrap();
    std::fs::create_dir_all(&backup_path).unwrap();
    
    (temp_dir, home_path, backup_path)
}

// Clean up after tests
fn cleanup_test_dirs() {
    set_test_home_dir(None);
    set_test_backup_dir(None);
    clear_test_id();
}

#[test]
fn test_get_home_dir() {
    let (_, home_path, _) = setup_test_dirs();
    
    // Set the test home dir
    set_test_home_dir(Some(home_path.clone()));
    let home = get_home_dir().unwrap();
    assert_eq!(home, home_path);
    
    // Unset the test home dir
    set_test_home_dir(None);
    let home = get_home_dir().unwrap();
    assert!(home.starts_with("/"));
    
    // Restore test state
    cleanup_test_dirs();
}

#[test]
fn test_get_backup_dir() {
    let (_, home_path, backup_path) = setup_test_dirs();
    
    // Set test home directory
    set_test_home_dir(Some(home_path.clone()));
    
    // Create expected backup dir path
    let expected_backup_dir = home_path.join(".local/share/dotfiles-rust/backup");
    
    // Get actual backup dir
    let backup_dir = get_backup_dir().unwrap();
    assert_eq!(backup_dir, expected_backup_dir);
    
    // Test with explicit backup dir
    set_test_backup_dir(Some(backup_path.clone()));
    
    let backup_dir = get_backup_dir().unwrap();
    assert_eq!(backup_dir, backup_path);
    
    // Clean up test state
    cleanup_test_dirs();
}

#[test]
fn test_ensure_parent_dirs() {
    let (temp_dir, _, _) = setup_test_dirs();
    let file_path = temp_dir.path().join("a/b/c/file.txt");
    let parent_dir = file_path.parent().unwrap();

    assert!(!parent_dir.exists());
    ensure_parent_dirs(&file_path, false).unwrap();
    assert!(parent_dir.exists());

    let temp_dir_dry_run = tempdir().unwrap();
    let file_path_dry_run = temp_dir_dry_run.path().join("x/y/z/file.txt");
    let parent_dir_dry_run = file_path_dry_run.parent().unwrap();

    assert!(!parent_dir_dry_run.exists());
    ensure_parent_dirs(&file_path_dry_run, true).unwrap();
    assert!(!parent_dir_dry_run.exists());
    
    cleanup_test_dirs();
} 