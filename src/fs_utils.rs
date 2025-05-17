use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context, Result};
use std::thread_local;
use std::sync::Mutex;
use std::sync::LazyLock;

#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

thread_local! {
    static TEST_HOME_DIR: std::cell::RefCell<Option<PathBuf>> = std::cell::RefCell::new(None);
    static TEST_BACKUP_DIR: std::cell::RefCell<Option<PathBuf>> = std::cell::RefCell::new(None);
    static TEST_ID: std::cell::RefCell<Option<u64>> = std::cell::RefCell::new(None);
}

static HOME_ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[cfg(test)]
static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(test)]
pub fn set_test_id() -> u64 {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::SeqCst);
    TEST_ID.with(|test_id| {
        *test_id.borrow_mut() = Some(id);
    });
    id
}

#[cfg(test)]
pub fn clear_test_id() {
    TEST_ID.with(|test_id| {
        *test_id.borrow_mut() = None;
    });
}

#[cfg(test)]
pub fn set_test_home_dir(path: Option<PathBuf>) {
    TEST_HOME_DIR.with(|dir| {
        *dir.borrow_mut() = path;
    });
}

#[cfg(test)]
pub fn set_test_backup_dir(path: Option<PathBuf>) {
    TEST_BACKUP_DIR.with(|dir| {
        *dir.borrow_mut() = path;
    });
}

pub fn get_home_dir() -> Result<PathBuf> {
    let test_home = TEST_HOME_DIR.with(|dir| dir.borrow().clone());
    
    if let Some(home) = test_home {
        return Ok(home);
    }
    
    let _lock = HOME_ENV_LOCK.lock().unwrap();
    let home_path = env::var("HOME")
        .map(PathBuf::from)
        .or_else(|_| {
            if let Some(home) = dirs::home_dir() {
                Ok(home)
            } else {
                Err(anyhow!("Could not determine home directory"))
            }
        })?;
    
    Ok(home_path)
}

pub fn get_backup_dir() -> Result<PathBuf> {
    let test_backup = TEST_BACKUP_DIR.with(|dir| dir.borrow().clone());
    
    if let Some(backup) = test_backup {
        return Ok(backup);
    }
    
    let home = get_home_dir()?;
    let backup_dir = home.join(".local/share/dotfiles-rust/backup");
    
    if !backup_dir.exists() {
        println!("Creating backup directory: {}", backup_dir.display());
        fs::create_dir_all(&backup_dir)
            .with_context(|| format!("Failed to create backup directory {}", backup_dir.display()))?;
    }
    
    Ok(backup_dir)
}

pub fn ensure_parent_dirs(path: &Path, dry_run: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.exists() && !dry_run {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }
    }
    Ok(())
} 