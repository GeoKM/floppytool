// src/image_types/imd.rs
use crate::{FormatHandler, Geometry};
use anyhow::{Result, anyhow};
use byteorder::ReadBytesExt;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::fs::File;
use crate::disk_formats::DiskFormat;

pub struct IMDHandler {
    data: Vec<u8>,
}

impl IMDHandler {
    pub fn new(data: Vec<u8>) -> Self { IMDHandler { data } }

    fn analyze_geometry(&self) -> Result<DiskFormat> {
        let header_end = self.data.iter().position(|&b| b == 0x1A)
            .ok_or_else(|| anyhow!(
                "Invalid .imd file: No header terminator (0x1A) found. The file may be corrupted or not in ImageDisk format."
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
            name: "IMD Custom", // Placeholder; could infer from disk_formats later
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
            let mut raw_data = Vec::new();
            let header_end = self.data.iter().position(|&b| b == 0x1A).unwrap();
            let header = &self.data[..header_end + 1];
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

                let mut sector_ids = Vec::new();
                for _ in 0..sector_count {
                    sector_ids.push(cursor.read_u8()?);
                }
                sector_ids_map.push((cylinder, head, sector_ids.clone()));
                let skip_bytes = if head & 0x80 != 0 { sector_count as u64 } else { 0 }
                    + if head & 0x40 != 0 { sector_count as u64 } else { 0 };
                cursor.set_position(cursor.position() + skip_bytes);

                let mut track_data = vec![vec![0u8; current_sector_size as usize]; sector_count as usize];
                let mut normal_sectors = 0;
                let mut compressed_sectors = 0;

                for i in 0..sector_count {
                    let type_byte = cursor.read_u8()?;
                    let sector_idx = (sector_ids[i as usize] - 1) as usize;
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
