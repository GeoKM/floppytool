// src/disk_formats/floppy_3_5inch_ibm.rs

use super::DiskFormat; // Import from parent module

pub const IBM_720K: DiskFormat = DiskFormat {
    cylinders: 80,
    heads: 2,
    sectors_per_track: 9,
    sector_size: 512,
    mode: 4,
    name: "720K 3.5\" DD",
};

pub const IBM_1_44M: DiskFormat = DiskFormat {
    cylinders: 80,
    heads: 2,
    sectors_per_track: 18,
    sector_size: 512,
    mode: 5,
    name: "1.44M 3.5\" HD",
};

pub fn infer_format(size: usize) -> Option<&'static DiskFormat> {
    match size {
        737_280 => Some(&IBM_720K),
        1_474_560 => Some(&IBM_1_44M),
        _ => None,
    }
}
