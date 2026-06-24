use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Result;

use crate::cache_pair::cache_pair::CachePair;
use crate::compression::post_ensmallening::decompress_post_ensmallening;
use crate::compression::pre_ensmallening::decompress_pre_ensmallening;
use crate::toc::{FileNode, Node, Toc};

/// A cache pair reader.
pub struct CachePairReader {
    is_post_ensmallening: bool,
    toc_path: PathBuf,
    cache_path: PathBuf,
    toc: Toc,
    cache_file: Mutex<Option<File>>,
    cache_size: u64,
}

impl CachePair for CachePairReader {
    fn new(toc_path: PathBuf, cache_path: PathBuf, is_post_ensmallening: bool) -> Self {
        let toc = Toc::new(toc_path.clone());
        let cache_size = std::fs::metadata(&cache_path).map(|m| m.len()).unwrap_or(0);
        Self {
            is_post_ensmallening,
            toc_path,
            cache_path,
            toc,
            cache_file: Mutex::new(None),
            cache_size,
        }
    }

    fn is_post_ensmallening(&self) -> bool {
        self.is_post_ensmallening
    }

    fn toc_path(&self) -> PathBuf {
        self.toc_path.clone()
    }

    fn cache_path(&self) -> PathBuf {
        self.cache_path.clone()
    }

    fn read_toc(&mut self) -> Result<()> {
        self.toc.read_toc()?;

        // Auto-detect is_post_ensmallening based on the first compressed file
        if let Some(first_compressed) = self.toc.files().iter().find(|node| node.comp_len() != node.len()) {
            if let Ok(mut file) = File::open(&self.cache_path) {
                if file.seek(SeekFrom::Start(first_compressed.cache_offset() as u64)).is_ok() {
                    let mut header = [0u8; 8];
                    if file.read_exact(&mut header).is_ok() {
                        self.is_post_ensmallening = header[0] == 0x80 && (header[7] & 0x0F) == 0x1;
                    }
                }
            }
        }

        Ok(())
    }

    fn unread_toc(&mut self) {
        self.toc.unread_toc()
    }

    fn cache_size(&self) -> u64 {
        self.cache_size
    }
}

impl CachePairReader {
    /// Get the directory node for the given path.
    pub fn get_directory_node<T: Into<PathBuf>>(&self, path: T) -> Option<Node> {
        self.toc.get_directory_node(path.into())
    }

    /// Get the file node for the given path.
    pub fn get_file_node<T: Into<PathBuf>>(&self, path: T) -> Option<Node> {
        self.toc.get_file_node(path.into())
    }

    /// Get the directory nodes
    pub fn directories(&self) -> &Vec<Node> {
        self.toc.directories()
    }

    /// Get the file nodes
    pub fn files(&self) -> &Vec<Node> {
        self.toc.files()
    }

    /// Read the data without decompressing it for the given file node.
    pub fn get_data(&self, file_node: Node) -> Result<Vec<u8>> {
        let mut guard = self.cache_file.lock().unwrap();
        if guard.is_none() {
            *guard = Some(File::open(&self.cache_path)?);
        }
        let cache_reader = guard.as_mut().unwrap();
        cache_reader.seek(SeekFrom::Start(file_node.cache_offset() as u64))?;

        let mut data = vec![0; file_node.comp_len() as usize];
        cache_reader.read_exact(&mut data)?;
        Ok(data)
    }

    /// Read and decompress the data for the given file node.
    ///
    /// If the file is not compressed, the data is read without decompressing it.
    pub fn decompress_data(&self, file_node: Node) -> Result<Vec<u8>> {
        if file_node.comp_len() == file_node.len() {
            return self.get_data(file_node);
        }

        let mut guard = self.cache_file.lock().unwrap();
        if guard.is_none() {
            *guard = Some(File::open(&self.cache_path)?);
        }
        let cache_reader = guard.as_mut().unwrap();
        cache_reader.seek(SeekFrom::Start(file_node.cache_offset() as u64))?;

        if self.is_post_ensmallening {
            return decompress_post_ensmallening(
                file_node.comp_len() as usize,
                file_node.len() as usize,
                cache_reader,
            );
        } else {
            return decompress_pre_ensmallening(
                file_node.comp_len() as usize,
                file_node.len() as usize,
                cache_reader,
            );
        }
    }
}
