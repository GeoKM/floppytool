use crate::{FormatHandler, Geometry};
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

    fn infer_geometry(&self) -> Result<(u8, u8, u8, u16)> {
        let size = self.data.len();

        let formats = [
            (360_000, 40, 2, 9),   // 5.25" DD 360 KB
            (720_000, 80, 2, 9),   // 3.5" DD 720 KB
            (1_200_000, 80, 2, 15), // 5.25" HD 1.2 MB
            (1_440_000, 80, 2, 18), // 3.5" HD 1.44 MB
        ];

        for &(expected_size, cyl, heads, spt) in &formats {
            if size == expected_size {
                return Ok((cyl, heads, spt, 512));
            }
        }

        if size == 368_640 {
            return Ok((40, 2, 9, 512));
        }

        if size % 512 == 0 {
            let total_sectors = size / 512;
            for cyl in 40..=80 {
                for heads in (2..=1).rev() {
                    let spt = total_sectors / (cyl * heads);
                    if spt * cyl * heads == total_sectors && spt <= 36 {
                        return Ok((cyl as u8, heads as u8, spt as u8, 512));
                    }
                }
            }
        }

        Err(anyhow!("No suitable geometry found; specify with --geometry"))
    }
}

impl FormatHandler for IMGHandler {
    fn display(&self) -> Result<String> {
        let size = self.data.len();
        let (cyl, heads, spt, sector_size) = self.infer_geometry()?;
        let mut output = Vec::new();

        output.push(format!("Raw IMG: {} bytes", size));
        output.push(format!(
            "Detected Geometry: {} cylinders, {} heads, {} sectors/track, {} bytes/sector",
            cyl, heads, spt, sector_size
        ));
        Ok(output.join("\n"))
    }

    fn convert(&self, target: &dyn FormatHandler, output_path: &PathBuf, geometry: Option<Geometry>) -> Result<()> {
        if target.data().len() == 0 { // Conversion to IMD
            let (cylinders, heads, sectors_per_track, sector_size, mode) = match geometry {
                Some(Geometry::Manual { cylinders, heads, sectors_per_track, sector_size, mode }) => {
                    (cylinders, heads, sectors_per_track, sector_size, mode)
                }
                Some(Geometry::Auto) | None => {
                    return Err(anyhow!("Conversion from .img to .imd requires explicit geometry (e.g., '--geometry 40,2,9,512,4')"));
                }
            };

            let expected_size = cylinders as usize * heads as usize * sectors_per_track as usize * sector_size as usize;
            if expected_size != self.data.len() {
                return Err(anyhow!(
                    "Geometry {}x{}x{}x{} ({} bytes) does not match file size ({} bytes)",
                    cylinders, heads, sectors_per_track, sector_size, expected_size, self.data.len()
                ));
            }

            let mut raw_data = Vec::new();
            raw_data.extend(b"IMD 1.18: 28/11/2015 10:08:58\r\nLaplink v3 \r\n\x1A");

            let mut pos = 0;
            for cyl in 0..cylinders {
                for head in 0..heads {
                    raw_data.push(mode);
                    raw_data.push(cyl);
                    raw_data.push(head);
                    raw_data.push(sectors_per_track);
                    raw_data.push(match sector_size { 128 => 0, 256 => 1, 512 => 2, 1024 => 3, 2048 => 4, 4096 => 5, _ => 2 });

                    for s in 1..=sectors_per_track {
                        raw_data.push(s);
                    }

                    for _ in 0..sectors_per_track {
                        let chunk = &self.data[pos..pos + sector_size as usize];
                        if chunk.iter().all(|&b| b == chunk[0]) {
                            raw_data.push(2); // Compressed
                            raw_data.push(chunk[0]);
                        } else {
                            raw_data.push(1); // Normal data
                            raw_data.extend_from_slice(chunk);
                        }
                        pos += sector_size as usize;
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

    fn geometry(&self) -> Result<Option<Geometry>> {
        let (cylinders, heads, sectors_per_track, sector_size) = self.infer_geometry()?;
        Ok(Some(Geometry::Manual { cylinders, heads, sectors_per_track, sector_size, mode: 5 }))
    }

    fn data(&self) -> &[u8] {
        &self.data
    }
}
