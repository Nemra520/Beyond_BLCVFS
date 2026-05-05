use super::types::*;
use super::error::Result;
use std::io::{Read, Cursor};
use byteorder::{LittleEndian, ReadBytesExt};

pub struct BlcParser;

impl BlcParser {
    pub fn parse(data: &[u8]) -> Result<BlcMainInfo> {
        let mut cursor = Cursor::new(data);
        Self::parse_main_info(&mut cursor)
    }
    
    fn parse_main_info(cursor: &mut Cursor<&[u8]>) -> Result<BlcMainInfo> {
        let version = cursor.read_i32::<LittleEndian>()?;
        let group_cfg_hash_name = cursor.read_u32::<LittleEndian>()?;
        
        let name_len = cursor.read_u16::<LittleEndian>()? as usize;
        let group_cfg_name = if name_len > 0 {
            let mut name_buf = vec![0u8; name_len];
            cursor.read_exact(&mut name_buf)?;
            String::from_utf8_lossy(&name_buf).to_string()
        } else {
            String::new()
        };
        
        cursor.read_u64::<LittleEndian>()?;
        
        let group_file_info_num = cursor.read_i32::<LittleEndian>()?;
        let group_chunks_length = cursor.read_i64::<LittleEndian>()?;
        let block_type = cursor.read_u8()?;
        let all_chunks_count = cursor.read_i32::<LittleEndian>()?;
        
        let mut all_chunks = Vec::with_capacity(all_chunks_count as usize);
        for _ in 0..all_chunks_count {
            all_chunks.push(Self::parse_chunk_info(cursor)?);
        }
        
        Ok(BlcMainInfo {
            version,
            group_cfg_name,
            group_cfg_hash_name,
            group_file_info_num,
            group_chunks_length,
            block_type,
            all_chunks_count,
            all_chunks,
        })
    }
    
    fn parse_chunk_info(cursor: &mut Cursor<&[u8]>) -> Result<ChunkInfo> {
        let mut md5_name_bytes = [0u8; 16];
        cursor.read_exact(&mut md5_name_bytes)?;
        let md5_name = UInt128::from_bytes(md5_name_bytes);
        
        let mut content_md5_bytes = [0u8; 16];
        cursor.read_exact(&mut content_md5_bytes)?;
        let content_md5 = UInt128::from_bytes(content_md5_bytes);
        
        let length = cursor.read_i64::<LittleEndian>()?;
        let block_type = cursor.read_u8()?;
        let file_tag = cursor.read_i32::<LittleEndian>()?;
        let files_count = cursor.read_i32::<LittleEndian>()?;
        
        let mut files = Vec::with_capacity(files_count as usize);
        for _ in 0..files_count {
            files.push(Self::parse_file_info(cursor)?);
        }
        
        Ok(ChunkInfo {
            md5_name,
            content_md5,
            length,
            block_type,
            file_tag,
            files_count,
            files,
        })
    }
    
    fn parse_file_info(cursor: &mut Cursor<&[u8]>) -> Result<FileInfo> {
        let name_len = cursor.read_u16::<LittleEndian>()? as usize;
        let file_name = if name_len > 0 {
            let mut name_buf = vec![0u8; name_len];
            cursor.read_exact(&mut name_buf)?;
            String::from_utf8_lossy(&name_buf).to_string()
        } else {
            String::new()
        };
        
        let file_name_hash = cursor.read_u64::<LittleEndian>()?;
        
        let mut file_chunk_md5_name_bytes = [0u8; 16];
        cursor.read_exact(&mut file_chunk_md5_name_bytes)?;
        let file_chunk_md5_name = UInt128::from_bytes(file_chunk_md5_name_bytes);
        
        let mut file_data_md5_bytes = [0u8; 16];
        cursor.read_exact(&mut file_data_md5_bytes)?;
        let file_data_md5 = UInt128::from_bytes(file_data_md5_bytes);
        
        let offset = cursor.read_i64::<LittleEndian>()?;
        let len = cursor.read_i64::<LittleEndian>()?;
        let block_type = cursor.read_u8()?;
        let b_use_encrypt = cursor.read_u8()? != 0;
        
        let iv_seed = if b_use_encrypt {
            Some(cursor.read_i64::<LittleEndian>()?)
        } else {
            None
        };
        
        let file_tag = cursor.read_i32::<LittleEndian>()?;
        
        Ok(FileInfo {
            file_name,
            file_name_hash,
            file_chunk_md5_name,
            file_data_md5,
            offset,
            len,
            block_type,
            b_use_encrypt,
            iv_seed,
            file_tag,
        })
    }
}
