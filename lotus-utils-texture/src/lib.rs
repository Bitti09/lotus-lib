//! Texture parsing utilities for Warframe and Soulframe cache files (DDS header extraction, format detection).

mod dds_format;
mod header;
mod kind;
mod raw_header;
mod utils;

pub use utils::Texture;
