use crate::{FormatHandler, Geometry};
use anyhow::{Result, anyhow};
use byteorder::ReadBytesExt;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

pub struct IMDHandler {
    data: Vec<u8>,
}

impl IMDHandler {
    pub fn new(data: Vec<u8>) -> Self { IMDHandler { data } }

    fn analyze_geometry(&self) -> Result<(u8, u8, u8, u16, u8)> {
        let header_end = self.data.iter().position(|&b| b == 0x1A)
            .ok_or_else(|| anyhow!("No header terminator found"))?;
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
        Ok((max_cyl, max_head, sectors_per_track, sector_size, mode))
    }
}

impl FormatHandler for IMDHandler {
    fn display(&self, ascii: bool) -> Result<String> {
        let mut output = Vec::new();
        let header_end = self.data.iter().position(|&b| b == 0x1A)
            .ok_or_else(|| anyhow!("No header terminator found"))?;
        let header = String::from_utf8_lossy(&self.data[..header_end]);
        output.push(format!("Header: {}", header));

        let (cylinders, heads, sectors_per_track, sector_size, mode) = self.analyze_geometry()?;
        let total_size = cylinders as usize * heads as usize * sectors_per_track as usize * sector_size as usize;

        if !ascii {
            output.push(format!("Raw Size: {} bytes", total_size));
            output.push(format!(
                "Detected Geometry: {} cylinders, {} heads, {} sectors/track, {} bytes/sector, mode {}",
                cylinders, heads, sectors_per_track, sector_size, mode
            ));
        } else {
            let mut cursor = Cursor::new(&self.data[header_end + 1..]);
            while cursor.position() < self.data.len() as u64 - header_end as u64 - 1 {
                let mode = cursor.read_u8()?;
                let cylinder = cursor.read_u8()?;
                let head = cursor.read_u8()?;
                let sector_count = cursor.read_u8()?;
                let sector_size_code = cursor.read_u8()?;
                let sector_size = 128 << sector_size_code;

                let skip_bytes = sector_count as u64
                    + if head & 0x80 != 0 { sector_count as u64 } else { 0 }
                    + if head & 0x40 != 0 { sector_count as u64 } else { 0 };
                cursor.set_position(cursor.position() + skip_bytes);

                for sector in 1..=sector_count {
                    let type_byte = cursor.read_u8()?;
                    let mut sector_data = Vec::new();
                    match type_byte {
                        1 => {
                            sector_data.resize(sector_size as usize, 0);
                            cursor.read_exact(&mut sector_data)?;
                        }
                        2 => {
                            let value = cursor.read_u8()?;
                            sector_data = vec![value; sector_size as usize];
                        }
                        _ => return Err(anyhow!("Unsupported sector type: {}", type_byte)),
                    }
                    let ascii_str: String = sector_data.iter()
                        .take(32)
                        .map(|&b| if b >= 32 && b <= 126 { b as char } else { '.' })
                        .collect();
                    output.push(format!(
                        "Cyl {}, Head {}, Sector {}, Size {} bytes, Mode {}: {}",
                        cylinder, head, sector, sector_size, mode, ascii_str
                    ));
                }
            }
        }
        Ok(output.join("\n"))
    }

    fn convert(&self, target: &dyn FormatHandler, output_path: &PathBuf, input_path: &PathBuf, meta_path: Option<&PathBuf>, _geometry: Option<Geometry>, verbose: bool, validate: bool) -> Result<()> {
        if target.data().len() == 0 { // IMG conversion
            let mut raw_data = Vec::new();
            let header_end = self.data.iter().position(|&b| b == 0x1A).unwrap();
            let header = &self.data[..header_end + 1]; // Include 0x1A
            let mut cursor = Cursor::new(&self.data[header_end + 1..]);
            let mut total_sectors = 0;
            let mut total_compressed = 0;
            let mut sector_size = 0;
            let mut sector_ids_map = Vec::new();

            while cursor.position() < self.data.len() as u64 - header_end as u64 - 1 {
                let mode = cursor.read_u8()?;
                let cylinder = cursor.read_u8()?;
                let head = cursor.read_u8()?;
                let sector_count = cursor.read_u8()?;
                let sector_size_code = cursor.read_u8()?;
                let current_sector_size = 128 << sector_size_code;
                if sector_size == 0 { sector_size = current_sector_size; }

                // Read sector ID map
                let mut sector_ids = Vec::new();
                for _ in 0..sector_count {
                    sector_ids.push(cursor.read_u8()?);
                }
                sector_ids_map.push((cylinder, head, sector_ids.clone()));
                let skip_bytes = if head & 0x80 != 0 { sector_count as u64 } else { 0 }
                    + if head & 0x40 != 0 { sector_count as u64 } else { 0 };
                cursor.set_position(cursor.position() + skip_bytes);

                // Store sector data in correct order
                let mut track_data = vec![vec![0u8; current_sector_size as usize]; sector_count as usize];
                let mut normal_sectors = 0;
                let mut compressed_sectors = 0;

                for i in 0..sector_count {
                    let type_byte = cursor.read_u8()?;
                    let sector_idx = (sector_ids[i as usize] - 1) as usize; // 1-based to 0-based
                    match type_byte {
                        1 => {
                            cursor.read_exact(&mut track_data[sector_idx])?;
                            normal_sectors += 1;
                        }
                        2 => {
                            let value = cursor.read_u8()?;
                            track_data[sector_idx].fill(value);
                            compressed_sectors += 1;
                        }
                        _ => {
                            if verbose {
                                println!("Skipping unsupported sector type {} in Cyl {}, Head {}", type_byte, cylinder, head);
                            }
                            cursor.set_position(cursor.position() + current_sector_size as u64);
                        }
                    }
                }

                // Append in sequential order for .img
                for sector_data in track_data {
                    raw_data.extend_from_slice(&sector_data);
                }

                total_sectors += sector_count as usize;
                total_compressed += compressed_sectors;

                if verbose {
                    println!(
                        "Processing Cyl {}, Head {}: {} sectors ({} normal, {} compressed), size {} bytes, mode {}",
                        cylinder, head, sector_count, normal_sectors, compressed_sectors, current_sector_size, mode
                    );
                }
            }

            let mut file = File::create(output_path)?;
            file.write_all(&raw_data)?;

            // Save header and sector IDs using input file stem unless meta_path is provided
            let default_meta_path = input_path.with_extension("imd.meta");
            let meta_path = meta_path.unwrap_or(&default_meta_path);
            let mut meta_file = File::create(meta_path)?;
            meta_file.write_all(header)?;
            for (cyl, head, ids) in sector_ids_map {
                meta_file.write_all(&[cyl, head, ids.len() as u8])?;
                meta_file.write_all(&ids)?;
            }

            if verbose {
                println!("Total sectors: {}, Compressed sectors: {}", total_sectors, total_compressed);
                println!("Saved metadata to {}", meta_path.display());
            }

            if validate {
                let expected_size = total_sectors * sector_size as usize;
                if raw_data.len() != expected_size {
                    return Err(anyhow!("Validation failed: Output size {} bytes does not match expected {} bytes", raw_data.len(), expected_size));
                }
                println!("Validation passed: Output size matches expected geometry");
            }

            Ok(())
        } else {
            Err(anyhow!("Conversion to this format not implemented"))
        }
    }

    fn geometry(&self) -> Result<Option<Geometry>> {
        let (cylinders, heads, sectors_per_track, sector_size, mode) = self.analyze_geometry()?;
        Ok(Some(Geometry::Manual { cylinders, heads, sectors_per_track, sector_size, mode }))
    }

    fn data(&self) -> &[u8] { &self.data }
}
