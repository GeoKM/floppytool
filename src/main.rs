use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

trait FormatHandler: Send + Sync {
    fn display(&self, ascii: bool) -> Result<String>;
    fn convert(&self, target: &dyn FormatHandler, output_path: &PathBuf, geometry: Option<Geometry>, verbose: bool, validate: bool) -> Result<()>;
    fn data(&self) -> &[u8];
    fn geometry(&self) -> Result<Option<Geometry>>;
}

fn load_handler(file_path: &PathBuf) -> Result<Box<dyn FormatHandler>> {
    let ext = file_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .ok_or_else(|| anyhow!("No file extension"))?;

    let mut file = File::open(file_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    match ext.as_str() {
        "imd" => Ok(Box::new(formats::imd::IMDHandler::new(data))),
        "img" => Ok(Box::new(formats::img::IMGHandler::new(data))),
        _ => Err(anyhow!("Unsupported format: {}", ext)),
    }
}

#[derive(Parser)]
#[command(about = "Floppy Disk Image Utility")]
struct Cli {
    #[arg(short, long)]
    input: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Display {
        #[arg(long)]
        ascii: bool,
    },
    Convert {
        #[arg(long)]
        format: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, value_parser = parse_geometry, default_value = "auto")]
        geometry: Geometry,
        #[arg(long)]
        verbose: bool,
        #[arg(long)]
        validate: bool,
    },
}

fn parse_geometry(s: &str) -> Result<Geometry, String> {
    if s == "auto" {
        Ok(Geometry::Auto)
    } else {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 5 {
            return Err("Geometry must be 'cylinders,heads,sectors,size,mode' (e.g., '40,2,9,512,4')".to_string());
        }
        Ok(Geometry::Manual {
            cylinders: parts[0].parse().map_err(|e| format!("Invalid cylinders: {}", e))?,
            heads: parts[1].parse().map_err(|e| format!("Invalid heads: {}", e))?,
            sectors_per_track: parts[2].parse().map_err(|e| format!("Invalid sectors: {}", e))?,
            sector_size: parts[3].parse().map_err(|e| format!("Invalid sector size: {}", e))?,
            mode: parts[4].parse().map_err(|e| format!("Invalid mode: {}", e))?,
        })
    }
}

#[derive(Debug, Clone)]
enum Geometry {
    Auto,
    Manual { cylinders: u8, heads: u8, sectors_per_track: u8, sector_size: u16, mode: u8 },
}

mod formats;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let handler = load_handler(&cli.input)?;

    match cli.command {
        Commands::Display { ascii } => println!("{}", handler.display(ascii)?),
        Commands::Convert { format, output, geometry, verbose, validate } => {
            let target: Box<dyn FormatHandler> = match format.as_str() {
                "img" => Box::new(formats::img::IMGHandler::new(Vec::new())) as Box<dyn FormatHandler>,
                "imd" => Box::new(formats::imd::IMDHandler::new(Vec::new())) as Box<dyn FormatHandler>,
                _ => return Err(anyhow!("Unknown target format: {}", format)),
            };
            let effective_geometry = match geometry.clone() {
                Geometry::Auto => handler.geometry()?.unwrap_or(Geometry::Manual {
                    cylinders: 40, heads: 2, sectors_per_track: 9, sector_size: 512, mode: 5
                }),
                g => g,
            };
            handler.convert(&*target, &output, Some(effective_geometry.clone()), verbose, validate)?;
            if format == "img" {
                if let Some(Geometry::Manual { cylinders, heads, sectors_per_track, sector_size, mode }) = handler.geometry()? {
                    println!("Geometry for reverse conversion: {},{},{},{},{}", cylinders, heads, sectors_per_track, sector_size, mode);
                }
            } else if format == "imd" && matches!(geometry, Geometry::Auto) {
                println!("Error: Conversion to .imd requires explicit geometry (e.g., '--geometry 40,2,9,512,4')");
                std::process::exit(1);
            }
            if validate {
                let output_handler = load_handler(&output)?;
                let output_data = output_handler.data();
                let input_data = handler.data();
                if format == "img" {
                    let expected_size = match effective_geometry {
                        Geometry::Manual { cylinders, heads, sectors_per_track, sector_size, .. } => {
                            cylinders as usize * heads as usize * sectors_per_track as usize * sector_size as usize
                        }
                        _ => return Err(anyhow!("Validation requires explicit geometry")),
                    };
                    if output_data.len() != expected_size {
                        return Err(anyhow!("Validation failed: Output size {} does not match expected geometry size {}", output_data.len(), expected_size));
                    }
                    if output_data.len() != input_data.len() {
                        println!("Warning: Output size {} differs from input size {} due to compression in .imd", output_data.len(), input_data.len());
                    }
                    println!("Validation passed: Output size matches expected geometry");
                } else {
                    if output_data.len() != input_data.len() {
                        println!("Warning: Output size {} differs from input size {} due to compression in .imd", output_data.len(), input_data.len());
                    }
                    println!("Validation passed: Output matches expected geometry");
                }
            }
            println!("Converted to {}", output.display());
        }
    }
    Ok(())
}
