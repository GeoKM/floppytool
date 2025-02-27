// src/image_types/imd.rs
use crate::{FormatHandler, Geometry};
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::fs::File;
use std::io::{Write, Cursor};
use byteorder::ReadBytesExt;
use crate::disk_formats::DiskFormat;

pub struct IMDHandler {
    data: Vec<u8>,
}

impl IMDHandler {
    pub fn new(data: Vec<u8>) -> Self { IMDHandler { data } }

    fn analyze_geometry(&self) -> Result<DiskFormat> {
        let header_end = self.data.iter().position(|&b| b == 0x1A)
            .ok_or_else(|| anyhow!(
                "Invalid .imd file: No header terminator (0x1A) found."
            ))?;
        let mut cursor = Cursor::new(&self.data[header_end + 1..]);
        let mut max_cyl = 0;
        let mut max_head = 0;
        let mut sectors_per_track = 0;
        let mut sector_size = 0;
        let mut mode = 0;

        while cursor.position() < self.data.len() as u64 - header_end as u64 - 1 {
            let track_mode = cursor.read_u8()?;
            let cylinder = cursor.read_u8()?;
            let head = cursor.read_u8()?;
            let sector_count = cursor.read_u8()?;
            let sector_size_code = cursor.read_u8()?;
            let size = 128 << sector_size_code;

            max_cyl = max_cyl.max(cylinder + 1);
            max_head = max_head.max(head + 1);
            if sectors_per_track == 0 { sectors_per_track = sector_count; }
            if sector_size == 0 { sector_size = size; }
            if mode == 0 { mode = track_mode; }

            let skip_bytes = sector_count as u64
                + if head & 0x80 != 0 { sector_count as u64 } else { 0 }
                + if head & 0x40 != 0 { sector_count as u64 } else { 0 };
            cursor.set_position(cursor.position() + skip_bytes);

            for _ in 0..sector_count {
                let type_byte = cursor.read_u8()?;
                match type_byte {
                    1 => cursor.set_position(cursor.position() + size as u64),
                    2 => cursor.set_position(cursor.position() + 1),
                    _ => return Err(anyhow!("Unsupported sector type: {}", type_byte)),
                }
            }
        }
        Ok(DiskFormat {
            cylinders: max_cyl,
            heads: max_head,
            sectors_per_track,
            sector_size,
            mode,
            name: "IMD Custom",
        })
    }
}

impl FormatHandler for IMDHandler {
    fn display(&self, ascii: bool) -> Result<String> {
        let format = self.analyze_geometry()?;
        crate::core::display(&self.data, &format, ascii)
    }

    fn convert(&self, target: &dyn FormatHandler, output_path: &PathBuf, input_path: &PathBuf, meta_path: Option<&PathBuf>, _geometry: Option<Geometry>, verbose: bool, _validate: bool) -> Result<()> {
        if target.data().len() == 0 { // IMG conversion
            let format = self.analyze_geometry()?;
            let raw_data = crate::core::convert_to_raw(&self.data, &format, verbose, true)?; // is_imd: true

            let mut file = File::create(output_path)?;
            file.write_all(&raw_data)?;

            let header_end = self.data.iter().position(|&b| b == 0x1A).unwrap();
            let header = &self.data[..header_end + 1];
            let default_meta_path = input_path.with_extension("imd.meta");
            let meta_path = meta_path.unwrap_or(&default_meta_path);
            let mut meta_file = File::create(meta_path)?;
            meta_file.write_all(header)?;

            if verbose {
                println!("Saved metadata to {}", meta_path.display());
            }
            Ok(())
        } else {
            Err(anyhow!("Conversion to this format not implemented"))
        }
    }

    fn data(&self) -> &[u8] { &self.data }

    fn geometry(&self) -> Result<Option<Geometry>> {
        let format = self.analyze_geometry()?;
        Ok(Some(Geometry::Manual {
            cylinders: format.cylinders,
            heads: format.heads,
            sectors_per_track: format.sectors_per_track,
            sector_size: format.sector_size,
            mode: format.mode,
        }))
    }
}
