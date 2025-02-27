// src/disk_formats/floppy-5.25inch-IBM.rs

/// Represents a floppy disk format with its physical characteristics.
#[derive(Debug, Clone, Copy)]
pub struct DiskFormat {
    pub cylinders: u8,
    pub heads: u8,
    pub sectors_per_track: u8,
    pub sector_size: u16,
    pub mode: u8, // 4 = MFM 250kbps, 3 = MFM 500kbps for 5.25" HD
    pub name: &'static str,
}

impl DiskFormat {
    /// Calculates the total size in bytes for this disk format.
    pub fn total_size(&self) -> usize {
        self.cylinders as usize * self.heads as usize * self.sectors_per_track as usize * self.sector_size as usize
    }
}

/// 360K Double Density 5.25-inch IBM/PC floppy (MFM, 250kbps).
pub const IBM_360K: DiskFormat = DiskFormat {
    cylinders: 40,
    heads: 2,
    sectors_per_track: 9,
    sector_size: 512,
    mode: 4,
    name: "360K 5.25\" DD",
};

/// 1.2M High Density 5.25-inch IBM/PC floppy (MFM, 500kbps).
pub const IBM_1_2M: DiskFormat = DiskFormat {
    cylinders: 80,
    heads: 2,
    sectors_per_track: 15,
    sector_size: 512,
    mode: 3,
    name: "1.2M 5.25\" HD",
};

/// Infers the disk format based on file size.
pub fn infer_format(size: usize) -> Option<&'static DiskFormat> {
    match size {
        368_640 => Some(&IBM_360K),
        1_228_800 => Some(&IBM_1_2M),
        _ => None,
    }
}
