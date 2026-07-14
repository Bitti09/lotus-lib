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

    pub fn directories(&self) -> &[Node] {
        &self.directories
    }

    pub fn files(&self) -> &[Node] {
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

        let mut toc_reader = File::open(&self.toc_path)?;
        let entry_count = (toc_reader.metadata()?.len() as usize - 8) / TOC_ENTRY_SIZE;
        toc_reader.seek(SeekFrom::Start(8))?;

        // Reserve space for the entries in the vectors to avoid unnecessary
        // reallocations
        self.files.reserve(entry_count);
        self.directories.reserve(entry_count);

        let resurrect = std::env::var("RESURRECT").is_ok() || {
            let args: Vec<String> = std::env::args().collect();
            args.iter().any(|arg| {
                matches!(
                    arg.as_str(),
                    "--resurrect" | "-resurrect"
                        | "--resurrect-deleted" | "-resurrect-deleted"
                        | "--resurectdeleted" | "-resurectdeleted"
                        | "--resurrect-modified" | "-resurrect-modified"
                        | "--resurectmodified" | "-resurectmodified"
                )
            })
        };

        self.directories.push(Node::root());
        self.node_lookup
            .insert("/".to_string(), self.directories[0].clone());

        let mut dir_paths: Vec<String> = Vec::with_capacity(entry_count);
        dir_paths.push(String::new()); // root path is empty

        let mut buffer = vec![0u8; TOC_ENTRY_SIZE * entry_count];
        toc_reader.read_exact(&mut buffer)?;

        let entries = TocEntry::slice_from(&buffer).unwrap();
        for entry in entries {
            // Entry timestamp of 0 means the entry has been replaced with a
            // newer version with the same name and path with a valid timestamp
            if entry.timestamp == 0 && !resurrect {
                continue;
            }

            // Entry name is a null-terminated string, so we need to find the
            // index of the null byte and truncate the string there
            let null_index = entry.name.iter().position(|&x| x == 0).unwrap_or(64);
            let entry_name_bytes = &entry.name[..null_index];
            let entry_name = std::str::from_utf8(entry_name_bytes)?;
            
            let entry_name_owned = if entry.timestamp == 0 {
                // Append suffix to indicate deleted/old version and prevent collision
                if let Some(pos) = entry_name.rfind('.') {
                    let mut name = String::with_capacity(entry_name.len() + 8);
                    name.push_str(&entry_name[..pos]);
                    name.push_str(".deleted");
                    name.push_str(&entry_name[pos..]);
                    name
                } else {
                    let mut name = String::with_capacity(entry_name.len() + 8);
                    name.push_str(entry_name);
                    name.push_str(".deleted");
                    name
                }
            } else {
                entry_name.to_string()
            };

            let parent_node = self
                .directories
                .get_mut(entry.parent_dir_index as usize)
                .unwrap();

            let parent_path = &dir_paths[entry.parent_dir_index as usize];
            
            // Normalize path and build both normalized and full paths in one pass
            let entry_name_norm = if entry_name_owned.contains('\\') {
                entry_name_owned.replace('\\', "/")
            } else {
                entry_name_owned.clone()
            };
            
            let norm_path = if parent_path.is_empty() {
                entry_name_norm.to_lowercase()
            } else {
                format!("{}/{}", parent_path, entry_name_norm).to_lowercase()
            };
            
            let full_path = if parent_path.is_empty() {
                PathBuf::from(&entry_name_norm)
            } else {
                PathBuf::from(format!("{}/{}", parent_path, entry_name_norm))
            };

            // If the cache offset is -1, then the entry is a directory
            if entry.cache_offset == -1 {
                let dir_node = Node::directory(&entry_name_owned, full_path);

                parent_node.append(dir_node.clone());
                dir_paths.push(format!("{}/{}", parent_path, entry_name_norm));
                self.node_lookup.insert(norm_path, dir_node.clone());
                self.directories.push(dir_node);
            } else {
                let file_node = Node::file(
                    &entry_name_owned,
                    full_path,
                    entry.cache_offset,
                    entry.timestamp,
                    entry.comp_len,
                    entry.len,
                );

                parent_node.append(file_node.clone());
                self.node_lookup.insert(norm_path, file_node.clone());
                self.files.push(file_node);
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

    fn get_node(&self, path: &str) -> Option<Node> {
        if !self.is_loaded() {
            return None;
        }

        if !path.starts_with('/') {
            panic!("Path must be absolute");
        }

        let path_norm = path.replace('\\', "/").to_lowercase();
        self.node_lookup.get(&path_norm).cloned()
    }

    pub fn get_directory_node(&self, path: &str) -> Option<Node> {
        match self.get_node(path) {
            Some(node) => match node.kind() {
                NodeKind::Directory => Some(node),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn get_file_node(&self, path: &str) -> Option<Node> {
        match self.get_node(path) {
            Some(node) => match node.kind() {
                NodeKind::File => Some(node),
                _ => None,
            },
            _ => None,
        }
    }
}
