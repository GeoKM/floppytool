// src/disk_formats/floppy-3.5inch-IBM.rs

/// Represents a floppy disk format with its physical characteristics.
#[derive(Debug, Clone, Copy)]
pub struct DiskFormat {
    pub cylinders: u8,
    pub heads: u8,
    pub sectors_per_track: u8,
    pub sector_size: u16,
    pub mode: u8, // 5 = MFM 500kbps, 4 = MFM 250kbps
    pub name: &'static str,
}

impl DiskFormat {
    /// Calculates the total size in bytes for this disk format.
    pub fn total_size(&self) -> usize {
        self.cylinders as usize * self.heads as usize * self.sectors_per_track as usize * self.sector_size as usize
    }
}

/// 720K Double Density 3.5-inch IBM/PC floppy (MFM, 250kbps).
pub const IBM_720K: DiskFormat = DiskFormat {
    cylinders: 80,
    heads: 2,
    sectors_per_track: 9,
    sector_size: 512,
    mode: 4, // Note: Often listed as 5 in tools, but technically 250kbps
    name: "720K 3.5\" DD",
};

/// 1.44M High Density 3.5-inch IBM/PC floppy (MFM, 500kbps).
pub const IBM_1_44M: DiskFormat = DiskFormat {
    cylinders: 80,
    heads: 2,
    sectors_per_track: 18,
    sector_size: 512,
    mode: 5,
    name: "1.44M 3.5\" HD",
};

/// Infers the disk format based on file size.
pub fn infer_format(size: usize) -> Option<&'static DiskFormat> {
    match size {
        737_280 => Some(&IBM_720K),
        1_474_560 => Some(&IBM_1_44M),
        _ => None,
    }
}
