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
    
    /// Uninstall dotfiles, restoring from backups when available
    Uninstall {
        /// Perform a dry run without making any changes
        #[arg(short, long)]
        dry_run: bool,
        
        /// Force removal even if files were modified
        #[arg(short, long)]
        force: bool,
        
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
    
    /// List available backups
    Backups {
        /// Specific file to list backups for
        #[arg(short, long)]
        file: Option<String>,
    },
    
    /// Clear all backup files
    Reset {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Show status of dotfiles
    Status {
        /// Display detailed file content differences
        #[arg(short, long)]
        verbose: bool,
    },
} 