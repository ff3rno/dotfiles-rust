use std::fs;
use std::path::{PathBuf};
use tempfile;

use crate::backup::*;
use crate::fs_utils::{set_test_home_dir, set_test_backup_dir, set_test_id, clear_test_id};

fn setup_test_dirs() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let test_id = set_test_id();
    
    let temp_dir = tempfile::tempdir().unwrap();
    let test_home = temp_dir.path().join(format!("home_{}", test_id));
    let backup_dir = temp_dir.path().join(format!("backup_{}", test_id));
    
    // Create directories explicitly
    fs::create_dir_all(&test_home).unwrap();
    fs::create_dir_all(&backup_dir).unwrap();
    
    // Set thread-local test directories
    set_test_home_dir(Some(test_home.clone()));
    set_test_backup_dir(Some(backup_dir.clone()));
    
    println!("Setup test dirs: home={}, backup={}", test_home.display(), backup_dir.display());
    
    (temp_dir, test_home, backup_dir)
}

// Clean up after tests
fn cleanup_test_dirs() {
    set_test_home_dir(None);
    set_test_backup_dir(None);
    clear_test_id();
}

#[test]
fn test_backup_file() {
    let (temp_dir, test_home, backup_dir) = setup_test_dirs();
    
    let file_path = test_home.join("test_file.txt");
    let file_content = "This is a test file.";
    fs::write(&file_path, file_content).unwrap();
    
    assert!(test_home.exists(), "Test home should exist");
    assert!(file_path.exists(), "Test file should exist");
    assert!(backup_dir.exists(), "Backup directory should exist");
    
    backup_file(&file_path, &backup_dir, false).unwrap();
    
    let entries = fs::read_dir(&backup_dir).unwrap()
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();
    
    assert!(!entries.is_empty(), "Backup directory should not be empty");
    
    let backup_file_path = entries.iter()
        .map(|e| e.path())
        .find(|p| {
            p.file_name().map_or(false, |name| 
                name.to_string_lossy().starts_with("test_file.txt."))
        });
    
    assert!(backup_file_path.is_some(), "Backup file should be created");
    
    let dry_run_dir = temp_dir.path().join("dry_run");
    fs::create_dir_all(&dry_run_dir).unwrap();
    
    let file_count_before = fs::read_dir(&dry_run_dir).unwrap().count();
    backup_file(&file_path, &dry_run_dir, true).unwrap();
    let file_count_after = fs::read_dir(&dry_run_dir).unwrap().count();
    
    assert_eq!(file_count_before, file_count_after, "Dry run should not create new files");
    
    // Cleanup test state
    cleanup_test_dirs();
}

#[test]
fn test_find_backup_by_version() {
    let (_, _, backup_dir) = setup_test_dirs();
    
    println!("Backup dir after setup: {}", backup_dir.display());
    println!("Backup dir exists: {}", backup_dir.exists());
    
    fs::create_dir_all(&backup_dir).unwrap();
    
    let backup_file_1 = backup_dir.join("test_file.txt.1678886400");
    let backup_file_2 = backup_dir.join("test_file.txt.1678972800");
    let backup_file_3 = backup_dir.join("test_file.txt.1679059200");
    
    println!("Writing backup files:");
    println!("  - {}", backup_file_1.display());
    println!("  - {}", backup_file_2.display());
    println!("  - {}", backup_file_3.display());
    
    fs::write(&backup_file_1, "Version 1").unwrap();
    fs::write(&backup_file_2, "Version 2").unwrap();
    fs::write(&backup_file_3, "Version 3").unwrap();
    
    println!("Checking if backup files exist:");
    println!("  - {} exists: {}", backup_file_1.display(), backup_file_1.exists());
    println!("  - {} exists: {}", backup_file_2.display(), backup_file_2.exists());
    println!("  - {} exists: {}", backup_file_3.display(), backup_file_3.exists());
    
    println!("Files in backup directory:");
    for entry in fs::read_dir(&backup_dir).unwrap() {
        let entry = entry.unwrap();
        println!("  - {}", entry.path().display());
    }
    
    println!("Calling find_backup_by_version(\"test_file.txt\", \"1678972800\", {})", backup_dir.display());
    let result = find_backup_by_version("test_file.txt", "1678972800", &backup_dir);
    
    match &result {
        Ok(path) => println!("Found backup at: {}", path.display()),
        Err(e) => println!("Error finding backup: {}", e),
    }
    
    let found_backup = result.unwrap();
    assert_eq!(found_backup, backup_file_2);
    
    cleanup_test_dirs();
}

#[test]
fn test_find_latest_backup() {
    let (_, _, backup_dir) = setup_test_dirs();
    
    fs::create_dir_all(&backup_dir).unwrap();
    
    let backup_file_1 = backup_dir.join("test_file.txt.1678886400");
    let backup_file_2 = backup_dir.join("test_file.txt.1678972800");
    let backup_file_3 = backup_dir.join("test_file.txt.1679059200");
    
    fs::write(&backup_file_1, "Version 1").unwrap();
    fs::write(&backup_file_2, "Version 2").unwrap();
    fs::write(&backup_file_3, "Version 3").unwrap();
    
    assert!(backup_file_1.exists(), "Backup file 1 should exist");
    assert!(backup_file_2.exists(), "Backup file 2 should exist");
    assert!(backup_file_3.exists(), "Backup file 3 should exist");
    
    let latest_backup = find_latest_backup("test_file.txt", &backup_dir).unwrap();
    assert_eq!(latest_backup, backup_file_3);
    
    cleanup_test_dirs();
}

#[test]
fn test_find_all_backup_versions() {
    let (_, _, backup_dir) = setup_test_dirs();
    
    fs::create_dir_all(&backup_dir).unwrap();
    
    let backup_file_1 = backup_dir.join("test_file.txt.1678886400");
    let backup_file_2 = backup_dir.join("test_file.txt.1678972800");
    let backup_file_3 = backup_dir.join("test_file.txt.1679059200");
    
    fs::write(&backup_file_1, "Version 1").unwrap();
    fs::write(&backup_file_2, "Version 2").unwrap();
    fs::write(&backup_file_3, "Version 3").unwrap();
    
    assert!(backup_file_1.exists(), "Backup file 1 should exist");
    assert!(backup_file_2.exists(), "Backup file 2 should exist");
    assert!(backup_file_3.exists(), "Backup file 3 should exist");
    
    let versions = find_all_backup_versions("test_file.txt", &backup_dir).unwrap();
    
    assert_eq!(versions.len(), 3, "Should find 3 backup versions");
    
    assert_eq!(versions[0], (1678886400, backup_file_1), "First element should be oldest backup");
    assert_eq!(versions[1], (1678972800, backup_file_2), "Second element should be middle backup");
    assert_eq!(versions[2], (1679059200, backup_file_3), "Third element should be newest backup");
    
    cleanup_test_dirs();
} 