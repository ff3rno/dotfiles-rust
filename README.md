# dotfiles-rust -- A Rust Dotfile Manager

[![Crates.io Version](https://img.shields.io/crates/v/dotfiles-rust?style=flat-square)](https://crates.io/crates/dotfiles-rust)
[![Crates.io Downloads](https://img.shields.io/crates/d/dotfiles-rust?style=flat-square)](https://crates.io/crates/dotfiles-rust)
![CI GitHub Workflow Status](https://github.com/f3rno64/dotfiles-rust/actions/workflows/ci.yml/badge.svg)

**dotfiles-rust** is a Rust CLI utility for managing your dotfiles across different machines. It allows you to define your dotfiles in a source directory and easily install them to your home directory, with built-in support for backing up existing files and restoring them later. The configuration is stored in a simple YAML file at `~/.dotfiles-rustrc.yaml`.

## Features

- **Easy Installation:** Copy dotfiles from a specified source directory to your home directory.
- **Backup & Restore:** Automatically backs up existing files before overwriting and provides a command to restore them.
- **Status Check:** See which dotfiles are installed, modified, or missing from your home directory.
- **Backup Management:** List and clear old backups.
- **Configuration:** Simple YAML configuration file to specify your dotfiles source directory.

## Example Usage

Below are a few example commands to illustrate the usual workflow for managing your dotfiles.

```bash
dotfiles-rust init /path/to/your/dotfiles
dotfiles-rust install --backup --verbose
dotfiles-rust status
dotfiles-rust backups
dotfiles-rust uninstall
dotfiles-rust reset
```

## Installation

**dotfiles-rust** is available as a [**Crate**](https://crates.io/crates/dotfiles-rust); install it using `cargo`.

```bash
cargo install dotfiles-rust
```

Once installed, it will be available as the **dotfiles-rust** command.

## Commands

**dotfiles-rust** provides commands for initializing the configuration, installing, uninstalling, managing backups, and checking the status of your dotfiles. To see a full list and detailed options, run **`dotfiles-rust --help`**.

### Core Commands

- **`dotfiles-rust init <source_dir>`** -- Initializes the configuration file (`~/.dotfiles-rustrc.yaml`) with the path to your dotfiles source directory. This must be run first.
- **`dotfiles-rust install`** -- Installs dotfiles from your configured source directory to your home directory.
    - `--dry-run`: Shows what would be done without actually copying files.
    - `--force`: Overwrites existing files in the home directory that are different from the source.
    - `--backup`: Backs up existing files in the home directory before overwriting.
    - `--verbose`: Provides more detailed output during installation.
- **`dotfiles-rust uninstall`** -- Removes dotfiles from your home directory that were installed from your source directory. Attempts to restore from backups if available.
    - `--dry-run`: Shows what would be done without actually modifying files.
    - `--force`: Removes modified files even if they differ from the source/backup.
    - `--verbose`: Provides more detailed output during uninstallation.
- **`dotfiles-rust status`** -- Shows the status of your dotfiles in the home directory compared to the source directory (installed, modified, or missing).
    - `--verbose`: Shows details about differences for modified files.

### Backup Management

- **`dotfiles-rust backups`** -- Lists all backup files in the backup directory (`~/.dotfiles-rust_backups`).
    - `<file>`: Lists backup versions for a specific file.
- **`dotfiles-rust reset`** -- Clears all backup files from the backup directory.
    - `--force`: Skips the confirmation prompt before clearing backups.

## Release History

See [*CHANGELOG.md*](/CHANGELOG.md) for more information.

## License

Distributed under the **MIT** license. See [*LICENSE.md*](/LICENSE.md) for more information.

## Contributing

1. Fork it
2. Create your feature branch (`git checkout -b my-new-feature`)
3. Commit your changes (`git commit -am 'Add some feature'`)
4. Push to the branch (`git push origin my-new-feature`)
5. Create a new Pull Request

[crates-image]: https://img.shields.io/crates/v/dotfiles-rust.svg?style=flat-square
[crates-url]: https://crates.io/crates/dotfiles-rust
[crates-downloads]: https://img.shields.io/crates/d/dotfiles-rust.svg?style=flat-square
