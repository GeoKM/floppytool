// src/image_types/img.rs
use crate::{FormatHandler, Geometry};
use anyhow::{Result, anyhow};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use crate::disk_formats::{self, DiskFormat};

pub struct IMGHandler {
    data: Vec<u8>,
}

impl IMGHandler {
    pub fn new(data: Vec<u8>) -> Self {
        IMGHandler { data }
    }

    fn infer_geometry(&self) -> Result<&'static DiskFormat> {
        disk_formats::infer_format(self.data.len())
            .ok_or_else(|| anyhow!(
                "No suitable geometry found for file size {} bytes. Specify with --geometry (e.g., '40,2,9,512,5' for 360KB).",
                self.data.len()
            ))
    }
}

impl FormatHandler for IMGHandler {
    fn display(&self, ascii: bool) -> Result<String> {
        let format = self.infer_geometry()?;
        crate::core::display(&self.data, format, ascii)
    }

    fn convert(&self, target: &dyn FormatHandler, output_path: &PathBuf, _input_path: &PathBuf, _meta_path: Option<&PathBuf>, geometry: Option<Geometry>, verbose: bool, validate: bool) -> Result<()> {
        if target.data().len() == 0 { // Conversion to IMD
            let format = match geometry {
                Some(Geometry::Manual { cylinders, heads, sectors_per_track, sector_size, mode }) => {
                    DiskFormat { cylinders, heads, sectors_per_track, sector_size, mode, name: "Custom" }
                }
                _ => *self.infer_geometry()?,
            };

            let raw_data = crate::core::convert_to_raw(&self.data, &format, verbose, false)?; // is_imd: false
            let mut file = File::create(output_path)?;
            file.write_all(&raw_data)?;

            if verbose {
                println!("Converted IMG to IMD: {} bytes written", raw_data.len());
            }
            if validate {
                let mut output_file = File::open(output_path)?;
                let mut output_data = Vec::new();
                output_file.read_to_end(&mut output_data)?;
                if output_data.len() != raw_data.len() {
                    return Err(anyhow!("Validation failed: Output size {} does not match written size {}", output_data.len(), raw_data.len()));
                }
                println!("Validation passed: Output matches written data");
            }
            Ok(())
        } else {
            Err(anyhow!("Conversion from IMG only supports IMD currently"))
        }
    }

    fn data(&self) -> &[u8] {
        &self.data
    }

    fn geometry(&self) -> Result<Option<Geometry>> {
        let format = self.infer_geometry()?;
        Ok(Some(Geometry::Manual {
            cylinders: format.cylinders,
            heads: format.heads,
            sectors_per_track: format.sectors_per_track,
            sector_size: format.sector_size,
            mode: format.mode,
        }))
    }
}
