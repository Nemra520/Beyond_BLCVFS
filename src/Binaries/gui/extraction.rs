use std::collections::HashSet;
use std::path::PathBuf;

const TABLE_CFG_PACKAGE: &str = "42A8FCA6";
const STRINGPATH_PACKAGES: &[&str] = &["3C9D9D2D", "D6E622F7"];

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

        // Check if file is .hgmmap -> convert to JSON
        let is_hgmmap = file_path.to_lowercase().ends_with(".hgmmap");

        // Check if file is .bin in specific packages -> convert to JSON via PathBytesParser
        let is_bin_in_target_pkg = file_path.to_lowercase().ends_with(".bin")
            && vfs.get_file_package(file_path)
                .map(|pkg| STRINGPATH_PACKAGES.iter().any(|&p| pkg.eq_ignore_ascii_case(p)))
                .unwrap_or(false);

        // Check if it's a compress bin (filename contains "compress")
        let is_compress_bin = is_bin_in_target_pkg && file_path.to_lowercase().contains("compress");

        if is_bytes_in_target_pkg {
            let json_str = blc_vfs::SparkBytesParser::parse_to_json(&data);
            let json_path = file_path.strip_suffix(".bytes")
                .map(|s| format!("{}.json", s))
                .unwrap_or_else(|| format!("{}.json", file_path));
            println!("[DEBUG] extract_file: converting .bytes to JSON for '{}'", file_path);
            let output_path = output_dir.join(json_path);
            Self::write_file(&output_path, json_str.into_bytes())?;
        } else if is_hgmmap {
            let json_str = std::thread::scope(|s| {
                s.spawn(|| blc_vfs::HgmmapParser::parse_to_json(&data)).join().unwrap_or_else(|_| "{\"error\": \"hgmmap parse thread panicked\"}".to_string())
            });
            let json_path = file_path.strip_suffix(".hgmmap")
                .map(|s| format!("{}.json", s))
                .unwrap_or_else(|| format!("{}.json", file_path));
            println!("[DEBUG] extract_file: converting .hgmmap to JSON for '{}'", file_path);
            let output_path = output_dir.join(json_path);
            Self::write_file(&output_path, json_str.into_bytes())?;
        } else if is_compress_bin {
            // Compress bin: create folder and extract multiple files
            // Preserve full virtual path, just strip .bin extension for folder name
            let folder_path = file_path.strip_suffix(".bin")
                .map(|s| output_dir.join(s))
                .unwrap_or_else(|| output_dir.join(file_path));
            
            std::fs::create_dir_all(&folder_path)
                .map_err(|e| format!("Failed to create directory '{}': {}", folder_path.display(), e))?;
            
            println!("[DEBUG] extract_file: extracting compress bin '{}' to folder '{}'", file_path, folder_path.display());
            
            // Parse and extract entries
            let entries = blc_vfs::PathBytesParser::parse_compress_entries(&data);
            for entry in entries {
                let entry_path = folder_path.join(&entry.filename);
                Self::write_file(&entry_path, entry.data)?;
                println!("[DEBUG] extract_file: wrote entry '{}'", entry_path.display());
            }
        } else if is_bin_in_target_pkg {
            // Regular bin: single JSON output
            let filename = std::path::Path::new(file_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let json_str = blc_vfs::PathBytesParser::parse_to_json(&data, &filename);
            let json_path = file_path.strip_suffix(".bin")
                .map(|s| format!("{}.json", s))
                .unwrap_or_else(|| format!("{}.json", file_path));
            println!("[DEBUG] extract_file: converting .bin to JSON for '{}'", file_path);
            let output_path = output_dir.join(json_path);
            Self::write_file(&output_path, json_str.into_bytes())?;
        } else {
            // Regular file: copy as-is
            let output_path = output_dir.join(file_path);
            Self::write_file(&output_path, data)?;
        };

        println!("[DEBUG] extract_file: successfully extracted '{}'", file_path);
        Ok(())
    }

    fn write_file(output_path: &std::path::Path, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        println!("[DEBUG] extract_file: output path {:?}", output_path);
        
        if let Some(parent) = output_path.parent() {
            println!("[DEBUG] extract_file: creating parent directory {:?}", parent);
            std::fs::create_dir_all(parent)
                .map_err(|e| {
                    println!("[DEBUG] extract_file: create_dir_all failed: {}", e);
                    format!("Failed to create directory '{:?}': {}", parent, e)
                })?;
        }
        
        println!("[DEBUG] extract_file: writing to {:?}", output_path);
        std::fs::write(output_path, data)
            .map_err(|e| {
                println!("[DEBUG] extract_file: write failed: {}", e);
                format!("Failed to write '{:?}': {}", output_path, e)
            })?;
        
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
