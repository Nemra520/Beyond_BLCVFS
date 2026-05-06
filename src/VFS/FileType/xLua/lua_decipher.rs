use super::xxtea::Xxtea;

const KEYS: [&str; 8] = [
    "cynb5",
    "paeky",
    "xmF5og",
    "ud35+e",
    "72iUy",
    "azWk3",
    "901lU",
    "dDfl2",
];

const INITIAL_KEY: &str = "Assets/Beyond/InitialAssets/";
const DEFAULT_DECRYPTION_KEY: &str = "Assets/Beyond/DynamicAssets/Gameplay/UI/Fonts/";

pub struct LuaDecipher;

impl LuaDecipher {
    pub fn decrypt(encrypted_data: &[u8]) -> Option<Vec<u8>> {
        let master_key = Self::get_master_key()?;
        
        let encrypted_bytes = if Self::is_base64(encrypted_data) {
            use base64::{Engine as _, engine::general_purpose};
            let cleaned: Vec<u8> = encrypted_data
                .iter()
                .filter(|&&b| b != b'\n' && b != b'\r')
                .copied()
                .collect();
            general_purpose::STANDARD.decode(&cleaned).ok()?
        } else {
            encrypted_data.to_vec()
        };
        
        let decrypted = Xxtea::decrypt(&encrypted_bytes, master_key.as_bytes())?;
        
        if !Self::is_valid_lua_bytecode(&decrypted) && Self::is_likely_encrypted(&decrypted) {
            eprintln!("Warning: Decrypted data does not match Lua format and has high entropy (may still be encrypted)");
        }
        
        Some(decrypted)
    }
    
    pub fn decrypt_with_key(encrypted_data: &[u8], key: &str) -> Option<Vec<u8>> {
        if key.is_empty() {
            return None;
        }
        
        let encrypted_bytes = if Self::is_base64(encrypted_data) {
            use base64::{Engine as _, engine::general_purpose};
            let cleaned: Vec<u8> = encrypted_data
                .iter()
                .filter(|&&b| b != b'\n' && b != b'\r')
                .copied()
                .collect();
            general_purpose::STANDARD.decode(&cleaned).ok()?
        } else {
            encrypted_data.to_vec()
        };
        
        let decrypted = Xxtea::decrypt(&encrypted_bytes, key.as_bytes())?;
        
        if !Self::is_valid_lua_bytecode(&decrypted) && Self::is_likely_encrypted(&decrypted) {
            eprintln!("Warning: Decrypted data does not match Lua format and has high entropy (may still be encrypted)");
        }
        
        Some(decrypted)
    }
    
    fn is_base64(data: &[u8]) -> bool {
        if data.is_empty() {
            return false;
        }
        
        for &byte in data {
            if !byte.is_ascii_alphanumeric() 
                && byte != b'+' 
                && byte != b'/' 
                && byte != b'='
                && byte != b'\n'
                && byte != b'\r' {
                return false;
            }
        }
        
        true
    }
    
    pub fn get_master_key() -> Option<String> {
        if KEYS.len() <= 5 {
            return None;
        }
        
        let encrypted_master_key = format!("{}{}{}{}==", KEYS[1], KEYS[5], KEYS[3], KEYS[2]);
        
        let master_key_bytes = Self::decrypt_subtraction(&encrypted_master_key, Some(INITIAL_KEY))?;
        
        if master_key_bytes.is_empty() {
            return None;
        }
        
        String::from_utf8(master_key_bytes).ok()
    }
    
    fn decrypt_subtraction(encrypted_text: &str, key: Option<&str>) -> Option<Vec<u8>> {
        let key = key.unwrap_or(DEFAULT_DECRYPTION_KEY);
        
        use base64::{Engine as _, engine::general_purpose};
        let mut data = general_purpose::STANDARD.decode(encrypted_text).ok()?;
        
        let key_bytes = key.as_bytes();
        let key_len = key_bytes.len();
        
        if key_len == 0 {
            return Some(data);
        }
        
        for i in 0..data.len() {
            data[i] = data[i].wrapping_sub(key_bytes[i % key_len]);
        }
        
        Some(data)
    }
    
    pub fn is_valid_lua_bytecode(data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }
        
        if data[0] == 0x1B && data[1] == 0x4C && data[2] == 0x75 && data[3] == 0x61 {
            return true;
        }
        
        if let Ok(text) = std::str::from_utf8(&data[..data.len().min(2000)]) {
            let text = text.trim_start_matches(|c: char| c == '\u{FEFF}' || c == '\r' || c == '\n' || c == ' ' || c == '\t');
            
            return text.starts_with("local ")
                || text.starts_with("function ")
                || text.starts_with("return ")
                || text.starts_with("require ")
                || text.starts_with("--")
                || text.starts_with("config ")
                || text.contains("= HL.Class(")
                || text.contains("= Class(")
                || text.contains("= {")
                || text.contains("= {")
                || text.contains("local ")
                || text.contains("function(")
                || text.contains("function ")
                || text.contains("MessageConst.")
                || text.contains("HL.")
                || text.contains("UIConst.")
                || text.contains("GameInstance.");
        }
        
        false
    }
    
    pub fn calculate_entropy(data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }
        
        let mut frequency = [0u64; 256];
        for &byte in data {
            frequency[byte as usize] += 1;
        }
        
        let len = data.len() as f64;
        let mut entropy = 0.0;
        
        for &count in &frequency {
            if count > 0 {
                let probability = count as f64 / len;
                entropy -= probability * probability.log2();
            }
        }
        
        entropy
    }
    
    pub fn is_likely_encrypted(data: &[u8]) -> bool {
        if data.len() < 100 {
            return false;
        }
        
        let entropy = Self::calculate_entropy(data);
        
        entropy > 7.5
    }
}
