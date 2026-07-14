use anyhow::Error;
use zerocopy::{ByteOrder, LittleEndian};

/// Represents the raw parsed binary audio header read from a Warframe cache metadata node.
#[allow(dead_code)]
pub struct RawAudioHeader<'a> {
    /// Unique hash identifying the audio asset.
    pub hash: &'a [u8; 16],
    /// Number of merged/sub-files referenced by this header.
    pub merged_file_count: u32,
    /// Paths of the merged/sub-files.
    pub file_paths: Vec<&'a str>,
    /// Length of the arguments string.
    pub arguments_length: u32,
    /// Arguments string passed for configuration.
    pub arguments: &'a str,
    /// File type indicator (e.g., 0x87 for early audio).
    pub file_type: u32,
    /// Format tag identifying the audio format (e.g. PCM, ADPCM, xWMA).
    pub format_tag: u32,
    /// Unknown metadata field 1.
    pub unknown1: u32,
    /// Unknown metadata segment 2.
    pub unknown2: &'a [u8],
    /// Number of audio samples per second (sample rate).
    pub samples_per_second: u32,
    /// Bit depth of each audio sample.
    pub bits_per_sample: u8,
    /// Number of audio channels.
    pub channels: u8,
    /// Unknown metadata field 3.
    pub unknown3: u32,
    /// Average bytes per second of the audio stream.
    pub average_bytes_per_second: u32,
    /// Block alignment size.
    pub block_align: u16,
    /// Number of samples per block.
    pub samples_per_block: u16,
    /// Unknown metadata field 4.
    pub unknown4: &'a [u8; 12],
    /// Size of the raw compressed audio stream.
    pub size: u32,
}

fn check_bounds(data: &[u8], offset: usize, len: usize) -> Result<(), Error> {
    if offset + len > data.len() {
        return Err(anyhow::anyhow!(
            "Header data truncated: offset {} + len {} is out of bounds for slice of length {}",
            offset,
            len,
            data.len()
        ));
    }
    Ok(())
}

impl<'a> TryFrom<&'a [u8]> for RawAudioHeader<'a> {
    type Error = Error;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        check_bounds(data, 0, 20)?;
        let hash = &data[0..16];
        let merged_file_count = LittleEndian::read_u32(&data[16..20]);

        let mut offset = 20;
        let mut file_paths = Vec::with_capacity(merged_file_count as usize);
        for _ in 0..merged_file_count {
            check_bounds(data, offset, 4)?;
            let path_length = LittleEndian::read_u32(&data[offset..offset + 4]) as usize;
            check_bounds(data, offset + 4, path_length)?;
            let path = std::str::from_utf8(&data[offset + 4..offset + 4 + path_length])?;

            file_paths.push(path);

            offset += 4 + path_length;
        }

        check_bounds(data, offset, 4)?;
        let arguments_length = LittleEndian::read_u32(&data[offset..offset + 4]);
        offset += 4;

        check_bounds(data, offset, arguments_length as usize)?;
        let arguments = std::str::from_utf8(&data[offset..offset + arguments_length as usize])?;
        offset += arguments_length as usize;

        // If the arguments length is > 0, then we need to skip the null byte
        if arguments_length > 0 {
            check_bounds(data, offset, 1)?;
            offset += 1;
        }

        check_bounds(data, offset, 12)?;
        let file_type = LittleEndian::read_u32(&data[offset..offset + 4]);
        let format_tag = LittleEndian::read_u32(&data[offset + 4..offset + 8]);
        let unknown1 = LittleEndian::read_u32(&data[offset + 8..offset + 12]);
        offset += 12;

        // Check if we have the early format where unknown2 is 28 bytes
        let unknown2_len = if file_type == 0x87 { 28 } else { 24 };
        check_bounds(data, offset, unknown2_len)?;
        let unknown2 = &data[offset..offset + unknown2_len];
        offset += unknown2_len;

        check_bounds(data, offset, 4)?;
        let samples_per_second = LittleEndian::read_u32(&data[offset..offset + 4]);
        offset += 4;

        check_bounds(data, offset, 2)?;
        let bits_per_sample = data[offset];
        let channels = data[offset + 1];
        offset += 2;

        check_bounds(data, offset, 12)?;
        let unknown3 = LittleEndian::read_u32(&data[offset..offset + 4]);
        let average_bytes_per_second = LittleEndian::read_u32(&data[offset + 4..offset + 8]);
        let block_align = LittleEndian::read_u16(&data[offset + 8..offset + 10]);
        let samples_per_block = LittleEndian::read_u16(&data[offset + 10..offset + 12]);
        offset += 12;

        check_bounds(data, offset, 16)?;
        let unknown4 = &data[offset..offset + 12];
        let size = LittleEndian::read_u32(&data[offset + 12..offset + 16]);

        Ok(RawAudioHeader {
            hash: hash.try_into()?,
            merged_file_count,
            file_paths,
            arguments_length,
            arguments,
            file_type,
            format_tag,
            unknown1,
            unknown2,
            samples_per_second,
            bits_per_sample,
            channels,
            unknown3,
            average_bytes_per_second,
            block_align,
            samples_per_block,
            unknown4: unknown4.try_into()?,
            size,
        })
    }
}
