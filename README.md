# floppytool

A command-line utility for converting and inspecting floppy disk images, supporting `.imd` and `.img` formats. Built with Rust for retro computing enthusiasts.

## Features
- Convert between `.imd` (ImageDisk) and `.img` (raw floppy image) formats.
- Display disk geometry and sector details.
- Optional verbose output and validation checks.
- ASCII view of sector data with `--ascii`.

## Installation
1. Ensure [Rust](https://www.rust-lang.org/tools/install) is installed.
2. Clone the repository:
   ```bash
   git clone <your-repo-url-here>
   cd floppytool
3. Build the release binary:
   ```bash
   cargo build --release

Usage

Run commands from the project root with the built binary.

The executable will be at target/release/floppytool.


Display Disk Information

Basic geometry:
   ```bash
       ./target/release/floppytool --input filename.imd display

test:
   ```bash
       ./
