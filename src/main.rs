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
use crate::commands::{list_backups, clear_backups, uninstall_dotfiles};
use crate::config::initialize_config;
use colored;

fn main() -> Result<()> {
    colored::control::set_override(true);
    let cli = Cli::parse();
    
    match cli.command {
        Args::Install { dry_run, force, backup, verbose } => {
            commands::install_dotfiles(dry_run, force, backup, verbose)
        },
        Args::Uninstall { dry_run, force, verbose } => {
            uninstall_dotfiles(dry_run, force, verbose)
        },
        Args::Init { source_dir } => {
            println!("{} {}", colorize::info("Initializing config with source directory:"), colorize::path(&source_dir));
            initialize_config(&source_dir)?;
            println!("{} {}", colorize::success("Configuration file created at"), colorize::path("~/.dotfiles-rustrc.yaml"));
            Ok(())
        },
        Args::Backups { file } => {
            list_backups(file.as_deref())
        },
        Args::Reset { force } => {
            clear_backups(force)
        },
        Args::Status { verbose } => {
            commands::status_dotfiles(verbose)
        }
    }
} 