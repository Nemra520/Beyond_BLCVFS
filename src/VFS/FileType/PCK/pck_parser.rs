use super::pck_decipher::decipher_inplace;
use std::io::{Cursor, Read};

pub const AKPK_MAGIC: u32 = 0x4B504B41;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PckSectorType {
    Bank,
    Sound,
    External,
}

#[derive(Debug, Clone)]
pub struct PckLanguage {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct PckFileEntry {
    pub file_id: u64,
    pub size: u32,
    pub offset: i64,
    pub language_id: u32,
    pub sector_type: PckSectorType,
}

#[derive(Debug, Clone)]
pub struct PckContent {
    pub languages: Vec<PckLanguage>,
    pub entries: Vec<PckFileEntry>,
}

pub struct PckParser<'a> {
    data: &'a [u8],
    is_vfs_encrypted: bool,
}

impl<'a> PckParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            is_vfs_encrypted: false,
        }
    }

    pub fn is_vfs_encrypted(&self) -> bool {
        self.is_vfs_encrypted
    }

    fn read_u32_le(data: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ])
    }

    #[allow(dead_code)]
    fn read_u64_le(data: &[u8], offset: usize) -> u64 {
        u64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ])
    }

    fn read_decrypted_header(&mut self) -> Result<Vec<u8>, String> {
        if self.data.len() < 8 {
            return Err("Data too short for PCK header".to_string());
        }

        let magic = Self::read_u32_le(self.data, 0);
        let header_size = Self::read_u32_le(self.data, 4) as usize;

        if header_size < 16 {
            return Err(format!("Header size too small: {}", header_size));
        }

        if 8 + header_size > self.data.len() {
            return Err("Header extends beyond data".to_string());
        }

        let header_content = &self.data[8..8 + header_size];

        if magic == AKPK_MAGIC {
            self.is_vfs_encrypted = false;
            return Ok(header_content.to_vec());
        }

        self.is_vfs_encrypted = true;
        let mut decrypted_payload = header_content[4..].to_vec();
        let payload_len = decrypted_payload.len();
        decipher_inplace(&mut decrypted_payload, header_size as u32, payload_len, 0);

        let mut result = Vec::with_capacity(4 + decrypted_payload.len());
        result.extend_from_slice(&1u32.to_le_bytes());
        result.extend_from_slice(&decrypted_payload);
        Ok(result)
    }

    pub fn parse(&mut self) -> Result<PckContent, String> {
        let header = self.read_decrypted_header()?;
        let mut cursor = Cursor::new(&header[..]);

        let flag = read_u32(&mut cursor)?;
        let big_endian = flag == 0x01000000;
        if big_endian {
            return Err("Big-endian PCK files are not supported".to_string());
        }

        let languages_sector_size = read_u32(&mut cursor)? as usize;
        let banks_sector_size = read_u32(&mut cursor)? as usize;
        let sounds_sector_size = read_u32(&mut cursor)? as usize;

        let overhead: usize = 4 + 4 + 4 + 4;
        let externals_sector_size =
            if languages_sector_size + banks_sector_size + sounds_sector_size + overhead < header.len()
            {
                read_u32(&mut cursor)? as usize
            } else {
                0
            };

        let languages = parse_languages(&mut cursor, languages_sector_size, &header)?;
        let mut entries = Vec::new();
        parse_sector(&mut cursor, banks_sector_size, PckSectorType::Bank, &header, &mut entries)?;
        parse_sector(&mut cursor, sounds_sector_size, PckSectorType::Sound, &header, &mut entries)?;
        if externals_sector_size > 0 {
            parse_sector(&mut cursor, externals_sector_size, PckSectorType::External, &header, &mut entries)?;
        }

        Ok(PckContent {
            languages,
            entries,
        })
    }

    pub fn get_file_data(&self, entry: &PckFileEntry) -> Result<Vec<u8>, String> {
        let offset = entry.offset as usize;
        let size = entry.size as usize;

        if offset + size > self.data.len() {
            return Err(format!(
                "File data extends beyond buffer: offset={}, size={}, total={}",
                offset,
                size,
                self.data.len()
            ));
        }

        let mut data = self.data[offset..offset + size].to_vec();

        if self.is_vfs_encrypted {
            let data_len = data.len();
            decipher_inplace(&mut data, entry.file_id as u32, data_len, 0);
        }

        Ok(data)
    }

    pub fn get_decrypted_pck_bytes(&self) -> Result<Vec<u8>, String> {
        // First, read and decrypt the header to determine if VFS encrypted
        let mut temp_parser = PckParser::new(self.data);
        let header = temp_parser.read_decrypted_header()?;
        let is_vfs_encrypted = temp_parser.is_vfs_encrypted();

        if !is_vfs_encrypted {
            return Ok(self.data.to_vec());
        }

        // Parse the content using the decrypted header
        let mut cursor = Cursor::new(&header[..]);
        let flag = read_u32(&mut cursor)?;
        let big_endian = flag == 0x01000000;
        if big_endian {
            return Err("Big-endian PCK files are not supported".to_string());
        }

        let languages_sector_size = read_u32(&mut cursor)? as usize;
        let banks_sector_size = read_u32(&mut cursor)? as usize;
        let sounds_sector_size = read_u32(&mut cursor)? as usize;

        let overhead: usize = 4 + 4 + 4 + 4;
        let externals_sector_size =
            if languages_sector_size + banks_sector_size + sounds_sector_size + overhead < header.len()
            {
                read_u32(&mut cursor)? as usize
            } else {
                0
            };

        let languages = parse_languages(&mut cursor, languages_sector_size, &header)?;
        let mut entries = Vec::new();
        parse_sector(&mut cursor, banks_sector_size, PckSectorType::Bank, &header, &mut entries)?;
        parse_sector(&mut cursor, sounds_sector_size, PckSectorType::Sound, &header, &mut entries)?;
        if externals_sector_size > 0 {
            parse_sector(&mut cursor, externals_sector_size, PckSectorType::External, &header, &mut entries)?;
        }

        let content = PckContent { languages, entries };

        // Now decrypt the entire PCK
        let mut result = self.data.to_vec();

        if result.len() < 8 + header.len() {
            return Err("PCK buffer is too small for decrypted header".to_string());
        }

        // Write AKPK magic
        let magic_bytes = AKPK_MAGIC.to_le_bytes();
        result[0..4].copy_from_slice(&magic_bytes);
        // Write decrypted header (starts at offset 8, after magic + header_size)
        result[8..8 + header.len()].copy_from_slice(&header);

        // Decrypt each file entry
        for entry in &content.entries {
            if entry.offset < 0 {
                continue;
            }

            let offset = entry.offset as usize;
            let size = entry.size as usize;
            if size == 0 || offset > result.len() - size {
                continue;
            }

            decipher_inplace(
                &mut result[offset..offset + size],
                entry.file_id as u32,
                size,
                0,
            );
        }

        Ok(result)
    }
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, String> {
    let mut buf = [0u8; 4];
    cursor
        .read_exact(&mut buf)
        .map_err(|e| format!("Read error: {}", e))?;
    Ok(u32::from_le_bytes(buf))
}

fn parse_languages(
    cursor: &mut Cursor<&[u8]>,
    sector_size: usize,
    header: &[u8],
) -> Result<Vec<PckLanguage>, String> {
    if sector_size == 0 {
        return Ok(Vec::new());
    }

    let sector_start = cursor.position() as usize;
    let count = read_u32(cursor)? as usize;
    let mut languages = Vec::with_capacity(count);

    for _ in 0..count {
        let name_offset = read_u32(cursor)? as usize;
        let lang_id = read_u32(cursor)?;

        let saved_pos = cursor.position();

        let abs_offset = sector_start + name_offset;
        let name = if abs_offset >= header.len() {
            format!("lang_{}", lang_id)
        } else {
            let b1 = header[abs_offset];
            let b2 = if abs_offset + 1 < header.len() {
                header[abs_offset + 1]
            } else {
                0
            };

            if b1 == 0 || b2 == 0 {
                let raw = &header[abs_offset..std::cmp::min(abs_offset + 0x20, header.len())];
                decode_utf16le(raw)
            } else {
                let raw = &header[abs_offset..std::cmp::min(abs_offset + 0x10, header.len())];
                String::from_utf8_lossy(raw)
                    .trim_end_matches('\0')
                    .to_string()
            }
        };

        languages.push(PckLanguage {
            id: lang_id,
            name,
        });

        cursor.set_position(saved_pos);
    }

    cursor.set_position((sector_start + sector_size) as u64);
    Ok(languages)
}

fn decode_utf16le(data: &[u8]) -> String {
    let mut chars = Vec::new();
    let mut i = 0;
    while i + 1 < data.len() {
        let code_unit = u16::from_le_bytes([data[i], data[i + 1]]);
        if code_unit == 0 {
            break;
        }
        if let Some(ch) = char::from_u32(code_unit as u32) {
            chars.push(ch);
        }
        i += 2;
    }
    chars.into_iter().collect()
}

fn parse_sector(
    cursor: &mut Cursor<&[u8]>,
    sector_size: usize,
    sector_type: PckSectorType,
    _header: &[u8],
    entries: &mut Vec<PckFileEntry>,
) -> Result<(), String> {
    if sector_size == 0 {
        return Ok(());
    }

    let sector_start = cursor.position() as usize;
    let file_count = read_u32(cursor)? as usize;

    if file_count == 0 {
        cursor.set_position((sector_start + sector_size) as u64);
        return Ok(());
    }

    let entry_size = (sector_size - 4) / file_count;
    let alt_mode = entry_size >= 0x18;

    for _ in 0..file_count {
        let file_id: u64;
        if alt_mode && sector_type == PckSectorType::External {
            let id_low = read_u32(cursor)? as u64;
            let id_high = read_u32(cursor)? as u64;
            file_id = id_low | (id_high << 32);
        } else {
            file_id = read_u32(cursor)? as u64;
        }

        let block_size = read_u32(cursor)? as u64;

        let size: u32;
        if alt_mode && sector_type != PckSectorType::External {
            let mut buf = [0u8; 8];
            cursor
                .read_exact(&mut buf)
                .map_err(|e| format!("Read error: {}", e))?;
            size = i64::from_le_bytes(buf) as u32;
        } else {
            size = read_u32(cursor)?;
        }

        let raw_offset = read_u32(cursor)? as u64;
        let language_id = read_u32(cursor)?;

        let offset = if block_size != 0 {
            (raw_offset * block_size) as i64
        } else {
            raw_offset as i64
        };

        entries.push(PckFileEntry {
            file_id,
            size,
            offset,
            language_id,
            sector_type,
        });
    }

    cursor.set_position((sector_start + sector_size) as u64);
    Ok(())
}
