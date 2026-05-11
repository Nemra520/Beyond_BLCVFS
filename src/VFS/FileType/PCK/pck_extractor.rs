use super::pck_parser::{PckContent, PckParser};
use crate::VFS::BLC::{BlcError, Result};
use std::path::{Path, PathBuf};

pub struct PckExtractor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PckEntryType {
    Bnk,
    Wem,
    WemX,
    Plg,
    Unknown,
}

impl PckEntryType {
    pub fn detect(magic: &[u8]) -> Self {
        if magic.len() < 4 {
            return Self::Unknown;
        }
        match magic {
            b"BKHD" => Self::Bnk,
            b"RIFF" | b"RIFX" => Self::Wem,
            b"RVPK" => Self::WemX,
            b"PLUG" => Self::Plg,
            _ => Self::Unknown,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Bnk => ".bnk",
            Self::Wem => ".wem",
            Self::WemX => ".wem",
            Self::Plg => ".plg",
            Self::Unknown => ".bin",
        }
    }
}

pub struct PckExtractResult {
    pub content: PckContent,
    pub is_vfs_encrypted: bool,
    pub entries: Vec<PckExtractEntry>,
}

pub struct PckExtractEntry {
    pub file_id: u64,
    pub entry_type: PckEntryType,
    pub data: Vec<u8>,
    pub language_id: u32,
}

impl PckExtractor {
    pub fn parse_pck(data: &[u8]) -> Result<PckContent> {
        let mut parser = PckParser::new(data);
        parser
            .parse()
            .map_err(|e| BlcError::InvalidFormat(format!("PCK parse error: {}", e)))
    }

    pub fn extract_pck(data: &[u8]) -> Result<PckExtractResult> {
        let mut parser = PckParser::new(data);
        let content = parser
            .parse()
            .map_err(|e| BlcError::InvalidFormat(format!("PCK parse error: {}", e)))?;

        let is_vfs_encrypted = parser.is_vfs_encrypted();

        let mut entries = Vec::new();
        for entry in &content.entries {
            let file_data = parser
                .get_file_data(entry)
                .map_err(|e| BlcError::InvalidFormat(format!("File data error: {}", e)))?;

            if file_data.len() < 4 {
                continue;
            }

            let entry_type = PckEntryType::detect(&file_data[..4]);

            entries.push(PckExtractEntry {
                file_id: entry.file_id,
                entry_type,
                data: file_data,
                language_id: entry.language_id,
            });
        }

        Ok(PckExtractResult {
            content,
            is_vfs_encrypted,
            entries,
        })
    }

    pub fn get_decrypted_pck(data: &[u8]) -> Result<Vec<u8>> {
        let parser = PckParser::new(data);
        parser
            .get_decrypted_pck_bytes()
            .map_err(|e| BlcError::InvalidFormat(format!("PCK decrypt error: {}", e)))
    }

    pub fn extract_to_dir(
        data: &[u8],
        output_dir: &Path,
        file_name: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        let decrypted_pck = Self::get_decrypted_pck(data)?;
        let name = file_name.unwrap_or("output.pck");
        let output_path = output_dir.join(name);

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| BlcError::Io(e))?;
        }

        std::fs::write(&output_path, &decrypted_pck)
            .map_err(|e| BlcError::Io(e))?;

        Ok(vec![output_path])
    }

    pub fn extract_entries(
        data: &[u8],
        output_dir: &Path,
        prefix: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        let result = Self::extract_pck(data)?;
        let mut saved_paths = Vec::new();

        let base_dir = if let Some(pfx) = prefix {
            output_dir.join(pfx)
        } else {
            output_dir.to_path_buf()
        };

        for entry in &result.entries {
            let file_name = format!(
                "{}{}",
                entry.file_id,
                entry.entry_type.extension()
            );
            let path = base_dir.join(&file_name);

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| BlcError::Io(e))?;
            }

            std::fs::write(&path, &entry.data)
                .map_err(|e| BlcError::Io(e))?;

            saved_paths.push(path);
        }

        Ok(saved_paths)
    }
}

pub fn resolve_language_name(languages: &[super::pck_parser::PckLanguage], language_id: u32) -> String {
    if language_id == 0 {
        return "SFX".to_string();
    }
    for lang in languages {
        if lang.id == language_id {
            return lang.name.clone();
        }
    }
    format!("lang_{}", language_id)
}
