# dotfiles-rust -- A Rust Dotfiles Manager

[![Crates.io Version](https://img.shields.io/crates/v/dotfiles-rust.svg?style=flat-square)](https://crates.io/crates/dotfiles-rust)
[![Downloads](https://img.shields.io/crates/d/dotfiles-rust.svg?style=flat-square)](https://crates.io/crates/dotfiles-rust)
![CI Status](https://github.com/f3rno/dotfiles-rust/actions/workflows/ci.yml/badge.svg)

[**dotfiles-rust**](https://crates.io/crates/dotfiles-rust) is a Rust CLI utility
for managing dotfiles installation and backups. It allows you to maintain your
dotfiles in a source directory and easily install them to your home directory,
with automatic backup creation and restoration capabilities.

The configuration is stored in a YAML file in your home directory at
`~/.dotfiles-rustrc.yaml`, and backups are maintained with timestamps for easy
version tracking and restoration.

## Example Usage

Below are a few example commands to illustrate the usual workflow for managing
dotfiles and backups.

```bash
dotfiles-rust init --source-dir ~/my-dotfiles
dotfiles-rust install
dotfiles-rust list
dotfiles-rust restore --file .vimrc
dotfiles-rust clear-backups
```

## Installation

Build from source using Cargo:

```bash
cargo install dotfiles-rust
```

Once installed, it will be available as the **dotfiles-rust** command.

## Commands

**dotfiles-rust** provides commands for both managing dotfile installation and
backup operations. To see a full list, run **`dotfiles-rust --help`**.

### Managing Dotfiles

- **`dotfiles-rust init`** -- initialize configuration with the current
directory as source
- **`dotfiles-rust init --source-dir <path>`** -- initialize with a specific
source directory
- **`dotfiles-rust install`** -- install dotfiles from source to home
directory
- **`dotfiles-rust install --dry-run`** -- preview installation without
making changes
- **`dotfiles-rust install --force`** -- overwrite existing files without
prompting

### Managing Backups

- **`dotfiles-rust list`** -- view all available backups
- **`dotfiles-rust list --file <name>`** -- view backups for a specific file
- **`dotfiles-rust restore`** -- restore all files from their latest backups
- **`dotfiles-rust restore --file <name>`** -- restore a specific file
- **`dotfiles-rust restore --version <timestamp>`** -- restore a specific
backup version
- **`dotfiles-rust clear-backups`** -- remove all backup files

### Useful Flags

Most commands support the following flags:
- **`--dry-run`** -- preview operations without making changes
- **`--force`** -- skip confirmation prompts
- **`--verbose`** -- display detailed operation information
- **`--keep-backups`** -- prevent automatic deletion of backup files after restore

## Configuration

The configuration file `.dotfiles-rustrc.yaml` is stored in your home directory:

```yaml
source_dir: /path/to/your/dotfiles
```

Legacy JSON configurations are automatically migrated to YAML format.

## Development

### Running Tests

```bash
cargo test
```

### Project Structure

- `src/main.rs` - Entry point
- `src/cli.rs` - CLI argument parsing
- `src/fs_utils.rs` - Filesystem utilities
- `src/backup.rs` - Backup functionality
- `src/commands.rs` - Command implementations
- `src/config.rs` - Configuration management
- `src/tests/` - Unit tests

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