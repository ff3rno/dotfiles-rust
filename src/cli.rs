use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about = "Manage dotfiles installation and backups")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Args,
}

#[derive(Subcommand)]
pub enum Args {
    /// Install dotfiles from the configured source directory to your home
    Install {
        /// Perform a dry run without making any changes
        #[arg(short, long)]
        dry_run: bool,
        
        /// Force overwrite existing files
        #[arg(short, long)]
        force: bool,
        
        /// Create backups of existing files before overwriting
        #[arg(short, long, default_value = "true")]
        backup: bool,
        
        /// Display verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Initialize configuration file with source directory
    Init {
        /// Source directory containing dotfiles
        #[arg(short, long, default_value = ".")]
        source_dir: String,
    },
    
    /// Restore files from backups
    Restore {
        /// Specific file to restore (if not specified, all files will be restored)
        #[arg(short, long)]
        file: Option<String>,
        
        /// Specific backup version to restore (timestamp)
        #[arg(short, long)]
        version: Option<String>,
        
        /// Perform a dry run without making any changes
        #[arg(short, long)]
        dry_run: bool,
        
        /// Keep backup files after successful restore (default: false)
        #[arg(short, long, default_value = "false")]
        keep_backups: bool,
    },
    
    /// List available backups
    List {
        /// Specific file to list backups for
        #[arg(short, long)]
        file: Option<String>,
    },
    
    /// Clear all backup files
    ClearBackups {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
} 