mod cli;
mod fs_utils;
mod backup;
mod commands;
mod config;
mod colorize;
#[cfg(test)]
mod tests;

use anyhow::Result;
use clap::Parser;
use crate::cli::{Cli, Args};
use crate::commands::{restore_backups, list_backups, clear_backups};
use crate::config::initialize_config;
use colored;

fn main() -> Result<()> {
    colored::control::set_override(true);
    let cli = Cli::parse();
    
    match cli.command {
        Args::Install { dry_run, force, backup, verbose } => {
            commands::install_dotfiles(dry_run, force, backup, verbose)
        },
        Args::Init { source_dir } => {
            println!("{} {}", colorize::info("Initializing config with source directory:"), colorize::path(&source_dir));
            initialize_config(&source_dir)?;
            println!("{} {}", colorize::success("Configuration file created at"), colorize::path("~/.dotfiles-rustrc.yaml"));
            Ok(())
        },
        Args::Restore { file, version, dry_run, keep_backups } => {
            restore_backups(file.as_deref(), version.as_deref(), dry_run, keep_backups)
        },
        Args::List { file } => {
            list_backups(file.as_deref())
        },
        Args::ClearBackups { force } => {
            clear_backups(force)
        },
        Args::Status { verbose } => {
            commands::status_dotfiles(verbose)
        }
    }
} 