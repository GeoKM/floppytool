// src/disk_formats/mod.rs

pub mod floppy_3_5inch_ibm;
pub mod floppy_5_25inch_ibm;

pub use floppy_3_5inch_ibm::{IBM_720K, IBM_1_44M};
pub use floppy_5_25inch_ibm::{IBM_360K, IBM_1_2M};
