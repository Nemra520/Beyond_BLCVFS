use std::collections::HashMap;

use super::types::{MappingEntry, StringPathHeader};

pub(super) struct PathBytesParser;

impl PathBytesParser {
    /// Parse StringPath binary data, return hash -> path mapping
    pub fn parse(data: &[u8]) -> HashMap<i64, String> {
        let mut result = HashMap::new();

        if data.len() < 8 {
            return result;
        }

        // Read header
        let header = Self::read_header(data);
        let slots_size = header.capacity as usize * 8;

        // Skip slots region, read nodes
        let nodes_start = 8 + slots_size;
        let nodes_end = header.string_pool_offset as usize;

        if nodes_start > data.len() || nodes_end > data.len() || nodes_start > nodes_end {
            return result;
        }

        // Each node is 16 bytes: hash(i64) + offset(i32) + padding(i32)
        let node_count = (nodes_end - nodes_start) / 16;
        let mut entries = Vec::with_capacity(node_count);

        for i in 0..node_count {
            let off = nodes_start + i * 16;
            if off + 16 > data.len() {
                break;
            }
            let hash = i64::from_le_bytes(data[off..off + 8].try_into().unwrap());
            let offset = i32::from_le_bytes(data[off + 8..off + 12].try_into().unwrap());
            // Skip 4 bytes padding
            entries.push(MappingEntry { hash, offset });
        }

        // Read strings from string pool based on offset
        let pool_base = header.string_pool_offset as usize;
        for entry in entries {
            if let Some(s) = Self::read_string_at(data, pool_base, entry.offset) {
                result.insert(entry.hash, s);
            }
        }

        result
    }

    fn read_header(data: &[u8]) -> StringPathHeader {
        let string_pool_offset = i32::from_le_bytes(data[0..4].try_into().unwrap());
        let capacity = i32::from_le_bytes(data[4..8].try_into().unwrap());
        StringPathHeader {
            string_pool_offset,
            capacity,
        }
    }

    /// Read UTF-16LE string at base + offset position in string pool
    /// Format: len(i32) + len bytes of UTF-16LE data
    fn read_string_at(data: &[u8], base: usize, offset: i32) -> Option<String> {
        let pos = base + offset as usize;
        if pos + 4 > data.len() {
            return None;
        }

        let len = i32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        let str_start = pos + 4;
        let str_end = str_start + len;

        if str_end > data.len() {
            return None;
        }

        let raw = &data[str_start..str_end];
        // Decode UTF-16LE manually
        let u16_iter = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]));
        Some(String::from_utf16_lossy(u16_iter.collect::<Vec<u16>>().as_slice()))
    }
}
