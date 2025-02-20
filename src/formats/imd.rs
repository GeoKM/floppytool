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
}

impl FormatHandler for IMDHandler {
    fn display(&self) -> Result<String> {
        let mut output = Vec::new();
        let header_end = self.data.iter().position(|&b| b == 0x1A)
            .ok_or_else(|| anyhow!("No header terminator found"))?;
        let header = String::from_utf8_lossy(&self.data[..header_end]);
        output.push(format!("Header: {}", header));

        let mut cursor = Cursor::new(&self.data[header_end + 1..]);
        while cursor.position() < self.data.len() as u64 - header_end as u64 - 1 {
            let _mode = cursor.read_u8()?;
            let cylinder = cursor.read_u8()?;
            let head = cursor.read_u8()?;
            let sector_count = cursor.read_u8()?;
            let sector_size_code = cursor.read_u8()?;
            let sector_size = 128 << sector_size_code;

            output.push(format!(
                "Cyl {}, Head {}: {} sectors, size {} bytes",
                cylinder, head, sector_count, sector_size
            ));

            let skip_bytes = sector_count as u64
                + if head & 0x80 != 0 { sector_count as u64 } else { 0 }
                + if head & 0x40 != 0 { sector_count as u64 } else { 0 };
            cursor.set_position(cursor.position() + skip_bytes);

            for _ in 0..sector_count {
                let type_byte = cursor.read_u8()?;
                match type_byte {
                    1 => cursor.set_position(cursor.position() + sector_size as u64),
                    2 => cursor.set_position(cursor.position() + 1),
                    _ => return Err(anyhow!("Unsupported sector type: {}", type_byte)),
                }
            }
        }
        Ok(output.join("\n"))
    }

    fn convert(&self, target: &dyn FormatHandler, output_path: &PathBuf, _geometry: Option<Geometry>) -> Result<()> {
        if target.data().len() == 0 { // IMG conversion
            let mut raw_data = Vec::new();
            let header_end = self.data.iter().position(|&b| b == 0x1A).unwrap();
            let mut cursor = Cursor::new(&self.data[header_end + 1..]);

            while cursor.position() < self.data.len() as u64 - header_end as u64 - 1 {
                let _mode = cursor.read_u8()?;
                let _cyl = cursor.read_u8()?;
                let _head = cursor.read_u8()?;
                let sector_count = cursor.read_u8()?;
                let sector_size_code = cursor.read_u8()?;
                let sector_size = 128 << sector_size_code;

                cursor.set_position(cursor.position() + sector_count as u64);
                for _ in 0..sector_count {
                    let type_byte = cursor.read_u8()?;
                    match type_byte {
                        1 => {
                            let mut sector_data = vec![0u8; sector_size as usize];
                            cursor.read_exact(&mut sector_data)?;
                            raw_data.extend_from_slice(&sector_data); // Fixed typo: §or_data -> sector_data
                        }
                        2 => {
                            let value = cursor.read_u8()?;
                            raw_data.extend(vec![value; sector_size as usize]);
                        }
                        _ => return Err(anyhow!("Unsupported sector type")),
                    }
                }
            }

            let mut file = File::create(output_path)?;
            file.write_all(&raw_data)?;
            Ok(())
        } else {
            Err(anyhow!("Conversion to this format not implemented"))
        }
    }

    fn data(&self) -> &[u8] { &self.data }
}
