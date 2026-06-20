use anyhow::Error;

/// Represents the compression formats used for audio data in Warframe caches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionFormat {
    /// Uncompressed PCM audio format.
    PCM,
    /// Compressed MS ADPCM audio format.
    ADPCM,
    /// Compressed Opus audio format.
    Opus,
    /// Compressed Microsoft xWMA/WMAv2 audio format.
    XWMA,
}

impl TryFrom<u32> for CompressionFormat {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x00 | 0x01 => Ok(CompressionFormat::PCM),
            0x05 => Ok(CompressionFormat::ADPCM),
            0x07 => Ok(CompressionFormat::Opus),
            _ => Err(Error::msg("Unknown compression format")),
        }
    }
}
