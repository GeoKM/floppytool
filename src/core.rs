// src/core.rs
use crate::disk_formats::DiskFormat;
use anyhow::Result;

pub fn display(data: &[u8], format: &DiskFormat, ascii: bool) -> Result<String> {
    let mut output = Vec::new();
    output.push(format!("Raw IMG: {} bytes", data.len())); // Match img.rs output
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
