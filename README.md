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


Usage

Run commands from the project root with the built binary.


Display Disk Information

Basic geometry:
   ```bash
       ./target/release/floppytool --input filename.imd display

With ASCII sector data (first 32 bytes):
   ```bash

    ./target/release/floppytool --input LAPLINK3.IMD display --ascii

Convert Formats

    .imd to .img:
   ```bash

./target/release/floppytool --input LAPLINK3.IMD convert --format img --output test.img --verbose --validate

    Outputs geometry for reverse conversion (e.g., 40,2,9,512,4).

   .img to .imd (requires geometry from previous step):
  ```bash

    ./target/release/floppytool --input test.img convert --format imd --output test.imd --geometry 40,2,9,512,4 --verbose --validate

Options

    --verbose: Show detailed conversion progress.
    --validate: Check output integrity (warns on size differences due to compression).
    --ascii: Display sector data in ASCII (for display command).

Examples

    Round-trip conversion:
    ```bash

./target/release/floppytool --input LAPLINK3.IMD convert --format img --output test.img --verbose --validate
./target/release/floppytool --input test.img convert --format imd --output test.imd --geometry 40,2,9,512,4 --verbose --validate
cmp -l LAPLINK3.IMD test.imd  # Should show no differences
Inspect sector contents:
    ```bash

    ./target/release/floppytool --input test.img display --ascii

Notes

    .img files are raw images; .imd files include metadata and compression.
    Validation warns about size differences due to compression but does not fail.

License

Licensed under the MIT License. See LICENSE for details.

Copyright (c) 2025 Keith Matthews
