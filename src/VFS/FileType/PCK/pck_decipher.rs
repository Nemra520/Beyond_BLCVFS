const CONST_M: u32 = 0x04E11C23;
const CONST_X: u32 = 0x9C5A0B29;

fn generate_key(counter: u32) -> u32 {
    let mut val = ((counter & 0xFF) ^ CONST_X).wrapping_mul(CONST_M);
    val = (val ^ (((counter >> 8) & 0xFF))).wrapping_mul(CONST_M);
    val = (val ^ (((counter >> 16) & 0xFF))).wrapping_mul(CONST_M);
    val = (val ^ (((counter >> 24) & 0xFF))).wrapping_mul(CONST_M);
    val
}

pub fn decipher_inplace(data: &mut [u8], seed: u32, size: usize, offset_to_file_start: usize) {
    if size == 0 {
        return;
    }

    let mut pos: usize = 0;
    let mut base_counter = seed.wrapping_add((offset_to_file_start >> 2) as u32);
    let aligned_offset = offset_to_file_start & 0b11;

    if aligned_offset > 0 {
        let key = generate_key(base_counter);
        let key_bytes = key.to_le_bytes();
        let bytes_leading = std::cmp::min(4 - aligned_offset, size);
        for i in 0..bytes_leading {
            data[pos] ^= key_bytes[aligned_offset + i];
            pos += 1;
        }
        base_counter = base_counter.wrapping_add(1);
    }

    let aligned_size = (size - pos) & !0b11;
    let num_blocks = aligned_size / 4;
    for block_idx in 0..num_blocks {
        let key = generate_key(base_counter.wrapping_add(block_idx as u32));
        let key_bytes = key.to_le_bytes();
        for i in 0..4 {
            data[pos + i] ^= key_bytes[i];
        }
        pos += 4;
    }

    if pos < size {
        let key = generate_key(base_counter.wrapping_add(num_blocks as u32));
        let key_bytes = key.to_le_bytes();
        let bytes_remaining = size - pos;
        for i in 0..bytes_remaining {
            data[pos + i] ^= key_bytes[i];
        }
    }
}
