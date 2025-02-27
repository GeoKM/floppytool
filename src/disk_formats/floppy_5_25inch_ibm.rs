// src/disk_formats/floppy_5_25inch_ibm.rs

use super::DiskFormat; // Import from parent module

pub const IBM_360K: DiskFormat = DiskFormat {
    cylinders: 40,
    heads: 2,
    sectors_per_track: 9,
    sector_size: 512,
    mode: 4,
    name: "360K 5.25\" DD",
};

pub const IBM_1_2M: DiskFormat = DiskFormat {
    cylinders: 80,
    heads: 2,
    sectors_per_track: 15,
    sector_size: 512,
    mode: 3,
    name: "1.2M 5.25\" HD",
};

pub fn infer_format(size: usize) -> Option<&'static DiskFormat> {
    match size {
        368_640 => Some(&IBM_360K),
        1_228_800 => Some(&IBM_1_2M),
        _ => None,
    }
}
