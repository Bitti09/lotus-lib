//! Crate `lotus-utils-audio` provides utilities for handling, parsing, and
//! converting audio files extracted from Warframe cache pairs.

/// Module containing types representing compression formats of audio caches.
pub mod compression_format;
/// Module containing types representing parsed audio headers.
pub mod header;
mod kind;
mod ogg;
mod opus;
/// Module containing types representing raw cache audio headers.
pub mod raw_header;
mod utils;

pub use utils::Audio;
