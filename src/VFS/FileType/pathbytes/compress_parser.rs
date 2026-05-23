use std::io::Read;

/// Compressed bin parse result
pub(super) struct CompressEntry {
    pub index: usize,
    pub data: Vec<u8>,
}

/// Compressed bin parser
///
/// Format:
/// - Bytes 0-3: offset entry count (u32)
/// - Following: each 4-byte offset entry
/// - Each offset entry points to a payload with 8-byte header: compressed size(u32) + decompressed size(u32)
/// - Payload body is Brotli compressed data, decompressed to JSON
pub(super) struct CompressParser;

impl CompressParser {
    /// Parse compressed bin, return all decompressed data entries
    pub fn parse(data: &[u8]) -> Vec<CompressEntry> {
        let mut results = Vec::new();

        if data.len() < 4 {
            return results;
        }

        let entry_count = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
        let offsets_start = 4;
        let offsets_end = offsets_start + entry_count * 4;

        if offsets_end > data.len() {
            return results;
        }

        for i in 0..entry_count {
            let off_pos = offsets_start + i * 4;
            let payload_offset = u32::from_le_bytes(data[off_pos..off_pos + 4].try_into().unwrap()) as usize;

            if payload_offset + 8 > data.len() {
                continue;
            }

            let compressed_size = u32::from_le_bytes(data[payload_offset..payload_offset + 4].try_into().unwrap()) as usize;
            let _decompressed_size = u32::from_le_bytes(data[payload_offset + 4..payload_offset + 8].try_into().unwrap()) as usize;

            let payload_start = payload_offset + 8;
            let payload_end = payload_start + compressed_size;

            if payload_end > data.len() {
                continue;
            }

            let compressed = &data[payload_start..payload_end];
            if let Some(decompressed) = Self::decompress_brotli(compressed) {
                results.push(CompressEntry { index: i, data: decompressed });
            }
        }

        results
    }

    fn decompress_brotli(data: &[u8]) -> Option<Vec<u8>> {
        let mut decoder = brotli_decompressor::Decompressor::new(data, 4096);
        let mut output = Vec::new();
        decoder.read_to_end(&mut output).ok()?;
        Some(output)
    }
}
