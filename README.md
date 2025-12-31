# Xplore CLI

A high-performance, full-screen CLI file manager and metadata tagger for Linux and Windows. 

Xplore uses **extended attributes (xattrs)** to store file descriptions directly on the filesystem, ensuring your notes stay with your files without the need for a database.

## Features
- 🚀 **Fast Navigation**: Browse your filesystem with Vim-like keys or arrow keys.
- 📝 **Metadata Tagging**: Add multiline descriptions to any file via `xattrs`.
- 🔍 **Deep Global Search**: Search for files and descriptions across your entire system.
- ⚙️ **Customizable**: In-app TUI for remapping keybindings (saved to `config.toml`).
- 📂 **File Opening**: Open files instantly with your system's default application.

## Installation

### 🦀 From Crates.io (Recommended)
If you have Rust installed, simply run:
```bash
cargo install xplore-cli
```

### 📦 Pre-built Binaries
Download the latest binary for your operating system from the [Releases](https://github.com/DarjaGFX/Xplore/releases) page. Move the `xplore` binary to your system's PATH (e.g., `/usr/local/bin`).

### 🛠️ From Source
```bash
git clone https://github.com/DarjaGFX/Xplore.git
cd Xplore
cargo install --path .
```

## Quick Start
1. Run `xplore`.
2. Navigate with `j`/`k` or arrows.
3. Press `e` to edit a file's description.
4. Press `F3` for a global system search.
5. Press `s` to customize your keybindings.

## Metadata Warning
Descriptions are stored in `user.xplore.description` xattrs. While Xplore's internal move/copy operations preserve this metadata, regular system tools or moving files to incompatible filesystems (like FAT32) may strip these attributes.

## License
MIT
