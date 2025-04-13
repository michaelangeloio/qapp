# qapp

A simple CLI tool to manage open macOS applications from the terminal.

## Features

- Interactive list of running applications with keyboard navigation
- Open or kill applications directly from the interactive list with keyboard shortcuts
- Open applications with fuzzy search
- Terminate (kill) running applications with fuzzy search

## Installation

### Option 1: From this repository

Make sure you have Rust installed. Then clone this repository and build:

```bash
git clone <your-repo-url>
cd qapp
cargo build --release
```

The binary will be available at `./target/release/qapp`.

For convenience, you can install it to your PATH:

```bash
cargo install --path .
```

### Option 2: Quick start (current directory)

If you already have the code, simply run:

```bash
# Build the release version
cargo build --release

# Run directly
./target/release/qapp

# Or install to your PATH
cargo install --path .
```

## Usage

### Interactive list of running applications

```bash
qapp list
# or simply
qapp
```

This will show an interactive list of all running applications where you can:
- Navigate up/down with arrow keys
- Press 'O' to open/focus the selected application
- Press 'K' to kill/quit the selected application
- Press 'Q' or Esc to quit

### Open an application

```bash
# Open by name
qapp open "Safari"

# Open with interactive fuzzy search
qapp open
```

### Kill (terminate) an application

```bash
# Kill by name
qapp kill "Safari"

# Kill with interactive fuzzy search
qapp kill
```

## Requirements

- macOS
- Rust 1.70+

## How it works

This tool uses AppleScript via the `osascript` command to interact with macOS applications. It's designed to be user-friendly with color output and fuzzy search capabilities.