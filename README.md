```markdown
# floppytool

A command-line utility for converting and inspecting floppy disk images, built with Rust for retro computing enthusiasts. Currently supports `.imd` and `.img` formats, with an extensible design for adding more.

## Features
- Convert between `.imd` (ImageDisk) and `.img` (raw floppy image) formats.
- Display disk geometry and sector details.
- Optional verbose output and validation checks.
- ASCII view of sector data with `--ascii`.

## Supported Formats
- **`.img`**: Raw floppy disk images (e.g., 1.44MB, 1.2MB), no metadata or compression.
- **`.imd`**: ImageDisk format, includes metadata and optional compression for efficient storage.

## Installation

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (latest stable version recommended).

### Build from Source
1. Clone the repository:
   ```bash
   git clone https://github.com/GeoKM/floppytool.git
   cd floppytool
   ```
2. Build the release binary:
   ```bash
   cargo build --release
   ```
   The executable will be at `target/release/floppytool`.

### Optional: Install Globally
To install `floppytool` to your Cargo bin directory (e.g., `~/.cargo/bin`):
```bash
cargo install --path .
```
Then run it from anywhere with `floppytool`.

## Usage

Run commands with the built binary from the project root (`./target/release/floppytool`) or globally if installed.

### Display Disk Information
- **Basic Geometry**:
  ```bash
  ./target/release/floppytool --input filename.imd display
  ```
  Shows disk geometry (e.g., cylinders, heads, sectors).

- **With ASCII Sector Data**:
  ```bash
  ./target/release/floppytool --input filename.imd display --ascii
  ```
  Displays the first 32 bytes of each sector as ASCII characters.

### Convert Formats
- **`.imd` to `.img`**:
  ```bash
  ./target/release/floppytool --input filename.imd convert --format img --output filename.img --verbose --validate
  ```
  Outputs geometry for reverse conversion (e.g., `40,2,9,512,4`).

- **`.img` to `.imd`**:
  ```bash
  ./target/release/floppytool --input filename.img convert --format imd --output newfilename.imd --geometry 40,2,9,512,4 --verbose --validate
  ```
  Use geometry from a prior `.imd` conversion or specify manually.

### Command Options
| Option         | Description                                      | Subcommand   | Default    |
|-----------------|--------------------------------------------------|--------------|------------|
| `--ascii`      | Show sector data as ASCII characters            | `display`    | `false`    |
| `--format`     | Target format (e.g., `img`, `imd`)              | `convert`    | Required   |
| `--output`     | Output file path                                | `convert`    | Required   |
| `--geometry`   | Geometry as `cyl,heads,sect,size,mode` or `auto`| `convert`    | `auto`     |
| `--verbose`    | Show detailed conversion progress               | `convert`    | `false`    |
| `--validate`   | Check output integrity                          | `convert`    | `false`    |

## Examples

### Round-Trip Conversion
Convert a 360KB `.imd` to `.img` and back, verifying integrity:
```bash
./target/release/floppytool --input LAPLINK3.IMD convert --format img --output test.img --verbose --validate
./target/release/floppytool --input test.img convert --format imd --output test.imd --geometry 40,2,9,512,4 --verbose --validate
cmp -l LAPLINK3.IMD test.imd  # Should show no differences
```

### Inspect a 1.44MB Floppy
View sector contents of a raw image:
```bash
./target/release/floppytool --input floppy144.img display --ascii
```

### Convert a 1.2MB Floppy
Convert with auto-detected geometry:
```bash
./target/release/floppytool --input disk12.img convert --format imd --output disk12.imd --verbose
```

## Notes
- **`.img` Files**: Raw images with no metadata; size implies geometry (e.g., 1,440,000 bytes = 80×2×18×512).
- **`.imd` Files**: Include metadata and compression; output size may differ from input due to compression.
- **Validation**: Warns about size differences but doesn’t fail—useful for checking compression effects.

## Contributing
Contributions are welcome! To add new formats (e.g., `.td0`, `.dsk`), implement the `FormatHandler` trait in `src/formats/`. Submit a pull request or open an issue with ideas.

## License
Licensed under the MIT License. See [LICENSE](./LICENSE) for details.

Copyright (c) 2025 Keith Matthews
```
