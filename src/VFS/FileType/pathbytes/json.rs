use std::collections::HashMap;

use super::compress_parser::CompressParser;
use super::parser::PathBytesParser as InnerParser;

/// Compressed bin export entry
pub struct CompressExportEntry {
    pub index: usize,
    pub filename: String,
    pub data: Vec<u8>,
}

/// StringPath .bin file parser
pub struct PathBytesParser;

impl PathBytesParser {
    /// Parse .bin file data, return formatted JSON string
    /// When filename contains "compress", use compressed parsing path, otherwise use standard StringPath parsing
    pub fn parse_to_json(data: &[u8], filename: &str) -> String {
        if filename.to_lowercase().contains("compress") {
            Self::parse_compress_to_json(data)
        } else {
            let map = InnerParser::parse(data);
            serde_json::to_string_pretty(&map).unwrap_or_else(|_| "{}".to_string())
        }
    }

    /// Parse .bin file data, return hash -> path mapping (standard StringPath format only)
    pub fn parse_to_map(data: &[u8]) -> HashMap<i64, String> {
        InnerParser::parse(data)
    }

    /// Parse compressed bin, return all export entries (for GUI export)
    /// Each entry's data is already formatted JSON bytes (UTF-8)
    pub fn parse_compress_entries(data: &[u8]) -> Vec<CompressExportEntry> {
        let entries = CompressParser::parse(data);
        entries
            .into_iter()
            .map(|e| {
                let formatted = Self::format_json(&e.data);
                CompressExportEntry {
                    filename: format!("{}.json", e.index),
                    index: e.index,
                    data: formatted,
                }
            })
            .collect()
    }

    /// Format byte data to pretty JSON, handling both UTF-8 and UTF-16LE encoding
    /// Returns UTF-8 formatted JSON bytes
    fn format_json(data: &[u8]) -> Vec<u8> {
        // First try to decode as UTF-16LE (common in Unity/Mono games)
        let json_str = Self::decode_utf16le(data)
            .or_else(|| String::from_utf8(data.to_vec()).ok())
            .unwrap_or_else(|| String::from_utf8_lossy(data).to_string());

        // Try to parse and re-format as pretty JSON
        match serde_json::from_str::<serde_json::Value>(&json_str) {
            Ok(v) => serde_json::to_string_pretty(&v)
                .unwrap_or(json_str)
                .into_bytes(),
            Err(_) => json_str.into_bytes(),
        }
    }

    /// Decode UTF-16LE bytes to String
    fn decode_utf16le(data: &[u8]) -> Option<String> {
        if data.len() < 2 || data.len() % 2 != 0 {
            return None;
        }
        // Check if it looks like UTF-16LE (null bytes at odd positions)
        let null_count = data.iter().skip(1).step_by(2).filter(|&&b| b == 0).count();
        if null_count < data.len() / 4 {
            // Not enough null bytes, probably not UTF-16LE
            return None;
        }
        let u16_iter = data.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]));
        String::from_utf16(&u16_iter.collect::<Vec<u16>>()).ok()
    }

    /// Parse compressed bin, merge and format all decompressed JSON (simple mode)
    fn parse_compress_to_json(data: &[u8]) -> String {
        let entries = CompressParser::parse(data);
        if entries.is_empty() {
            return "{}".to_string();
        }
        if entries.len() == 1 {
            let json_str = Self::decode_utf16le(&entries[0].data)
                .or_else(|| String::from_utf8(entries[0].data.clone()).ok())
                .unwrap_or_else(|| String::from_utf8_lossy(&entries[0].data).to_string());
            match serde_json::from_str::<serde_json::Value>(&json_str) {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or(json_str),
                Err(_) => json_str,
            }
        } else {
            let values: Vec<serde_json::Value> = entries
                .iter()
                .filter_map(|e| {
                    let s = Self::decode_utf16le(&e.data)
                        .or_else(|| String::from_utf8(e.data.clone()).ok())?;
                    serde_json::from_str(&s).ok()
                })
                .collect();
            serde_json::to_string_pretty(&values).unwrap_or_else(|_| "[]".to_string())
        }
    }
}
