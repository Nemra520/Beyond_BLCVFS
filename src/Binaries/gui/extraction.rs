use std::collections::HashSet;
use std::path::PathBuf;

pub struct ExtractionManager;

impl ExtractionManager {
    pub fn extract_single_file(
        vfs: &blc_vfs::MultiVFS,
        file_path: &str,
        output_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("[DEBUG] extract_file: reading '{}'", file_path);
        
        let data = vfs.read_file(file_path)
            .map_err(|e| {
                println!("[DEBUG] extract_file: read failed for '{}': {}", file_path, e);
                format!("Failed to read '{}': {}", file_path, e)
            })?;
        
        println!("[DEBUG] extract_file: read {} bytes for '{}'", data.len(), file_path);
        
        let output_path = output_dir.join(file_path);
        println!("[DEBUG] extract_file: output path {:?}", output_path);
        
        if let Some(parent) = output_path.parent() {
            println!("[DEBUG] extract_file: creating parent directory {:?}", parent);
            std::fs::create_dir_all(parent)
                .map_err(|e| {
                    println!("[DEBUG] extract_file: create_dir_all failed: {}", e);
                    format!("Failed to create directory for '{}': {}", file_path, e)
                })?;
        }
        
        println!("[DEBUG] extract_file: writing to {:?}", output_path);
        std::fs::write(&output_path, data)
            .map_err(|e| {
                println!("[DEBUG] extract_file: write failed: {}", e);
                format!("Failed to write '{}': {}", file_path, e)
            })?;
        
        println!("[DEBUG] extract_file: successfully extracted '{}'", file_path);
        Ok(())
    }
    
    pub fn get_files_in_current_dir(
        vfs: &blc_vfs::MultiVFS,
        current_dir: &str,
    ) -> Vec<String> {
        let all_files = vfs.list_all_files();
        println!("[DEBUG] Total files in VFS: {}", all_files.len());
        println!("[DEBUG] Current directory: '{}'", current_dir);
        
        let dir_prefix = if current_dir.is_empty() {
            String::new()
        } else {
            format!("{}/", current_dir.trim_matches('/'))
        };
        println!("[DEBUG] Directory prefix: '{}'", dir_prefix);
        
        // Build a set of all directory paths to filter them out
        let mut dir_paths: HashSet<String> = HashSet::new();
        for f in &all_files {
            let mut path = f.to_string();
            while let Some(pos) = path.rfind('/') {
                path = path[..pos].to_string();
                if !path.is_empty() {
                    dir_paths.insert(path.clone());
                }
            }
        }
        println!("[DEBUG] Found {} unique directory paths", dir_paths.len());
        
        all_files
            .into_iter()
            .filter(|f| {
                // Skip if this is a directory path
                if dir_paths.contains(*f) {
                    println!("[DEBUG] Skipping directory: '{}'", f);
                    return false;
                }
                
                let matches = if current_dir.is_empty() {
                    true
                } else {
                    f.starts_with(&dir_prefix)
                };
                if matches {
                    println!("[DEBUG] Matched file: '{}'", f);
                }
                matches
            })
            .map(|s| s.to_string())
            .collect()
    }
    
    pub fn get_selected_files(
        vfs: &blc_vfs::MultiVFS,
        selected_files: &HashSet<String>,
    ) -> Vec<String> {
        let all_files = vfs.list_all_files();
        
        // Build a set of all directory paths
        let mut dir_paths: HashSet<String> = HashSet::new();
        for f in &all_files {
            let mut path = f.to_string();
            while let Some(pos) = path.rfind('/') {
                path = path[..pos].to_string();
                if !path.is_empty() {
                    dir_paths.insert(path.clone());
                }
            }
        }
        
        // Collect all files to extract (including files in selected directories)
        let mut files_to_extract: Vec<String> = Vec::new();
        
        for selected in selected_files {
            if dir_paths.contains(selected) {
                // This is a directory, extract all files under it
                let dir_prefix = format!("{}/", selected.trim_matches('/'));
                for file in &all_files {
                    if file.starts_with(&dir_prefix) && !dir_paths.contains(*file) {
                        files_to_extract.push(file.to_string());
                    }
                }
            } else {
                // This is a file
                files_to_extract.push(selected.clone());
            }
        }
        
        files_to_extract
    }
}
