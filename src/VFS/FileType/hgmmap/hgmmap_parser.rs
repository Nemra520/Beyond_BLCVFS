use std::io::{Cursor, Read};
use byteorder::{LittleEndian, ReadBytesExt};

use super::types::{Bundle, AssetInfo, ManifestScheme};
use super::json::scheme_to_json;

const HEAD1: u32 = 0xFF11FF11;
const HEAD2: u32 = 0xF1F2F3F4;

pub struct HgmmapParser;

impl HgmmapParser {
    /// Parse hgmmap file data and return ManifestScheme
    /// Data should be decrypted Brotli-compressed data
    pub fn parse(data: &[u8]) -> Result<ManifestScheme, String> {
        let decompressed = Self::decompress_data(data)?;
        Self::parse_binary(&decompressed)
    }

    /// Parse hgmmap file data and return JSON string
    pub fn parse_to_json(data: &[u8]) -> String {
        match Self::parse(data) {
            Ok(scheme) => {
                let value = scheme_to_json(&scheme);
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
            }
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    /// Decompress Brotli data
    fn decompress_data(data: &[u8]) -> Result<Vec<u8>, String> {
        const MAX_OUTPUT_SIZE: usize = 500 * 1024 * 1024;

        let mut decoder = brotli_decompressor::Decompressor::new(Cursor::new(data), 4096);
        let mut result = Vec::with_capacity(data.len() * 10);

        let mut buffer = [0u8; 8192];
        loop {
            match decoder.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if result.len() + n > MAX_OUTPUT_SIZE {
                        return Err(format!("Decompressed data exceeds maximum size of {} bytes", MAX_OUTPUT_SIZE));
                    }
                    result.extend_from_slice(&buffer[..n]);
                }
                Err(e) => {
                    return Err(format!("Brotli decompression failed: {}", e));
                }
            }
        }

        Ok(result)
    }

    /// Parse binary data
    fn parse_binary(data: &[u8]) -> Result<ManifestScheme, String> {
        let mut cursor = Cursor::new(data);

        // Check HEAD1
        let head1 = cursor.read_u32::<LittleEndian>()
            .map_err(|e| format!("Failed to read HEAD1: {}", e))?;
        if head1 != HEAD1 {
            return Err(format!("Invalid Manifest HEAD1: expected 0x{:08X}, got 0x{:08X}", HEAD1, head1));
        }

        // Read version
        let version = Self::read_len_unicode_string(&mut cursor)?;

        // Check HEAD2
        let head2 = cursor.read_u32::<LittleEndian>()
            .map_err(|e| format!("Failed to read HEAD2: {}", e))?;
        if head2 != HEAD2 {
            return Err(format!("Invalid Manifest HEAD2: expected 0x{:08X}, got 0x{:08X}", HEAD2, head2));
        }

        // Read Hash
        let hash = Self::read_len_unicode_string(&mut cursor)?;

        // Read perforceCL
        let perforce_cl = Self::read_len_unicode_string(&mut cursor)?;

        // Read AssetInfo region size and skip
        let asset_info_size = cursor.read_i32::<LittleEndian>()
            .map_err(|e| format!("Failed to read assetInfoSize: {}", e))?;
        let asset_info_address = cursor.position() as i64;
        cursor.set_position((asset_info_address + asset_info_size as i64) as u64);

        // Read Bundle region size and skip
        let bundle_size = cursor.read_i32::<LittleEndian>()
            .map_err(|e| format!("Failed to read bundleSize: {}", e))?;
        let bundle_address = cursor.position() as i64;
        cursor.set_position((bundle_address + bundle_size as i64) as u64);

        // Read BundleArray region size
        let bundle_array_size = cursor.read_i32::<LittleEndian>()
            .map_err(|e| format!("Failed to read bundleArraySize: {}", e))?;
        let bundle_array_address = cursor.position() as i64;

        // Data region address
        let data_address = bundle_array_address + bundle_array_size as i64 + 4;

        // Parse Bundles
        let mut bundles = Vec::new();
        cursor.set_position(bundle_array_address as u64);
        let bundle_count = cursor.read_i32::<LittleEndian>()
            .map_err(|e| format!("Failed to read bundleCount: {}", e))?;

        for _ in 0..bundle_count {
            let bundle_index = cursor.read_i32::<LittleEndian>()
                .map_err(|e| format!("Failed to read bundleIndex: {}", e))?;
            let name_offset = cursor.read_i32::<LittleEndian>()
                .map_err(|e| format!("Failed to read nameOffset: {}", e))?;
            let deps_offset = cursor.read_i32::<LittleEndian>()
                .map_err(|e| format!("Failed to read depsOffset: {}", e))?;
            let rev_deps_offset = cursor.read_i32::<LittleEndian>()
                .map_err(|e| format!("Failed to read revDepsOffset: {}", e))?;
            let dir_deps_offset = cursor.read_i32::<LittleEndian>()
                .map_err(|e| format!("Failed to read dirDepsOffset: {}", e))?;
            let bundle_flags = cursor.read_i32::<LittleEndian>()
                .map_err(|e| format!("Failed to read bundleFlags: {}", e))?;
            let hash_name = cursor.read_i64::<LittleEndian>()
                .map_err(|e| format!("Failed to read hashName: {}", e))?;
            let hash_version = cursor.read_i64::<LittleEndian>()
                .map_err(|e| format!("Failed to read hashVersion: {}", e))?;
            let category = cursor.read_i32::<LittleEndian>()
                .map_err(|e| format!("Failed to read category: {}", e))?;
            cursor.read_i32::<LittleEndian>()
                .map_err(|e| format!("Failed to read padding: {}", e))?;

            let curr_pos = cursor.position();

            let name = Self::read_string_at(data, data_address as usize, name_offset as usize);
            let dependencies = Self::read_int_array_at(data, data_address as usize, deps_offset as usize);
            let direct_reverse_dependencies = Self::read_int_array_at(data, data_address as usize, rev_deps_offset as usize);
            let direct_dependencies = Self::read_int_array_at(data, data_address as usize, dir_deps_offset as usize);

            bundles.push(Bundle {
                bundle_index,
                name,
                dependencies,
                direct_reverse_dependencies,
                direct_dependencies,
                bundle_flags,
                hash_name,
                hash_version,
                category,
            });

            cursor.set_position(curr_pos);
        }

        // Parse Assets
        let mut assets = Vec::new();
        cursor.set_position(asset_info_address as u64);
        let asset_capacity = cursor.read_i32::<LittleEndian>()
            .map_err(|e| format!("Failed to read assetCapacity: {}", e))?;

        // Skip buckets (capacity * 8 bytes)
        cursor.set_position((cursor.position() + asset_capacity as u64 * 8) as u64);

        // Calculate asset count
        let asset_count = ((asset_info_address + asset_info_size as i64) - cursor.position() as i64) / 24;

        for _ in 0..asset_count {
            let path_hash_head = cursor.read_i64::<LittleEndian>()
                .map_err(|e| format!("Failed to read pathHashHead: {}", e))?;
            let path_offset = cursor.read_i32::<LittleEndian>()
                .map_err(|e| format!("Failed to read pathOffset: {}", e))?;
            let bundle_index = cursor.read_i32::<LittleEndian>()
                .map_err(|e| format!("Failed to read bundleIndex: {}", e))?;
            let asset_size = cursor.read_i32::<LittleEndian>()
                .map_err(|e| format!("Failed to read assetSize: {}", e))?;
            cursor.read_i32::<LittleEndian>()
                .map_err(|e| format!("Failed to read padding: {}", e))?;

            let path = Self::read_compress_string_at(data, data_address as usize, path_offset as usize)?;

            assets.push(AssetInfo {
                path_hash_head,
                path,
                bundle_index,
                asset_size,
            });
        }

        Ok(ManifestScheme {
            version,
            hash,
            perforce_cl,
            m_asset_info_address: asset_info_address,
            m_bundle_address: bundle_address,
            m_bundle_array_address: bundle_array_address,
            m_data_address: data_address,
            bundles,
            assets,
        })
    }

    /// Read length-prefixed Unicode string
    fn read_len_unicode_string(cursor: &mut Cursor<&[u8]>) -> Result<String, String> {
        let len = cursor.read_i32::<LittleEndian>()
            .map_err(|e| format!("Failed to read string length: {}", e))? as usize;
        let mut bytes = vec![0u8; len * 2];
        cursor.read_exact(&mut bytes)
            .map_err(|e| format!("Failed to read string bytes: {}", e))?;
        from_utf16le(&bytes)
            .map_err(|e| format!("Failed to decode UTF-16LE: {}", e))
    }

    /// Read string at specified position
    fn read_string_at(data: &[u8], data_pos: usize, offset: usize) -> String {
        let pos = data_pos + offset;
        if pos + 4 > data.len() {
            return String::new();
        }
        let len = i32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        if pos + 4 + len > data.len() {
            return String::new();
        }
        let bytes = &data[pos + 4..pos + 4 + len];
        from_utf16le(bytes).unwrap_or_default()
    }

    /// Read compressed string at specified position
    fn read_compress_string_at(data: &[u8], data_pos: usize, offset: usize) -> Result<String, String> {
        let pos = data_pos + offset;
        if pos + 4 > data.len() {
            return Ok(String::new());
        }
        let compress_len = i32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        if pos + 4 + compress_len > data.len() {
            return Ok(String::new());
        }
        let raw_data = &data[pos + 4..pos + 4 + compress_len];
        let decompressed = Self::decompress_data(raw_data)?;
        from_utf16le(&decompressed)
            .map_err(|e| format!("Failed to decode compressed UTF-16LE: {}", e))
    }

    /// Read int array at specified position
    fn read_int_array_at(data: &[u8], data_pos: usize, offset: usize) -> Vec<i32> {
        let pos = data_pos + offset;
        if pos + 4 > data.len() {
            return Vec::new();
        }
        let count = i32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            let val_pos = pos + 4 + i * 4;
            if val_pos + 4 > data.len() {
                break;
            }
            let val = i32::from_le_bytes(data[val_pos..val_pos + 4].try_into().unwrap());
            result.push(val);
        }
        result
    }

}

/// Decode UTF-16LE bytes to String
fn from_utf16le(v: &[u8]) -> Result<String, std::string::FromUtf16Error> {
    let u16_slice: Vec<u16> = v.chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    String::from_utf16(&u16_slice)
}
