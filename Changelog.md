# Changelog

All notable changes to lotus-lib and its utility crates.

## [0.2.0] - 2026-07-14

### Added
- **O(1) Normalized Path Lookup:** `HashMap<String, Node>` inside `Toc` with lowercase + forward-slash normalization for absolute paths.
- **O(1) TOC Path Calculation:** Sequential parent-paths cache during parsing eliminates recursive tree-walking after parse.
- **File Handle Caching:** Lazy `Mutex<Option<File>>` in `CachePairReader` avoids open/close per chunk, reducing disk system calls from thousands to a few per thread.
- **Cache Size Caching:** `cache_size()` method on `CachePair` trait eliminates redundant `fs::metadata` calls.
- **Thread-Local Decompression Buffer:** 256 KB reused buffer (`DECOMPRESS_BUFFER`) in `decompress_post_ensmallening` removes per-block heap allocation.
- **LZ Lookbehind Optimization:** `copy_within` replaces byte-by-byte dictionary copying in `decompress_custom_lz` (pre-ensmallening).
- **TOC Statistics Module:** `statistics.rs` — exposes file count, directory count, and total size per package.
- **Localization Suffixes:** API supports language-specific path suffixes (e.g. `_de`, `_fr`) for localized asset lookups.

### Changed
- **Breaking API:** `arctree::Node::read()` returns `Arc<str>` / `Arc<Path>` instead of temporary `RwLockReadGuard` references.
- **Breaking API:** Function signatures use `&str` instead of `&String` where applicable.
- **`PackageType` enum:** `H` variant correctly described as header metadata package.
- **Timestamp semantics:** `0` = deleted entry; `reserved` field: `-1` = directory, `0` = normal file, `>0` = engine type ID. Timestamps are Windows FILETIME (100ns ticks since 1601-01-01).
- **All `cargo check` warnings resolved** — 0 errors, 0 warnings.

### Fixed
- **Buffer allocation bug:** `post_ensmallening.rs` used `reserve()` (grows capacity only) where `resize()` (sets length) was required, causing panics on decompression.
- **Texture H-cache node lookup:** `lotus-utils-texture/src/utils.rs` looked up the F/B node path instead of the H node, producing wrong or missing texture headers.
- **Windows Path Separator Bug:** `PathBuf::components()` split backslash-containing directory entries (e.g. `\Lotus\Sounds...`), preventing `get_file_node` from finding files with mixed separators.

---

## [lotus-utils-audio] - 2026-07-14

### Added
- **Merged into lotus-lib:** `lotus-utils-audio/` moved from standalone crate to `lotus-lib/lotus-utils-audio/`.
- **`decompress_audio_as_pcm()`:** Unified audio decompression with FFmpeg fallback for unsupported formats.
- **Manual Opus Decoding:** `opus-decoder` dependency for direct Opus-to-PCM without FFmpeg.
- **`.deleted` File Resurrection:** Automatically recovers audio files marked as deleted (timestamp = 0) by checking H-package payloads.
- **Streaming CRC Digest:** OGG files compute CRC incrementally during read instead of buffering entire file.
- **`check_bounds()` Validation:** `raw_header.rs` validates offset/size fields against cache bounds before access.
- **`Clone` derive on `AudioHeader`.**
- **xWMA (WMAv2) Support:** RIFF header construction for legacy WMA audio containers.

### Fixed
- **xWMA vs PCM Format Tag:** Format tag `0x01` mapped to XWMA only when `block_align > 16`; small-block PCM preserved correctly for early 2013-era cache files.

---

## [lotus-utils-texture] - 2026-07-14

### Fixed
- **H-cache node lookup:** `is_texture()` and `decompress_texture()` now look up the H-package node by the correct path, instead of passing the F/B node path.

### Changed
- **All `cargo check` warnings resolved.**
