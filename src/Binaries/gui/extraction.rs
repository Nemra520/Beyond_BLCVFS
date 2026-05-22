use std::collections::HashSet;
use std::path::PathBuf;

const TABLE_CFG_PACKAGE: &str = "42A8FCA6";

pub struct ExtractionManager;

impl ExtractionManager {
    pub fn extract_single_file(
        vfs: &blc_vfs::MultiVFS,
        file_path: &str,
        output_dir: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("[DEBUG] extract_file: reading '{}'", file_path);

        let data = if file_path.to_lowercase().ends_with(".pck") {
            vfs.read_pck_file(file_path)
                .map_err(|e| {
                    println!("[DEBUG] extract_file: read_pck_file failed for '{}': {}", file_path, e);
                    format!("Failed to read PCK '{}': {}", file_path, e)
                })?
        } else {
            vfs.read_file(file_path)
                .map_err(|e| {
                    println!("[DEBUG] extract_file: read failed for '{}': {}", file_path, e);
                    format!("Failed to read '{}': {}", file_path, e)
                })?
        };

        println!("[DEBUG] extract_file: read {} bytes for '{}'", data.len(), file_path);

        // Check if file is in package 42A8FCA6 and is .bytes -> convert to JSON
        let is_bytes_in_target_pkg = file_path.to_lowercase().ends_with(".bytes")
            && vfs.get_file_package(file_path)
                .map(|pkg| pkg.eq_ignore_ascii_case(TABLE_CFG_PACKAGE))
                .unwrap_or(false);

        let (output_path, write_data) = if is_bytes_in_target_pkg {
            let json_str = blc_vfs::SparkBytesParser::parse_to_json(&data);
            let json_path = file_path.strip_suffix(".bytes")
                .map(|s| format!("{}.json", s))
                .unwrap_or_else(|| format!("{}.json", file_path));
            println!("[DEBUG] extract_file: converting .bytes to JSON for '{}'", file_path);
            (output_dir.join(json_path), json_str.into_bytes())
        } else {
            (output_dir.join(file_path), data)
        };

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
        std::fs::write(&output_path, write_data)
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

        // Collect all files under the current directory (including subdirectories)
        let files: Vec<String> = all_files
            .into_iter()
            .filter(|f| {
                if current_dir.is_empty() {
                    // In root: include all files
                    true
                } else {
                    // In subdirectory: include all files that start with the prefix
                    f.starts_with(&dir_prefix)
                }
            })
            .map(|s| s.to_string())
            .collect();

        println!("[DEBUG] Found {} files in current directory (including subdirectories)", files.len());
        files
    }
    
    pub fn get_selected_files(
        vfs: &blc_vfs::MultiVFS,
        selected_files: &HashSet<String>,
    ) -> Vec<String> {
        let all_files = vfs.list_all_files();
        let mut files_to_extract: Vec<String> = Vec::new();

        for selected in selected_files {
            let dir_prefix = format!("{}/", selected.trim_matches('/'));
            let mut found_files_in_dir = false;

            // Check if this is a directory by looking for files under it
            // Include all files in the directory and its subdirectories
            for file in &all_files {
                if file.starts_with(&dir_prefix) {
                    found_files_in_dir = true;
                    // Include all files under this directory (including subdirectories)
                    files_to_extract.push(file.to_string());
                }
            }

            // If no files found under this path, check if it's a file itself
            if !found_files_in_dir {
                // Check if the selected item exists as a file in the VFS
                if all_files.contains(&selected.as_str()) {
                    files_to_extract.push(selected.clone());
                }
            }
        }

        files_to_extract
    }
}
