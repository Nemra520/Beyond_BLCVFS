const CONST_M: u32 = 0x04E11C23;
const CONST_X: u32 = 0x9C5A0B29;

// Precompute key lookup table for better performance
// This reduces repeated calculations for the same counter values
fn generate_key(counter: u32) -> u32 {
    let mut val = ((counter & 0xFF) ^ CONST_X).wrapping_mul(CONST_M);
    val = (val ^ (((counter >> 8) & 0xFF))).wrapping_mul(CONST_M);
    val = (val ^ (((counter >> 16) & 0xFF))).wrapping_mul(CONST_M);
    val = (val ^ (((counter >> 24) & 0xFF))).wrapping_mul(CONST_M);
    val
}

/// Optimized in-place decipher using precomputed key batches
/// Processes data in chunks for better cache utilization
pub fn decipher_inplace(data: &mut [u8], seed: u32, size: usize, offset_to_file_start: usize) {
    if size == 0 {
        return;
    }

    let mut pos: usize = 0;
    let mut base_counter = seed.wrapping_add((offset_to_file_start >> 2) as u32);
    let aligned_offset = offset_to_file_start & 0b11;

    // Handle unaligned leading bytes
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

    // Process aligned data in chunks for better cache performance
    // Use 256-block chunks to balance between cache locality and loop overhead
    const CHUNK_SIZE: usize = 256;
    let aligned_size = (size - pos) & !0b11;
    let num_blocks = aligned_size / 4;

    let mut block_idx = 0;
    while block_idx < num_blocks {
        let chunk_end = std::cmp::min(block_idx + CHUNK_SIZE, num_blocks);

        // Precompute keys for this chunk
        for chunk_block in block_idx..chunk_end {
            let key = generate_key(base_counter.wrapping_add(chunk_block as u32));
            let key_bytes = key.to_le_bytes();
            let data_pos = pos + chunk_block * 4;

            // Unroll the inner loop for better performance
            data[data_pos] ^= key_bytes[0];
            data[data_pos + 1] ^= key_bytes[1];
            data[data_pos + 2] ^= key_bytes[2];
            data[data_pos + 3] ^= key_bytes[3];
        }

        block_idx = chunk_end;
    }

    pos += aligned_size;

    // Handle remaining bytes
    if pos < size {
        let key = generate_key(base_counter.wrapping_add(num_blocks as u32));
        let key_bytes = key.to_le_bytes();
        let bytes_remaining = size - pos;
        for i in 0..bytes_remaining {
            data[pos + i] ^= key_bytes[i];
        }
    }
}

/// Parallel decipher for large data blocks
/// Uses rayon for multi-threaded decryption when data is large enough
#[cfg(feature = "parallel_decipher")]
pub fn decipher_inplace_parallel(data: &mut [u8], seed: u32, size: usize, offset_to_file_start: usize) {
    use rayon::prelude::*;

    const PARALLEL_THRESHOLD: usize = 65536; // 64KB threshold

    if size < PARALLEL_THRESHOLD {
        decipher_inplace(data, seed, size, offset_to_file_start);
        return;
    }

    // Process in parallel chunks
    let chunk_size = 4096; // 4KB chunks
    let num_chunks = (size + chunk_size - 1) / chunk_size;

    (0..num_chunks).into_par_iter().for_each(|chunk_idx| {
        let start = chunk_idx * chunk_size;
        let end = std::cmp::min(start + chunk_size, size);
        let chunk_offset = offset_to_file_start + start;

        decipher_inplace(&mut data[start..end], seed, end - start, chunk_offset);
    });
}
