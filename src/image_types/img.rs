use crate::{FormatHandler, Geometry};
use anyhow::{Result, anyhow};
use std::fs::File;
use std::io::{Read, Write, Cursor};
use byteorder::ReadBytesExt;
use std::path::PathBuf;

pub struct IMGHandler {
    data: Vec<u8>,
}

impl IMGHandler {
    pub fn new(data: Vec<u8>) -> Self {
        IMGHandler { data }
    }

    fn infer_geometry(&self) -> Result<(u8, u8, u8, u16, u8)> {
        let size = self.data.len();

        let formats = [
            (360_000, 40, 2, 9, 5),
            (720_000, 80, 2, 9, 5),
            (1_228_800, 80, 2, 15, 4),
            (1_474_560, 80, 2, 18, 5),
        ];

        for &(expected_size, cyl, heads, spt, mode) in &formats {
            if size == expected_size {
                return Ok((cyl, heads, spt, 512, mode));
            }
        }

        if size == 368_640 {
            return Ok((40, 2, 9, 512, 5));
        }

        if size % 512 == 0 {
            let total_sectors = size / 512;
            for cyl in (40..=80).rev() {
                for heads in (1..=2).rev() {
                    let spt = total_sectors / (cyl * heads);
                    if spt * cyl * heads == total_sectors && spt <= 36 {
                        return Ok((cyl as u8, heads as u8, spt as u8, 512, 5));
                    }
                }
            }
        }

        Err(anyhow!(
            "No suitable geometry found for file size {} bytes. Specify with --geometry (e.g., '40,2,9,512,5' for 360KB, '80,2,18,512,5' for 1.44MB). Common sizes: 360KB, 720KB, 1.2MB, 1.44MB.",
            size
        ))
    }
}

impl FormatHandler for IMGHandler {
    fn display(&self, ascii: bool) -> Result<String> {
        let size = self.data.len();
        let (cylinders, heads, sectors_per_track, sector_size, mode) = self.infer_geometry()?;
        let mut output = Vec::new();

        output.push(format!("Raw IMG: {} bytes", size));
        if !ascii {
            output.push(format!(
                "Detected Geometry: {} cylinders, {} heads, {} sectors/track, {} bytes/sector",
                cylinders, heads, sectors_per_track, sector_size
            ));
            output.push(format!(
                "Note: Mode is not stored in .img files; inferred mode {} (common modes for this size: 4 or 5)",
                mode
            ));
        } else {
            let mut pos = 0;
            for cyl in 0..cylinders {
                for head in 0..heads {
                    for sector in 1..=sectors_per_track {
                        let chunk = &self.data[pos..pos + sector_size as usize];
                        let ascii_str: String = chunk.iter()
                            .take(32)
                            .map(|&b| if b >= 32 && b <= 126 { b as char } else { '.' })
                            .collect();
                        output.push(format!(
                            "Cyl {}, Head {}, Sector {}, Size {} bytes, Mode {}: {}",
                            cyl, head, sector, sector_size, mode, ascii_str
                        ));
                        pos += sector_size as usize;
                    }
                }
            }
        }
        Ok(output.join("\n"))
    }

    fn convert(&self, target: &dyn FormatHandler, output_path: &PathBuf, input_path: &PathBuf, meta_path: Option<&PathBuf>, geometry: Option<Geometry>, verbose: bool, validate: bool) -> Result<()> {
        if target.data().len() == 0 { // Conversion to IMD
            let (cylinders, heads, sectors_per_track, sector_size, mode) = match geometry {
                Some(Geometry::Manual { cylinders, heads, sectors_per_track, sector_size, mode }) => {
                    (cylinders, heads, sectors_per_track, sector_size, mode)
                }
                _ => self.infer_geometry()?,
            };

            let expected_size = cylinders as usize * heads as usize * sectors_per_track as usize * sector_size as usize;
            if expected_size != self.data.len() {
                return Err(anyhow!(
                    "Geometry {}x{}x{}x{} ({} bytes) does not match file size ({} bytes)",
                    cylinders, heads, sectors_per_track, sector_size, expected_size, self.data.len()
                ));
            }

            let mut raw_data = Vec::new();
            let mut sector_ids_map = Vec::new();

            // Load header and sector IDs from meta_path if provided, else fall back to input-based default
            let default_meta_path = input_path.with_extension("imd.meta");
            let meta_path = meta_path.unwrap_or(&default_meta_path);
            if meta_path.exists() {
                let mut meta_file = File::open(meta_path)?;
                let mut meta_data = Vec::new();
                meta_file.read_to_end(&mut meta_data)?;
                let header_end = meta_data.iter().position(|&b| b == 0x1A).unwrap();
                raw_data.extend_from_slice(&meta_data[..header_end + 1]);
                
                let mut cursor = Cursor::new(&meta_data[header_end + 1..]);
                while cursor.position() < meta_data.len() as u64 - header_end as u64 - 1 {
                    let cyl = cursor.read_u8()?;
                    let head = cursor.read_u8()?;
                    let count = cursor.read_u8()?;
                    let mut ids = Vec::new();
                    for _ in 0..count {
                        ids.push(cursor.read_u8()?);
                    }
                    sector_ids_map.push((cyl, head, ids));
                }
                if verbose {
                    println!("Loaded metadata from {}", meta_path.display());
                }
            } else {
                raw_data.extend(b"IMD 1.18 - floppytool\n\x1A");
                if verbose {
                    println!("No metadata found at {}; using default header", meta_path.display());
                }
            }

            let mut pos = 0;
            let mut total_sectors = 0;
            let mut total_compressed = 0;

            for cyl in 0..cylinders {
                for head in 0..heads {
                    raw_data.push(mode);
                    raw_data.push(cyl);
                    raw_data.push(head);
                    raw_data.push(sectors_per_track);
                    raw_data.push(match sector_size { 128 => 0, 256 => 1, 512 => 2, 1024 => 3, 2048 => 4, 4096 => 5, _ => 2 });

                    // Use original sector IDs if available
                    let sector_ids = sector_ids_map.iter()
                        .find(|&&(c, h, _)| c == cyl && h == head)
                        .map(|(_, _, ids)| ids.clone())
                        .unwrap_or_else(|| (1..=sectors_per_track).collect());
                    for s in sector_ids { // Removed * here
                        raw_data.push(s);
                    }

                    let mut normal_sectors = 0;
                    let mut compressed_sectors = 0;

                    for _ in 0..sectors_per_track {
                        let chunk = &self.data[pos..pos + sector_size as usize];
                        if chunk.iter().all(|&b| b == chunk[0]) {
                            raw_data.push(2); // Compressed
                            raw_data.push(chunk[0]);
                            compressed_sectors += 1;
                        } else {
                            raw_data.push(1); // Normal data
                            raw_data.extend_from_slice(chunk);
                            normal_sectors += 1;
                        }
                        pos += sector_size as usize;
                    }

                    total_sectors += sectors_per_track as usize;
                    total_compressed += compressed_sectors;

                    if verbose {
                        println!(
                            "Writing Cyl {}, Head {}: {} sectors ({} normal, {} compressed), size {} bytes, mode {}",
                            cyl, head, sectors_per_track, normal_sectors, compressed_sectors, sector_size, mode
                        );
                    }
                }
            }

            let mut file = File::create(output_path)?;
            file.write_all(&raw_data)?;

            if verbose {
                println!("Total sectors: {}, Compressed sectors: {}", total_sectors, total_compressed);
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

    fn geometry(&self) -> Result<Option<Geometry>> {
        let (cylinders, heads, sectors_per_track, sector_size, mode) = self.infer_geometry()?;
        Ok(Some(Geometry::Manual { cylinders, heads, sectors_per_track, sector_size, mode }))
    }

    fn data(&self) -> &[u8] {
        &self.data
    }
}
