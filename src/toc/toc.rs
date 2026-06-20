use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use anyhow::Result;
use zerocopy::FromBytes;

use crate::toc::node::{Node, NodeKind};
use crate::toc::toc_entry::{TocEntry, TOC_ENTRY_SIZE};

pub(crate) struct Toc {
    toc_path: PathBuf,
    directories: Vec<Node>,
    files: Vec<Node>,
    node_lookup: HashMap<String, Node>,
}

impl Toc {
    pub fn new(toc_path: PathBuf) -> Self {
        Self {
            toc_path,
            directories: Vec::new(),
            files: Vec::new(),
            node_lookup: HashMap::new(),
        }
    }

    pub fn directories(&self) -> &Vec<Node> {
        &self.directories
    }

    pub fn files(&self) -> &Vec<Node> {
        &self.files
    }

    pub fn is_loaded(&self) -> bool {
        !self.directories.is_empty()
    }

    pub fn read_toc(&mut self) -> Result<()> {
        if self.is_loaded() {
            return Ok(()); // TOC already loaded
        }

        // Clear the directory and file vectors in case they were populated
        // from a previous read
        self.unread_toc();

        let mut toc_reader = File::open(&self.toc_path).unwrap();
        let entry_count = (toc_reader.metadata().unwrap().len() as usize - 8) / TOC_ENTRY_SIZE;
        toc_reader.seek(SeekFrom::Start(8)).unwrap();

        // Reserve space for the entries in the vectors to avoid unnecessary
        // reallocations
        self.files.reserve(entry_count);
        self.directories.reserve(entry_count);

        let mut file_count = 0;
        let mut dir_count = 1; // Hardcoded root directory

        self.directories.insert(0, Node::root());
        self.node_lookup
            .insert("/".to_string(), self.directories[0].clone());

        let mut dir_paths: Vec<String> = Vec::with_capacity(entry_count);
        dir_paths.push("".to_string()); // root path is empty

        let mut buffer = vec![0u8; TOC_ENTRY_SIZE * entry_count];
        toc_reader.read_exact(&mut buffer).unwrap();

        let entries = TocEntry::slice_from(&buffer).unwrap();
        for entry in entries {
            // Entry timestamp of 0 means the entry has been replaced with a
            // newer version with the same name and path with a valid timestamp
            if entry.timestamp == 0 {
                continue;
            }

            // Entry name is a null-terminated string, so we need to find the
            // index of the null byte and truncate the string there
            let entry_name = {
                let null_index = entry.name.iter().position(|&x| x == 0).unwrap_or(64);
                let entry_name = std::str::from_utf8(&entry.name[..null_index])?;
                entry_name
            };

            let parent_node = self
                .directories
                .get_mut(entry.parent_dir_index as usize)
                .unwrap();

            let parent_path = &dir_paths[entry.parent_dir_index as usize];
            let entry_name_norm = entry_name.replace('\\', "/");
            let norm_path = format!("{}/{}", parent_path, entry_name_norm).to_lowercase();

            // If the cache offset is -1, then the entry is a directory
            if entry.cache_offset == -1 {
                let dir_node = Node::directory(entry_name);

                parent_node.append(dir_node.clone());
                self.directories.insert(dir_count, dir_node.clone());
                dir_paths.insert(dir_count, format!("{}/{}", parent_path, entry_name_norm));

                self.node_lookup.insert(norm_path, dir_node);

                dir_count += 1;
            } else {
                let file_node = Node::file(
                    entry_name,
                    entry.cache_offset,
                    entry.timestamp,
                    entry.comp_len,
                    entry.len,
                );

                parent_node.append(file_node.clone());
                self.files.insert(file_count, file_node.clone());

                self.node_lookup.insert(norm_path, file_node);

                file_count += 1;
            }
        }

        // Shrink the vectors to the actual size of the vectors to save memory
        self.directories.shrink_to_fit();
        self.files.shrink_to_fit();

        Ok(()) // TOC read successfully
    }

    pub fn unread_toc(&mut self) {
        self.directories.clear();
        self.files.clear();
        self.node_lookup.clear();
    }

    fn get_node(&self, path: PathBuf) -> Option<Node> {
        if !self.is_loaded() {
            return None;
        }

        if !path.has_root() {
            panic!("Path must be absolute");
        }

        let path_norm = path.to_string_lossy().replace('\\', "/").to_lowercase();
        self.node_lookup.get(&path_norm).cloned()
    }

    pub fn get_directory_node(&self, path: PathBuf) -> Option<Node> {
        match self.get_node(path) {
            Some(node) => match node.kind() {
                NodeKind::Directory => Some(node),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn get_file_node(&self, path: PathBuf) -> Option<Node> {
        match self.get_node(path) {
            Some(node) => match node.kind() {
                NodeKind::File => Some(node),
                _ => None,
            },
            _ => None,
        }
    }
}
