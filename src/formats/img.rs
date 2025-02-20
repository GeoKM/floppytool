use crate::{FormatHandler, Geometry}; // Re-added Geometry
use anyhow::{Result, anyhow};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub struct IMGHandler {
    data: Vec<u8>,
}

impl IMGHandler {
    pub fn new(data: Vec<u8>) -> Self {
        IMGHandler { data }
    }

    fn infer_geometry(&self) -> Vec<(u8, u8, u8, u16)> {
        let size = self.data.len();
        let mut possibles = Vec::new();

        let formats = [
            (360_000, 40, 2, 9),   // 5.25" DD 360 KB
            (720_000, 80, 2, 9),   // 3.5" DD 720 KB
            (1_200_000, 80, 2, 15), // 5.25" HD 1.2 MB
            (1_440_000, 80, 2, 18), // 3.5" HD 1.44 MB
        ];

        for &(expected_size, cyl, heads, spt) in &formats {
            if size == expected_size {
                possibles.push((cyl, heads, spt, 512));
            }
        }

        if possibles.is_empty() && size % 512 == 0 {
            let total_sectors = size / 512;
            for cyl in 40..=80 {
                for heads in 1..=2 {
                    let spt = total_sectors / (cyl * heads);
                    if spt * cyl * heads == total_sectors && spt <= 36 {
                        possibles.push((cyl as u8, heads as u8, spt as u8, 512));
                    }
                }
            }
        }

        if possibles.is_empty() {
            possibles.push((80, 2, 18, 512)); // Fallback
        }
        possibles
    }
}

impl FormatHandler for IMGHandler {
    fn display(&self) -> Result<String> {
        let size = self.data.len();
        let possibles = self.infer_geometry();
        let mut output = Vec::new();

        output.push(format!("Raw IMG: {} bytes", size));
        if possibles.len() == 1 {
            let (cyl, heads, spt, sector_size) = possibles[0];
            output.push(format!(
                "Detected Geometry: {} cylinders, {} heads, {} sectors/track, {} bytes/sector",
                cyl, heads, spt, sector_size
            ));
        } else {
            output.push("Possible Geometries:".to_string());
            for (i, &(cyl, heads, spt, sector_size)) in possibles.iter().enumerate() {
                output.push(format!(
                    "  {}. {} cylinders, {} heads, {} sectors/track, {} bytes/sector",
                    i + 1, cyl, heads, spt, sector_size
                ));
            }
            output.push("Use --geometry to specify if ambiguous".to_string());
        }
        Ok(output.join("\n"))
    }

    fn convert(&self, target: &dyn FormatHandler, output_path: &PathBuf, geometry: Option<Geometry>) -> Result<()> {
        if target.data().len() == 0 { // Conversion to IMD
            let (cylinders, heads, sectors_per_track, sector_size) = match geometry.unwrap_or(Geometry::Auto) {
                Geometry::Manual { cylinders, heads, sectors_per_track, sector_size } => {
                    (cylinders, heads, sectors_per_track, sector_size)
                }
                Geometry::Auto => {
                    let possibles = self.infer_geometry();
                    if possibles.len() != 1 {
                        return Err(anyhow!("Ambiguous geometry; specify with --geometry (e.g., '80,2,18,512')"));
                    }
                    possibles[0]
                }
            };

            // Validate geometry against file size
            let expected_size = cylinders as usize * heads as usize * sectors_per_track as usize * sector_size as usize;
            if expected_size != self.data.len() {
                return Err(anyhow!(
                    "Geometry {}x{}x{}x{} ({} bytes) does not match file size ({} bytes)",
                    cylinders, heads, sectors_per_track, sector_size, expected_size, self.data.len()
                ));
            }

            let mut raw_data = Vec::new();
            raw_data.extend(b"IMD 1.18: 19/02/2025 00:00:00\r\n\x1A");

            let mut pos = 0;
            for cyl in 0..cylinders {
                for head in 0..heads {
                    raw_data.push(5); // 250kbps MFM
                    raw_data.push(cyl);
                    raw_data.push(head);
                    raw_data.push(sectors_per_track);
                    raw_data.push(match sector_size { 128 => 0, 256 => 1, 512 => 2, 1024 => 3, 2048 => 4, 4096 => 5, _ => 2 });

                    for s in 1..=sectors_per_track {
                        raw_data.push(s);
                    }

                    for _ in 0..sectors_per_track {
                        if pos + sector_size as usize <= self.data.len() {
                            raw_data.push(1); // Normal data
                            raw_data.extend_from_slice(&self.data[pos..pos + sector_size as usize]);
                            pos += sector_size as usize;
                        } else {
                            raw_data.push(2); // Compressed zeros
                            raw_data.push(0);
                        }
                    }
                }
            }

            let mut file = File::create(output_path)?;
            file.write_all(&raw_data)?;
            Ok(())
        } else {
            Err(anyhow!("Conversion from IMG only supports IMD currently"))
        }
    }

    fn data(&self) -> &[u8] {
        &self.data
    }
}
