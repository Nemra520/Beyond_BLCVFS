use crate::VFS::BLC::{BlcError, Result};
use crate::VFS::FileType::PckExtractor;
use super::Package;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use rayon::prelude::*;

pub struct MultiVFS {
    packages: HashMap<String, Package>,
    file_to_package: HashMap<String, String>,
}

impl MultiVFS {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
            file_to_package: HashMap::new(),
        }
    }

    pub fn mount_folder<P: AsRef<Path>>(folder: P) -> Result<Self> {
        let folder = folder.as_ref();

        // Collect all package directories first
        let package_dirs: Vec<PathBuf> = std::fs::read_dir(folder)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.path())
            .collect();

        let total_packages = package_dirs.len();
        println!("  Found {} package directories to mount", total_packages);

        // Use rayon for parallel processing
        let mounted_packages: Vec<(String, Package, Vec<String>)> = package_dirs
            .par_iter()
            .filter_map(|path| {
                match Package::mount(path) {
                    Ok(package) => {
                        let package_name = package.name.clone();
                        let files: Vec<String> = package.list_files()
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect();
                        Some((package_name, package, files))
                    }
                    Err(_) => None
                }
            })
            .collect();

        let mut multi_vfs = Self::new();
        let mut total_files = 0usize;

        for (idx, (package_name, package, files)) in mounted_packages.into_iter().enumerate() {
            let file_count = files.len();
            total_files += file_count;

            for file in files {
                multi_vfs.file_to_package.insert(file, package_name.clone());
            }

            multi_vfs.packages.insert(package_name.clone(), package);

            println!("  Mounted [{}/{}]: {} ({} files, total: {})",
                idx + 1, total_packages, package_name, file_count, total_files);
        }

        println!("  ✓ Mounted {} packages with {} total files",
            multi_vfs.packages.len(), total_files);

        Ok(multi_vfs)
    }
    
    pub fn list_packages(&self) -> Vec<&str> {
        self.packages.keys().map(|s| s.as_str()).collect()
    }
    
    pub fn list_all_files(&self) -> Vec<&str> {
        self.file_to_package.keys().map(|s| s.as_str()).collect()
    }
    
    pub fn get_package(&self, name: &str) -> Option<&Package> {
        self.packages.get(name)
    }
    
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let package_name = self.file_to_package
            .get(path)
            .ok_or_else(|| BlcError::FileNotFound(std::path::PathBuf::from(path)))?;
        
        let package = self.packages
            .get(package_name)
            .ok_or_else(|| BlcError::FileNotFound(std::path::PathBuf::from(path)))?;
        
        package.read_file(path)
    }
    
    pub fn get_total_file_count(&self) -> usize {
        self.file_to_package.len()
    }
    
    pub fn get_package_count(&self) -> usize {
        self.packages.len()
    }
    
    #[allow(dead_code)]
    pub fn get_file_package(&self, file_path: &str) -> Option<&str> {
        self.file_to_package.get(file_path).map(|s| s.as_str())
    }
    
    pub fn file_exists(&self, path: &str) -> bool {
        self.file_to_package.contains_key(path)
    }

    pub fn list_pck_files(&self) -> Vec<&str> {
        self.file_to_package
            .keys()
            .filter(|k| k.to_lowercase().ends_with(".pck"))
            .map(|s| s.as_str())
            .collect()
    }

    pub fn read_pck_file(&self, path: &str) -> Result<Vec<u8>> {
        let data = self.read_file(path)?;
        PckExtractor::get_decrypted_pck(&data)
    }

    pub fn extract_pck_to_dir(&self, path: &str, output_dir: &Path) -> Result<Vec<PathBuf>> {
        let data = self.read_file(path)?;
        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("output.pck");
        PckExtractor::extract_to_dir(&data, output_dir, Some(file_name))
    }

    pub fn extract_all_pck(&self, output_dir: &Path) -> Result<(usize, usize)> {
        let pck_files = self.list_pck_files();
        let _total = pck_files.len();
        let mut success = 0;
        let mut failed = 0;

        for pck_path in &pck_files {
            match self.extract_pck_to_dir(pck_path, output_dir) {
                Ok(_) => success += 1,
                Err(_) => failed += 1,
            }
        }

        Ok((success, failed))
    }
}
