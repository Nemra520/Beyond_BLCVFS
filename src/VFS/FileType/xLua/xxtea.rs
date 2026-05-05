pub struct Xxtea;

impl Xxtea {
    const DELTA: u32 = 0x9E3779B9;
    
    pub fn decrypt(data: &[u8], key: &[u8]) -> Option<Vec<u8>> {
        if data.is_empty() {
            return Some(data.to_vec());
        }
        
        let v = Self::to_u32_array(data, false);
        let k = Self::to_u32_array_key(&Self::fix_key(key));
        
        let decrypted = Self::decrypt_internal(&v, &k);
        
        Self::to_byte_array(&decrypted, true)
    }
    
    fn mx(sum: u32, y: u32, z: u32, p: usize, e: u32, k: &[u32]) -> u32 {
        // 注意：C# 中 + 的优先级高于 ^，所以需要用括号保持相同运算顺序
        let part1 = (z >> 5 ^ y << 2).wrapping_add(y >> 3 ^ z << 4);
        let part2 = (sum ^ y).wrapping_add(k[(p & 3) ^ (e as usize)] ^ z);
        part1 ^ part2
    }
    
    fn decrypt_internal(v: &[u32], k: &[u32]) -> Vec<u32> {
        let n = v.len() - 1;
        if n < 1 {
            return v.to_vec();
        }
        
        let mut v = v.to_vec();
        let q = 6 + 52 / (n + 1);
        let mut sum = (q as u32).wrapping_mul(Self::DELTA);
        
        let mut y = v[0];
        
        while sum != 0 {
            let e = (sum >> 2) & 3;
            
            for p in (1..=n).rev() {
                let z = v[p - 1];
                y = v[p].wrapping_sub(Self::mx(sum, y, z, p, e, k));
                v[p] = y;
            }
            
            let z = v[n];
            y = v[0].wrapping_sub(Self::mx(sum, y, z, 0, e, k));
            v[0] = y;
            
            sum = sum.wrapping_sub(Self::DELTA);
        }
        
        v
    }
    
    fn fix_key(key: &[u8]) -> Vec<u8> {
        if key.len() == 16 {
            return key.to_vec();
        }
        
        let mut fixed_key = vec![0u8; 16];
        let copy_len = key.len().min(16);
        fixed_key[..copy_len].copy_from_slice(&key[..copy_len]);
        fixed_key
    }
    
    fn to_u32_array(data: &[u8], include_length: bool) -> Vec<u32> {
        let length = data.len();
        let n = if (length & 3) == 0 {
            length >> 2
        } else {
            (length >> 2) + 1
        };
        
        let mut result = if include_length {
            let mut v = vec![0u32; n + 1];
            v[n] = length as u32;
            v
        } else {
            vec![0u32; n]
        };
        
        for (i, &byte) in data.iter().enumerate() {
            result[i >> 2] |= (byte as u32) << ((i & 3) << 3);
        }
        
        result
    }
    
    fn to_u32_array_key(key: &[u8]) -> Vec<u32> {
        let mut result = vec![0u32; 4];
        
        for (i, &byte) in key.iter().enumerate() {
            result[i >> 2] |= (byte as u32) << ((i & 3) << 3);
        }
        
        result
    }
    
    fn to_byte_array(data: &[u32], include_length: bool) -> Option<Vec<u8>> {
        let mut n = data.len() << 2;
        
        if include_length {
            let m = data[data.len() - 1] as usize;
            n -= 4;
            if m < n - 3 || m > n {
                return None;
            }
            n = m;
        }
        
        let mut result = vec![0u8; n];
        for i in 0..n {
            result[i] = (data[i >> 2] >> ((i & 3) << 3)) as u8;
        }
        
        Some(result)
    }
}
