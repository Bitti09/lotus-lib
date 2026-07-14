use anyhow::{Error, Result};
use bytebuffer::ByteBuffer;
use log::debug;
use lotus_lib::cache_pair::CachePairReader;
use lotus_lib::package::{Package, PackageType};
use lotus_lib::toc::{FileNode, Node};

use crate::compression_format::CompressionFormat;
use crate::header::AudioHeader;
use crate::kind::AudioKind;
use crate::ogg::{get_segment_table, OggPage};
use crate::raw_header::RawAudioHeader;

/// Extension trait providing methods to inspect and decompress audio nodes from cache packages.
pub trait Audio {
    /// Checks if the given node is an audio file.
    ///
    /// # Errors
    ///
    /// Returns an error if the H cache is not found.
    fn is_audio(&self, node: &Node) -> Result<bool>;

    /// Decompresses the audio file data and get the name for the given node.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to decompress the audio file for.
    ///
    /// # Returns
    ///
    /// A tuple containing the decompressed audio file data and the name of the audio file.
    fn decompress_audio(&self, node: &Node) -> Result<(Vec<u8>, String)>;

    /// Decompresses the audio file data directly as a WAV PCM byte stream.
    fn decompress_audio_as_pcm(&self, node: &Node) -> Result<Vec<u8>>;
}

impl Audio for Package<CachePairReader> {
    fn is_audio(&self, node: &Node) -> Result<bool> {
        if !node.name().ends_with(".wav") {
            return Ok(false);
        }

        let h_cache = self
            .borrow(PackageType::H)
            .ok_or(Error::msg("No header file found"))?;

        let header_file_data = h_cache.decompress_data(node.clone())?;
        let header = match RawAudioHeader::try_from(header_file_data.as_slice()) {
            Ok(header) => header,
            Err(_) => return Ok(false),
        };

        match AudioKind::try_from(header.file_type) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn decompress_audio(&self, node: &Node) -> Result<(Vec<u8>, String)> {
        let h_cache = self.borrow(PackageType::H);
        let f_cache = self.borrow(PackageType::F);
        let b_cache = self.borrow(PackageType::B);

        // Unwrap the H cache as there should always be a header file
        let h_cache = match h_cache {
            Some(h_cache) => h_cache,
            None => return Err(Error::msg("No header file found")),
        };

        // Get the decompressed header file data
        let header_file_data = h_cache.decompress_data(node.clone())?;

        // Parse the header file
        let header = AudioHeader::try_from(header_file_data.as_slice())?;
        debug!("Header: {:?}", header);

        match header.format_tag {
            CompressionFormat::PCM | CompressionFormat::ADPCM | CompressionFormat::XWMA => {
                // Get the file data
                let f_cache = f_cache.unwrap();
                let b_cache = b_cache.unwrap();

                let mut f_file_node = f_cache.get_file_node(node.path().to_str().unwrap());
                let mut b_file_node = b_cache.get_file_node(node.path().to_str().unwrap());

                if f_file_node.is_none() && b_file_node.is_none() {
                    let path_str = node.path().to_string_lossy().to_string();
                    if path_str.contains(".deleted") {
                        let clean_path = path_str.replace(".deleted", "");
                        f_file_node = f_cache.get_file_node(&clean_path);
                        b_file_node = b_cache.get_file_node(&clean_path);
                    }
                }

                let mut buffer = ByteBuffer::new();

                if b_file_node.is_some() {
                    let b_file_node = b_file_node.unwrap();

                    debug!("Part B file node found!");

                    debug!("Cache offset: {}", b_file_node.cache_offset() as u64);
                    debug!("Cache audio size: {}", b_file_node.comp_len() as u64);
                    debug!("Decompressed audio size: {}", b_file_node.len() as u64);

                    let b_file_data = b_cache.decompress_data(b_file_node.clone())?;
                    buffer.write_bytes(&b_file_data);
                }

                if f_file_node.is_some() {
                    let f_file_node = f_file_node.unwrap();

                    debug!("Part F file node found!");

                    debug!("Cache offset: {}", f_file_node.cache_offset() as u64);
                    debug!("Cache audio size: {}", f_file_node.comp_len() as u64);
                    debug!("Decompressed audio size: {}", f_file_node.len() as u64);

                    let f_file_data = f_cache.decompress_data(f_file_node.clone())?;
                    buffer.write_bytes(&f_file_data);
                }

                debug!("Real audio size: {}", header.size as u64);

                if buffer.len() < header.size as usize {
                    return Err(Error::msg("Audio payload missing or incomplete in F/B caches"));
                }

                let file_data = &buffer.as_bytes()[(buffer.len() - header.size as usize)..];

                let mut buffer = ByteBuffer::new();

                match header.format_tag {
                    CompressionFormat::PCM => buffer.write_bytes(&header.to_wav_pcm()?),
                    CompressionFormat::ADPCM => buffer.write_bytes(&header.to_wav_adpcm()?),
                    CompressionFormat::XWMA => buffer.write_bytes(&header.to_wav_xwma(file_data)?),
                    _ => return Err(Error::msg("Error extracting audio file")),
                }

                if header.format_tag != CompressionFormat::XWMA {
                    buffer.write_bytes(file_data);
                }

                let file_name = {
                    let name = node.name();
                    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(&name);
                    if header.format_tag == CompressionFormat::XWMA {
                        format!("{}.xwma", stem)
                    } else {
                        format!("{}.wav", stem)
                    }
                };

                Ok((buffer.as_bytes().to_vec(), file_name))
            }
            CompressionFormat::Opus => {
                // Get the file data
                let b_cache = b_cache.unwrap();
                let f_cache = f_cache.unwrap();

                let mut b_file_node = b_cache.get_file_node(node.path().to_str().unwrap());
                let mut f_file_node = f_cache.get_file_node(node.path().to_str().unwrap());

                if f_file_node.is_none() && b_file_node.is_none() {
                    let path_str = node.path().to_string_lossy().to_string();
                    if path_str.contains(".deleted") {
                        let clean_path = path_str.replace(".deleted", "");
                        f_file_node = f_cache.get_file_node(&clean_path);
                        b_file_node = b_cache.get_file_node(&clean_path);
                    }
                }

                let mut buffer = ByteBuffer::new();

                if f_file_node.is_some() {
                    let f_file_node = f_file_node.clone().unwrap();

                    debug!("Part F file node found!");

                    debug!("Cache offset: {}", f_file_node.cache_offset() as u64);
                    debug!("Cache audio size: {}", f_file_node.comp_len() as u64);
                    debug!("Decompressed audio size: {}", f_file_node.len() as u64);

                    let f_file_data = f_cache.decompress_data(f_file_node.clone())?;
                    buffer.write_bytes(&f_file_data);
                }

                if (f_file_node.is_none() || buffer.len() != header.size as usize)
                    && b_file_node.is_some()
                {
                    let b_file_node = b_file_node.unwrap();

                    debug!("Part B file node found!");

                    debug!("Cache offset: {}", b_file_node.cache_offset() as u64);
                    debug!("Cache audio size: {}", b_file_node.comp_len() as u64);
                    debug!("Decompressed audio size: {}", b_file_node.len() as u64);

                    let b_file_data = b_cache.decompress_data(b_file_node.clone())?;
                    buffer.write_bytes(&b_file_data);
                }

                debug!("Real audio size: {}", header.size as u64);

                if buffer.len() < header.size as usize {
                    return Err(Error::msg("Audio payload missing or incomplete in F/B caches"));
                }

                let file_data = &buffer.as_bytes()[..header.size as usize];

                let stream_serial_number = header.stream_serial_number;
                let samples_per_second = header.samples_per_second as u64;
                let block_align = header.block_align;

                let mut buffer = ByteBuffer::new();

                buffer.write_bytes(&header.to_opus()?);

                // Write the opus data
                let mut page_sequence_number = 2;
                let mut granule_position = samples_per_second;

                let chunk_size = block_align as usize * 50;

                for chunk in file_data.chunks(chunk_size) {
                    let header_type = if chunk.len() < chunk_size { 0x04 } else { 0x00 };
                    let segment_table = get_segment_table(chunk, block_align.into());
                    let data_page = OggPage::new(
                        header_type,
                        granule_position,
                        stream_serial_number,
                        page_sequence_number,
                        segment_table.len() as u8,
                        segment_table,
                        chunk.to_vec(),
                    );

                    buffer.write_bytes(&Into::<Vec<u8>>::into(data_page));

                    page_sequence_number += 1;
                    granule_position += samples_per_second;
                }

                let file_name = {
                    let name = node.name();
                    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(&name);
                    format!("{}.opus", stem)
                };

                Ok((buffer.as_bytes().to_vec(), file_name))
            }
        }
    }

    fn decompress_audio_as_pcm(&self, node: &Node) -> Result<Vec<u8>> {
        let h_cache = self.borrow(PackageType::H).ok_or(Error::msg("No header file found"))?;
        let header_file_data = h_cache.decompress_data(node.clone())?;
        let header = AudioHeader::try_from(header_file_data.as_slice())?;

        match header.format_tag {
            CompressionFormat::Opus => {
                // Try FFmpeg first for Opus
                if let Ok(wav_bytes) = decompress_via_ffmpeg(self, node) {
                    return Ok(wav_bytes);
                }

                // Fallback to manual Opus decoding:
                let b_cache = self.borrow(PackageType::B);
                let f_cache = self.borrow(PackageType::F);

                let path_str = node.path().to_string_lossy().to_string();
                let clean_path_str = path_str.replace(".deleted", "");

                let b_file_node = b_cache.and_then(|c| c.get_file_node(&clean_path_str));
                let f_file_node = f_cache.and_then(|c| c.get_file_node(&clean_path_str));

                let mut buffer = ByteBuffer::new();

                if let (Some(f_cache), Some(f_node)) = (f_cache, f_file_node) {
                    let f_file_data = f_cache.decompress_data(f_node.clone())?;
                    buffer.write_bytes(&f_file_data);
                }

                if let (Some(b_cache), Some(b_node)) = (b_cache, b_file_node) {
                    if f_cache.is_none() || buffer.len() != header.size as usize {
                        let b_file_data = b_cache.decompress_data(b_node.clone())?;
                        buffer.write_bytes(&b_file_data);
                    }
                }

                if buffer.len() < header.size as usize {
                    return Err(Error::msg("Audio payload missing or incomplete in F/B caches"));
                }

                let file_data = &buffer.as_bytes()[..header.size as usize];
                let block_align = header.block_align;

                let mut decoder = opus_decoder::OpusDecoder::new(header.samples_per_second, header.channels as usize)
                    .map_err(|e| anyhow::Error::msg(format!("Failed to create Opus decoder: {:?}", e)))?;
                let mut pcm_out = Vec::new();
                let packet_size = block_align as usize;
                
                let mut pcm_buffer = vec![0i16; 20000];
                
                for packet in file_data.chunks(packet_size) {
                    if let Ok(num_samples) = decoder.decode(packet, &mut pcm_buffer, false) {
                        let total_samples = num_samples * header.channels as usize;
                        pcm_out.extend_from_slice(&pcm_buffer[..total_samples]);
                    }
                }

                let mut pcm_bytes = Vec::with_capacity(pcm_out.len() * 2);
                for sample in pcm_out {
                    pcm_bytes.extend_from_slice(&sample.to_le_bytes());
                }

                let mut wav_header = header.clone();
                wav_header.size = pcm_bytes.len() as u32;
                wav_header.bits_per_sample = 16;

                let mut buffer = ByteBuffer::new();
                buffer.write_bytes(&wav_header.to_wav_pcm()?);
                buffer.write_bytes(&pcm_bytes);

                Ok(buffer.as_bytes().to_vec())
            }
            CompressionFormat::XWMA => {
                // Try FFmpeg for xWMA
                if let Ok(wav_bytes) = decompress_via_ffmpeg(self, node) {
                    return Ok(wav_bytes);
                }
                Err(Error::msg("xWMA decoding requires FFmpeg, which was not found in PATH"))
            }
            _ => {
                let (data, _) = self.decompress_audio(node)?;
                Ok(data)
            }
        }
    }
}

fn find_ffmpeg() -> Option<std::path::PathBuf> {
    if let Ok(path) = std::env::var("PATH") {
        for p in std::env::split_paths(&path) {
            let exe = p.join("ffmpeg.exe");
            if exe.exists() {
                return Some(exe);
            }
            let exe_no_ext = p.join("ffmpeg");
            if exe_no_ext.exists() {
                return Some(exe_no_ext);
            }
        }
    }
    None
}

fn decompress_via_ffmpeg(package: &Package<CachePairReader>, node: &Node) -> Result<Vec<u8>> {
    let ffmpeg_path = find_ffmpeg().ok_or_else(|| Error::msg("ffmpeg not found in PATH"))?;

    // Decompress the audio to get the wrapped container format (e.g. Ogg Opus or xWMA)
    let (compressed_data, _) = package.decompress_audio(node)?;

    let mut cmd = std::process::Command::new(ffmpeg_path);
    cmd.arg("-i")
        .arg("pipe:0")
        .arg("-f")
        .arg("wav")
        .arg("pipe:1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn()?;
    let mut stdin = child.stdin.take().ok_or_else(|| Error::msg("Failed to open stdin"))?;

    // Write input data in a background thread to prevent deadlocks when output pipes fill up
    std::thread::spawn(move || {
        use std::io::Write;
        let _ = stdin.write_all(&compressed_data);
    });

    let output = child.wait_with_output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        Err(Error::msg(format!("FFmpeg failed to decode audio: {}", err_msg)))
    }
}
