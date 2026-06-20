use std::fs::File;
use std::io::Read;

use anyhow::Result;

/// Decompresses data from a pre-ensmallening cache block structure.
pub fn decompress_pre_ensmallening(
    compressed_len: usize,
    decompressed_len: usize,
    cache_reader: &mut File,
) -> Result<Vec<u8>> {
    let mut compressed_data = vec![0u8; compressed_len];
    cache_reader.read_exact(&mut compressed_data).unwrap();

    let mut decompressed_data = Vec::with_capacity(decompressed_len);
    let mut pos = 0;

    while pos < compressed_data.len() {
        if pos + 4 > compressed_data.len() {
            return Err(anyhow::anyhow!("Block header incomplete"));
        }
        let block_comp_len =
            u16::from_be_bytes([compressed_data[pos], compressed_data[pos + 1]]) as usize;
        let block_decomp_len =
            u16::from_be_bytes([compressed_data[pos + 2], compressed_data[pos + 3]]) as usize;
        pos += 4;

        if pos + block_comp_len > compressed_data.len() {
            return Err(anyhow::anyhow!("Block data incomplete"));
        }

        let compressed_payload = &compressed_data[pos..pos + block_comp_len];
        pos += block_comp_len;

        if block_comp_len == block_decomp_len {
            decompressed_data.extend_from_slice(compressed_payload);
        } else {
            let decomp = decompress_custom_lz(compressed_payload, block_decomp_len)
                .map_err(|e| anyhow::anyhow!("Custom LZ decompression error: {}", e))?;
            decompressed_data.extend_from_slice(&decomp);
        }
    }

    Ok(decompressed_data)
}

fn decompress_custom_lz(
    compressed: &[u8],
    decompressed_len: usize,
) -> Result<Vec<u8>, &'static str> {
    let mut result = vec![0u8; decompressed_len];
    let mut comp_pos = 0;
    let mut decomp_pos = 0;

    while comp_pos < compressed.len() {
        let code_word = compressed[comp_pos];
        comp_pos += 1;
        let code_word_value = code_word as usize;

        if code_word_value < 32 {
            let copy_len = code_word_value + 1;
            if decomp_pos + copy_len > result.len() {
                return Err("Decompression buffer overflow in literal copy");
            }
            if comp_pos + copy_len > compressed.len() {
                return Err("Compression buffer read overrun in literal copy");
            }
            result[decomp_pos..decomp_pos + copy_len]
                .copy_from_slice(&compressed[comp_pos..comp_pos + copy_len]);
            decomp_pos += copy_len;
            comp_pos += copy_len;
            continue;
        }

        let mut copy_length = code_word_value >> 5;
        if copy_length == 7 {
            if comp_pos >= compressed.len() {
                return Err("Compression buffer read overrun in copy_length extension");
            }
            copy_length += compressed[comp_pos] as usize;
            comp_pos += 1;
        }

        if comp_pos >= compressed.len() {
            return Err("Compression buffer read overrun in dictDist");
        }
        let temp = (code_word_value & 0x1F) << 8;
        let dict_dist = temp | (compressed[comp_pos] as usize);
        comp_pos += 1;

        copy_length += 2;
        if decomp_pos + copy_length > result.len() {
            return Err("Decompression buffer overflow in dict copy");
        }

        let decomp_dist_begin_pos = decomp_pos as isize - 1 - dict_dist as isize;
        if decomp_dist_begin_pos < 0 {
            return Err("Negative lookbehind offset");
        }

        let start = decomp_dist_begin_pos as usize;
        for i in 0..copy_length {
            result[decomp_pos + i] = result[start + i];
        }
        decomp_pos += copy_length;
    }

    Ok(result)
}
