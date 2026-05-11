use crate::VFS::BLC::{BlcError, Result};
use crate::VFS::FileType::PckExtractor;
use super::Package;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
        let mut multi_vfs = Self::new();
        
        for entry in std::fs::read_dir(folder)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Ok(package) = Package::mount(&path) {
                    let package_name = package.name.clone();
                    println!("  Found package: {} ({} files)", package_name, package.get_file_count());
                    
                    for file_name in package.list_files() {
                        multi_vfs.file_to_package.insert(file_name.to_string(), package_name.clone());
                    }
                    
                    multi_vfs.packages.insert(package_name, package);
                }
            }
        }
        
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
