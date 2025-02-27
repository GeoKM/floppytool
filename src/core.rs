// src/core.rs
use crate::disk_formats::DiskFormat;
use anyhow::{Result, anyhow};
use std::io::{Write, Cursor, Read};
use byteorder::ReadBytesExt;

pub fn display(data: &[u8], format: &DiskFormat, ascii: bool) -> Result<String> {
    let mut output = Vec::new();
    output.push(format!("Raw IMG: {} bytes", data.len()));
    if !ascii {
        output.push(format!(
            "Detected Format: {}\nGeometry: {} cylinders, {} heads, {} sectors/track, {} bytes/sector, mode {}",
            format.name, format.cylinders, format.heads, format.sectors_per_track, format.sector_size, format.mode
        ));
    } else {
        let mut pos = 0;
        for cyl in 0..format.cylinders {
            for head in 0..format.heads {
                for sector in 1..=format.sectors_per_track {
                    let chunk = &data[pos..pos + format.sector_size as usize];
                    let ascii_str: String = chunk.iter()
                        .take(32)
                        .map(|&b| if b >= 32 && b <= 126 { b as char } else { '.' })
                        .collect();
                    output.push(format!(
                        "Cyl {}, Head {}, Sector {}, Size {} bytes, Mode {}: {}",
                        cyl, head, sector, format.sector_size, format.mode, ascii_str
                    ));
                    pos += format.sector_size as usize;
                }
            }
        }
    }
    Ok(output.join("\n"))
}

pub fn convert_to_raw(data: &[u8], format: &DiskFormat, verbose: bool, is_imd: bool) -> Result<Vec<u8>> {
    let expected_size = format.total_size();
    let mut raw_data = Vec::with_capacity(expected_size);

    if is_imd {
        let header_end = data.iter().position(|&b| b == 0x1A)
            .ok_or_else(|| anyhow!("Invalid .imd file: No header terminator (0x1A) found."))?;
        let mut cursor = Cursor::new(&data[header_end + 1..]);
        let mut total_sectors = 0;
        let mut total_compressed = 0;

        while cursor.position() < data.len() as u64 - header_end as u64 - 1 {
            let _mode = cursor.read_u8()?;
            let cylinder = cursor.read_u8()?;
            let head = cursor.read_u8()?;
            let sector_count = cursor.read_u8()?;
            let sector_size_code = cursor.read_u8()?;
            let sector_size = match sector_size_code {
                0 => 128,
                1 => 256,
                2 => 512,
                3 => 1024,
                4 => 2048,
                5 => 4096,
                _ => return Err(anyhow!("Invalid sector size code: {}", sector_size_code)),
            };

            let mut sector_ids = Vec::new();
            for _ in 0..sector_count {
                sector_ids.push(cursor.read_u8()?);
            }
            let skip_bytes = if head & 0x80 != 0 { sector_count as u64 } else { 0 }
                + if head & 0x40 != 0 { sector_count as u64 } else { 0 };
            cursor.set_position(cursor.position() + skip_bytes);

            let mut track_data = vec![vec![0u8; sector_size as usize]; sector_count as usize];
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
                    _ => return Err(anyhow!("Unsupported sector type: {}", type_byte)),
                }
            }

            for sector_data in track_data {
                raw_data.extend_from_slice(&sector_data);
            }

            total_sectors += sector_count as usize;
            total_compressed += compressed_sectors;

            if verbose {
                println!(
                    "Processing Cyl {}, Head {}: {} sectors ({} normal, {} compressed), size {} bytes",
                    cylinder, head, sector_count, normal_sectors, compressed_sectors, sector_size
                );
            }
        }

        if verbose {
            println!("Total sectors: {}, Compressed sectors: {}", total_sectors, total_compressed);
        }
    } else {
        // Raw data (e.g., .img)
        if data.len() != expected_size {
            return Err(anyhow!(
                "Data size {} bytes does not match expected {} bytes for geometry {}",
                data.len(), expected_size, format.name
            ));
        }
        raw_data.extend_from_slice(data);
    }

    Ok(raw_data)
}
