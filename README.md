# syd

A dotfile management tool written in Rust that simplifies backing up and restoring your configuration files. Syd helps you maintain your dotfiles across different machines by automatically collecting them from various locations, storing them in a git repository, and syncing with cloud services like GitHub or GitLab.

## Features

- 🔄 Seamless backup and restore of dotfiles
- 📁 Support for files located anywhere in the system, not just in `$HOME/.config`
- 🌐 Integration with GitHub and GitLab for cloud backup
- ⚙️ Flexible configuration through files or command-line arguments
- 🦀 Written in Rust for performance and reliability

## Installation

Currently, syd is under development. Once released, you'll be able to install it using:

```bash
cargo install syd
```

## Usage

### Basic Commands

```bash
# Back up your dotfiles
syd backup

# Restore your dotfiles
syd restore

# List tracked files
syd list
```

### Configuration

You can configure syd in a configuration file (`~/.config/syd/syd.conf`)

#### Configuration File Example

```toml
[files]
# Location where files will be backed up
folder = "~/.local/share/syd"
# Files to track
paths = [
    "~/.zshrc",
    "~/.vimrc",
    "~/.config/nvim/init.vim"
]

[repository]
# Git repository settings
remote = "github.com/username/dotfiles"
branch = "main"
```

## License

This project is licensed under the Unlicense License - see the [LICENSE](LICENSE) file for details.


## Status

🚧 This project is currently under active development. Features and APIs may change.


