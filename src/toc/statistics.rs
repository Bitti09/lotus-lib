use std::sync::Arc;

use crate::toc::node::{FileNode, NodeKind};
use crate::toc::Toc;

/// Statistics about the contents of a TOC file.
#[derive(Debug, Clone, Default)]
pub struct TocStatistics {
    /// Total number of directory entries.
    pub directory_count: usize,
    /// Total number of active file entries.
    pub file_count: usize,
    /// Total number of deleted/replaced entries.
    pub deleted_count: usize,
    /// Total compressed size of all active files.
    pub total_compressed_size: i64,
    /// Total decompressed size of all active files.
    pub total_decompressed_size: i64,
    /// Overall compression ratio (compressed / decompressed).
    pub compression_ratio: f64,
    /// Number of uncompressed files (comp_len == len).
    pub uncompressed_count: usize,
    /// Size distribution buckets.
    pub size_distribution: SizeDistribution,
}

/// File size distribution in buckets.
#[derive(Debug, Clone, Default)]
pub struct SizeDistribution {
    /// Files smaller than 1 KB.
    pub under_1kb: usize,
    /// Files between 1 KB and 1 MB.
    pub kb_to_mb: usize,
    /// Files between 1 MB and 10 MB.
    pub mb_to_10mb: usize,
    /// Files larger than 10 MB.
    pub over_10mb: usize,
}

impl Toc {
    #[allow(dead_code)]
    pub fn statistics(&self) -> TocStatistics {
        let mut stats = TocStatistics::default();

        for node in self.files() {
            stats.file_count += 1;

            let comp_len = node.comp_len() as i64;
            let len = node.len() as i64;

            stats.total_compressed_size += comp_len;
            stats.total_decompressed_size += len;

            if comp_len == len {
                stats.uncompressed_count += 1;
            }

            if len < 1024 {
                stats.size_distribution.under_1kb += 1;
            } else if len < 1024 * 1024 {
                stats.size_distribution.kb_to_mb += 1;
            } else if len < 10 * 1024 * 1024 {
                stats.size_distribution.mb_to_10mb += 1;
            } else {
                stats.size_distribution.over_10mb += 1;
            }
        }

        for node in self.directories() {
            if node.kind() == NodeKind::Directory {
                stats.directory_count += 1;
            }
        }

        if stats.total_decompressed_size > 0 {
            stats.compression_ratio =
                stats.total_compressed_size as f64 / stats.total_decompressed_size as f64;
        }

        stats
    }

    #[allow(dead_code)]
    pub fn largest_file(&self) -> Option<(Arc<str>, i32, i32)> {
        self.files()
            .iter()
            .max_by_key(|node| node.len())
            .map(|node| (node.name(), node.comp_len(), node.len()))
    }

    #[allow(dead_code)]
    pub fn total_entries(&self) -> usize {
        self.directories().len() + self.files().len()
    }
}