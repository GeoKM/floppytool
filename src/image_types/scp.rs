// src/image_types/scp.rs
use crate::{FormatHandler, Geometry};
use anyhow::{Result, anyhow};
use std::io::{Cursor, Read};
use byteorder::{LittleEndian, ReadBytesExt};
use std::path::PathBuf; // Added this line
use crate::disk_formats::{DiskFormat, IBM_1_44M};

pub struct SCPHandler {
    data: Vec<u8>,
}

impl SCPHandler {
    pub fn new(data: Vec<u8>) -> Self {
        SCPHandler { data }
    }

    fn infer_geometry(&self) -> Result<DiskFormat> {
        if self.data.len() < 16 {
            return Err(anyhow!("File too short: {} bytes. Expected at least 16 bytes for header.", self.data.len()));
        }
        let mut cursor = Cursor::new(&self.data);
        let mut magic = [0u8; 3];
        cursor.read_exact(&mut magic)?;
        if magic != *b"SCP" {
            return Err(anyhow!("Invalid .scp file: Magic bytes not 'SCP'"));
        }

        let _version = cursor.read_u8()?;
        let disk_type = cursor.read_u8()?;
        let _revolutions = cursor.read_u8()?;
        let _start_track = cursor.read_u8()?;
        let _end_track = cursor.read_u8()?;
        let _flags = cursor.read_u8()?;
        let _bit_cell_width = cursor.read_u8()?;
        let _heads = cursor.read_u8()?;
        let _resolution = cursor.read_u8()?;
        let _checksum = cursor.read_u32::<LittleEndian>()?;

        // For now, only handle PC 1.44M variants; expand later
        if disk_type == 0x33 || disk_type == 0x80 || disk_type == 0x35 {
            Ok(IBM_1_44M) // Use predefined 1.44M format
        } else {
            Err(anyhow!(
                "Unsupported SCP disk type 0x{:02X}. Only PC 1.44M (0x33, 0x80, 0x35) supported currently.",
                disk_type
            ))
        }
    }
}

impl FormatHandler for SCPHandler {
    fn display(&self, ascii: bool) -> Result<String> {
        let format = self.infer_geometry()?;
        crate::core::display(&self.data, &format, ascii)
    }

    fn convert(&self, _target: &dyn FormatHandler, _output_path: &PathBuf, _input_path: &PathBuf, _meta_path: Option<&PathBuf>, _geometry: Option<Geometry>, _verbose: bool, _validate: bool) -> Result<()> {
        Err(anyhow!("Conversion from .scp not yet implemented"))
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
