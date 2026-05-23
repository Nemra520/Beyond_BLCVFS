///MIT license https://github.com/Manicsteiner/VGMToolbox/blob/main/LICENSE
/// USM (Criware Stream Media) extractor
/// Extracts video streams from USM files (audio is ignored per requirements)
/// 
/// USM file structure (per C# VGMToolbox CriUsmStream / MpegStream):
/// - Each packet: signature(4) + size(4) + header_size(2) + footer_size(2) + stream_id(1) + ...
/// - Payload starts at: offset + 4(sig) + 4(size) + header_size
/// - Payload ends at: offset + size - footer_size
/// - Video blocks have signature @SFV (0x40534656)
pub struct UsmExtractor;

// Block signatures (big-endian u32)
const SIG_CRID: u32 = 0x43524944; // "CRID"
const SIG_SFV: u32 = 0x40534656; // "@SFV" - video block
const SIG_SFA: u32 = 0x40534641; // "@SFA" - audio block
const SIG_ALP: u32 = 0x40414C50; // "@ALP"
const SIG_SBT: u32 = 0x40534254; // "@SBT"
const SIG_CUE: u32 = 0x40435545; // "@CUE"

// Header/footer markers (in the combined video stream, 32 bytes each)
const HEADER_END_BYTES: &[u8] = &[
    0x23, 0x48, 0x45, 0x41, 0x44, 0x45, 0x52, 0x20,
    0x45, 0x4E, 0x44, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D,
    0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x00,
];
const METADATA_END_BYTES: &[u8] = &[
    0x23, 0x4D, 0x45, 0x54, 0x41, 0x44, 0x41, 0x54,
    0x41, 0x20, 0x45, 0x4E, 0x44, 0x20, 0x20, 0x20,
    0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D,
    0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x00,
];
const CONTENTS_END_BYTES: &[u8] = &[
    0x23, 0x43, 0x4F, 0x4E, 0x54, 0x45, 0x4E, 0x54,
    0x53, 0x20, 0x45, 0x4E, 0x44, 0x20, 0x20, 0x20,
    0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D,
    0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x3D, 0x00,
];

impl UsmExtractor {
    /// Extract video from USM data, returns raw video bytes (.m2v)
    pub fn extract_video(data: &[u8]) -> Option<Vec<u8>> {
        let offsets = Self::find_sfv_offsets(data);

        if offsets.is_empty() {
            eprintln!("[USM] No @SFV blocks found");
            return None;
        }

        eprintln!("[USM] Found {} @SFV blocks", offsets.len());

        // Extract payload from each @SFV block and concatenate
        let mut video_data = Vec::new();
        for (i, &offset) in offsets.iter().enumerate() {
            if let Some(payload) = Self::extract_sfv_payload(data, offset) {
                eprintln!(
                    "[USM] @SFV #{}: offset={}, payload_size={}",
                    i,
                    offset,
                    payload.len()
                );
                video_data.extend_from_slice(payload);
            } else {
                eprintln!("[USM] @SFV #{}: offset={}, failed to extract payload", i, offset);
            }
        }

        if video_data.is_empty() {
            return None;
        }

        eprintln!(
            "[USM] Total concatenated payload: {} bytes, first 4 bytes: {:02X?}",
            video_data.len(),
            &video_data[..4.min(video_data.len())]
        );

        // Strip header/footer markers from combined data
        Some(Self::strip_markers(&video_data))
    }

    /// Find all @SFV block offsets in the file
    fn find_sfv_offsets(data: &[u8]) -> Vec<usize> {
        let mut offsets = Vec::new();
        let mut pos = 0;
        let sfv_bytes: [u8; 4] = SIG_SFV.to_be_bytes();

        while pos + 8 <= data.len() {
            if let Some(idx) = Self::find_bytes(data, &sfv_bytes, pos) {
                offsets.push(idx);
                pos = idx + 4;
            } else {
                break;
            }
        }

        offsets
    }

    /// Extract payload from an @SFV block at given offset
    ///
    /// C# MpegStream logic for video block:
    ///   blockSize = size at offset+4 (4 bytes BE)
    ///   videoBlockSkipSize = header_size at offset+8 (2 bytes BE)
    ///   videoBlockFooterSize = footer_size at offset+10 (2 bytes BE)
    ///   write_start = currentOffset + 4(blockId) + 4(blockSize) + videoBlockSkipSize
    ///   write_length = blockSize - videoBlockSkipSize - videoBlockFooterSize
    fn extract_sfv_payload<'a>(data: &'a [u8], offset: usize) -> Option<&'a [u8]> {
        if offset + 12 > data.len() {
            return None;
        }

        let block_size = Self::read_u32_be(data, offset + 4)? as usize;
        let header_size = Self::read_u16_be(data, offset + 8)? as usize;
        let footer_size = Self::read_u16_be(data, offset + 10)? as usize;

        // C#: cutSize = blockSize - videoBlockSkipSize - videoBlockFooterSize
        // Also need block_size >= header_size + footer_size
        if block_size < header_size + footer_size {
            return None;
        }

        // C# write start: currentOffset + currentBlockId.Length(4) + blockSizeArray.Length(4) + videoBlockSkipSize
        let payload_start = offset + 4 + 4 + header_size;
        // C# write length: blockSize - videoBlockSkipSize - videoBlockFooterSize
        let payload_len = block_size - header_size - footer_size;
        let payload_end = payload_start + payload_len;

        if payload_start > data.len() || payload_end > data.len() {
            return None;
        }

        if payload_len == 0 {
            return None;
        }

        Some(&data[payload_start..payload_end])
    }

    /// Strip header/footer markers from video data
    ///
    /// Matches VGMToolbox CriUsmStream.DoFinalTasks logic exactly:
    /// 1. Find both #HEADER END and #METADATA END in the concatenated video data
    /// 2. If metadata_end_offset > header_end_offset: header_size = metadata_end_offset + 32
    ///    Else: header_size = header_end_offset + 32
    /// 3. Find #CONTENTS END, footer_offset = contents_end_offset - header_size
    ///    footer_size = data_len - footer_offset
    /// 4. Remove header [0..header_size] and footer [footer_offset..end]
    fn strip_markers(data: &[u8]) -> Vec<u8> {
        // Find both markers
        let header_end_offset = Self::find_marker(data, HEADER_END_BYTES);
        let metadata_end_offset = Self::find_marker(data, METADATA_END_BYTES);

        eprintln!(
            "[USM] strip_markers: #HEADER END at {:?}, #METADATA END at {:?}",
            header_end_offset, metadata_end_offset
        );

        // C# logic:
        // if (metadataEndOffset > headerEndOffset) headerSize = metadataEndOffset + 32;
        // else headerSize = headerEndOffset + 32;
        let header_size = match (header_end_offset, metadata_end_offset) {
            (Some(h), Some(m)) => {
                if m > h {
                    m + METADATA_END_BYTES.len()
                } else {
                    h + HEADER_END_BYTES.len()
                }
            }
            (Some(h), None) => h + HEADER_END_BYTES.len(),
            (None, Some(m)) => m + METADATA_END_BYTES.len(),
            (None, None) => 0,
        };

        // C# logic for footer:
        // footerOffset = ParseFile.GetNextOffset(outputFiles[streamId], 0, CONTENTS_END_BYTES) - headerSize;
        // footerSize = outputFiles[streamId].Length - footerOffset;
        // Then: RemoveChunkFromFile(sourceFileName, footerOffset, footerSize)
        //
        // This means: after removing header, find CONTENTS_END in the remaining data.
        // footerOffset is relative to the data-after-header.
        // So actual contents_end_offset in original data = header_size + footerOffset
        // Which is just the position where CONTENTS END marker starts.
        // The video data we want is from header_size to contents_end_offset.
        let contents_end_offset = Self::find_marker(data, CONTENTS_END_BYTES);

        eprintln!(
            "[USM] strip_markers: #CONTENTS END at {:?}, header_size={}, data_len={}",
            contents_end_offset,
            header_size,
            data.len()
        );

        let end_pos = contents_end_offset.unwrap_or(data.len());

        if header_size > 0 && header_size < end_pos && end_pos <= data.len() {
            eprintln!(
                "[USM] strip_markers: returning data[{}..{}] ({} bytes)",
                header_size,
                end_pos,
                end_pos - header_size
            );
            data[header_size..end_pos].to_vec()
        } else {
            eprintln!(
                "[USM] strip_markers: no markers stripped, returning full data ({} bytes)",
                data.len()
            );
            data.to_vec()
        }
    }

    /// Find byte pattern in data starting from position
    fn find_bytes(data: &[u8], pattern: &[u8], start: usize) -> Option<usize> {
        if start + pattern.len() > data.len() {
            return None;
        }
        data[start..]
            .windows(pattern.len())
            .position(|w| w == pattern)
            .map(|p| start + p)
    }

    /// Find marker in data (exact match)
    fn find_marker(data: &[u8], marker: &[u8]) -> Option<usize> {
        data.windows(marker.len()).position(|w| w == marker)
    }

    fn read_u32_be(data: &[u8], offset: usize) -> Option<u32> {
        data.get(offset..offset + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_u16_be(data: &[u8], offset: usize) -> Option<u16> {
        data.get(offset..offset + 2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
    }
}
