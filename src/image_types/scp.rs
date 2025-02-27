use crate::{FormatHandler, Geometry};
use anyhow::{Result, anyhow};
use std::io::{Cursor, Read};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

pub struct SCPHandler {
    data: Vec<u8>,
}

impl SCPHandler {
    pub fn new(data: Vec<u8>) -> Self {
        SCPHandler { data }
    }

    fn parse_header(&self) -> Result<SCPHeader> {
        if self.data.len() < 16 {
            return Err(anyhow!("File too short: {} bytes. Expected at least 16 bytes for header.", self.data.len()));
        }
        let mut cursor = Cursor::new(&self.data);
        let mut magic = [0u8; 3];
        cursor.read_exact(&mut magic)?;
        if magic != *b"SCP" {
            return Err(anyhow!("Invalid .scp file: Magic bytes not 'SCP'"));
        }

        let version = cursor.read_u8()?;
        let disk_type = cursor.read_u8()?;
        let revolutions = cursor.read_u8()?;
        let start_track = cursor.read_u8()?;
        let end_track = cursor.read_u8()?;
        let flags = cursor.read_u8()?;
        let bit_cell_width = cursor.read_u8()?;
        let heads = cursor.read_u8()?;
        let resolution = cursor.read_u8()?;
        let checksum = cursor.read_u32::<LittleEndian>()?;

        if revolutions < 1 || revolutions > 5 {
            return Err(anyhow!("Invalid revolutions: {}. Must be 1-5.", revolutions));
        }
        if start_track > end_track || end_track > 167 {
            return Err(anyhow!("Invalid track range: start {} to end {}. Max 167.", start_track, end_track));
        }

        Ok(SCPHeader {
            version,
            disk_type,
            revolutions,
            start_track,
            end_track,
            flags,
            bit_cell_width,
            heads,
            resolution,
            checksum,
        })
    }

    fn disk_type_to_string(&self, disk_type: u8) -> String {
        let manufacturer = disk_type >> 4;
        let subclass = disk_type & 0x0F;
        match manufacturer {
            0x00 => format!("CBM, subclass {}", subclass),
            0x01 => format!("Atari, subclass {}", subclass),
            0x02 => format!("Apple, subclass {}", subclass),
            0x03 => match subclass {
                0x03 => "PC 1.44M".to_string(),
                0x05 => "PC 1.44M".to_string(), // Add 0x35 for v2.5 Index Mode
                _ => format!("PC, subclass {}", subclass),
            },
            0x08 => match subclass {
                0x00 => "PC 1.44M (Non-standard)".to_string(),
                _ => format!("Unknown manufacturer 8, subclass {}", subclass),
            },
            _ => format!("Unknown manufacturer {}, subclass {}", manufacturer, subclass),
        }
    }

    fn parse_track_headers(&self) -> Result<Vec<TrackInfo>> {
        let header = self.parse_header()?;
        let mut tracks = Vec::new();
        let mut cursor = Cursor::new(&self.data);

        // Skip header (16 bytes) to TDH offset table
        cursor.set_position(16);
        let track_count = (header.end_track as usize - header.start_track as usize + 1) as u64;
        if self.data.len() < (16 + track_count * 4) as usize {
            return Err(anyhow!("File too short for TDH table: {} bytes, need {}", self.data.len(), 16 + track_count * 4));
        }

        for _track_num in header.start_track as usize..=header.end_track as usize {
            let offset = cursor.read_u32::<LittleEndian>()?;
            if offset == 0 {
                continue; // No data for this track
            }
            if offset as usize + 12 > self.data.len() {
                return Err(anyhow!("Track offset 0x{:08X} exceeds file size {}", offset, self.data.len()));
            }
            let mut track_cursor = Cursor::new(&self.data);
            track_cursor.set_position(offset as u64);
            let mut trk = [0u8; 3];
            track_cursor.read_exact(&mut trk)?;
            if trk != *b"TRK" {
                return Err(anyhow!("Invalid track header at offset 0x{:08X}: Expected 'TRK'", offset));
            }
            let track_number = track_cursor.read_u8()?;
            let duration_total = track_cursor.read_u32::<BigEndian>()?; // Total duration in resolution units
            // println!("Track {} duration_total: {}", track_number, duration_total); // Debug print (commented out)
            let length_bytes = [
                track_cursor.read_u8()?,
                track_cursor.read_u8()?,
                track_cursor.read_u8()?,
            ];
            let length = ((length_bytes[0] as u32) << 16) | ((length_bytes[1] as u32) << 8) | (length_bytes[2] as u32); // 3 bytes, big-endian
            let effective_length = std::cmp::min(length, (self.data.len() - offset as usize) as u32); // Cap length
            track_cursor.read_u8()?; // Skip extra byte (flux data start)
            if offset as usize + effective_length as usize > self.data.len() {
                let track_start = offset as usize;
                let track_end = std::cmp::min(track_start + 12, self.data.len());
                let track_hex: Vec<String> = self.data[track_start..track_end].iter().map(|b| format!("{:02X}", b)).collect();
                println!("Warning: Track {} length {} bytes at offset 0x{:08X} exceeds file size {}. Track header bytes: {}", 
                         track_number, effective_length, offset, self.data.len(), track_hex.join(" "));
            }
            tracks.push(TrackInfo {
                track_number,
                duration_total,
                length: effective_length,
                offset,
            });
        }
        Ok(tracks)
    }
}

struct SCPHeader {
    version: u8,
    disk_type: u8,
    revolutions: u8,
    start_track: u8,
    end_track: u8,
    flags: u8,
    bit_cell_width: u8,
    heads: u8,
    resolution: u8,
    checksum: u32,
}

struct TrackInfo {
    track_number: u8,
    duration_total: u32, // Total in resolution units across all revolutions
    length: u32,         // in bytes (capped)
    offset: u32,         // file offset
}

impl FormatHandler for SCPHandler {
    fn display(&self, _ascii: bool) -> Result<String> {
        let header = self.parse_header()?;
        let tracks = self.parse_track_headers()?;
        let mut output = Vec::new();

        // Debug: Dump first 16 bytes
        let header_hex: Vec<String> = self.data[..16].iter().map(|b| format!("{:02X}", b)).collect();
        output.push(format!("Header Hex: {}", header_hex.join(" ")));

        output.push(format!("SuperCard Pro Image (.scp)"));
        output.push(format!("File Size: {} bytes", self.data.len()));
        output.push(format!("Version: {}.{}", header.version >> 4, header.version & 0x0F));
        output.push(format!("Disk Type: {} (0x{:02X})", self.disk_type_to_string(header.disk_type), header.disk_type));
        output.push(format!("Revolutions: {}", header.revolutions));
        output.push(format!("Track Range: {} to {}", header.start_track, header.end_track));
        output.push(format!("Flags: 0x{:02X} (Index: {}, TPI: {}, Flux at Index: {})",
            header.flags,
            header.flags & 0x01 != 0,
            if header.flags & 0x02 != 0 { "96" } else { "48" },
            header.flags & 0x20 != 0
        ));
        output.push(format!(
            "Bit Cell Width: {}",
            if header.bit_cell_width == 0 { "16 flux units".to_string() } else { format!("{}", header.bit_cell_width) }
        ));
        output.push(format!(
            "Heads: {}",
            match header.heads {
                0 => "Both".to_string(),
                1 => "Side 0".to_string(),
                2 => "Side 1".to_string(),
                _ => format!("Invalid: {}", header.heads),
            }
        ));
        let resolution_ns = if header.resolution == 0 { 25 } else { header.resolution as u32 * 25 }; // Default 25ns
        let inferred_resolution_ns = if !tracks.is_empty() && header.revolutions > 0 {
            let duration_units = tracks[0].duration_total as f64 / header.revolutions as f64;
            if duration_units > 0.0 {
                (200_000.0 / (duration_units / 1_000_000.0)) as u32 // Assume 1µs units, convert to ns
            } else {
                resolution_ns
            }
        } else {
            resolution_ns
        }; // Infer from first track if possible
        output.push(format!(
            "Resolution: {}ns (inferred: {}ns){}",
            resolution_ns,
            inferred_resolution_ns,
            if header.resolution == 0 { " (assumed default for v2.5)" } else { "" }
        ));
        output.push(format!("Checksum: 0x{:08X}", header.checksum));

        // Track data
        output.push(format!("Tracks ({}):", tracks.len()));
        let resolution_us = inferred_resolution_ns as f64 / 1_000_000.0; // Use inferred resolution in µs
        for track in tracks {
            let duration_ms_total = (track.duration_total as f64 * resolution_us) / 1_000.0; // Total µs to ms
            let duration_ms_per_rev = duration_ms_total / header.revolutions as f64; // Per revolution
            output.push(format!(
                "  Track {}: Duration {:.2}ms/rev (Total {:.2}ms), Length {} bytes, Offset 0x{:08X}",
                track.track_number, duration_ms_per_rev, duration_ms_total, track.length, track.offset
            ));
        }

        Ok(output.join("\n"))
    }

    fn convert(&self, _target: &dyn FormatHandler, _output_path: &std::path::PathBuf, _input_path: &std::path::PathBuf, _meta_path: Option<&std::path::PathBuf>, _geometry: Option<Geometry>, _verbose: bool, _validate: bool) -> Result<()> {
        Err(anyhow!("Conversion from .scp not yet implemented"))
    }

    fn data(&self) -> &[u8] {
        &self.data
    }

    fn geometry(&self) -> Result<Option<Geometry>> {
        let header = self.parse_header()?;
        if header.disk_type == 0x33 || header.disk_type == 0x80 || header.disk_type == 0x35 { // PC 1.44M variants
            Ok(Some(Geometry::Manual {
                cylinders: 80,
                heads: 2,
                sectors_per_track: 18,
                sector_size: 512,
                mode: 5, // Common MFM mode for 1.44M
            }))
        } else {
            Ok(None) // Geometry inference not implemented for other types yet
        }
    }
}
