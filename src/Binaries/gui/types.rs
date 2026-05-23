use std::collections::HashSet;

#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub full_path: String,
}

pub struct ExtractProgress {
    pub current: usize,
    pub total: usize,
    pub success: usize,
    pub failed: usize,
}

pub struct PckView {
    pub pck_path: String,
    pub entries: Vec<PckEntryView>,
    #[allow(dead_code)]
    pub parent_dir: String,
    pub selected_entries: HashSet<u64>,
}

pub struct PckEntryView {
    pub file_id: u64,
    pub entry_type: String,
    pub size: usize,
}

/// Manifest 虚拟目录视图状态
pub struct ManifestView {
    pub manifest_path: String,
    pub vfs: blc_vfs::ManifestVFS,
    pub current_dir: String,
    pub hgmmap_base_path: String,  // hgmmap 文件夹的基础路径
    pub ab_file_prefix: String,    // AB 文件在 VFS 中的前缀路径 (如 "Data/Bundles/Windows/")
}

impl ManifestView {
    pub fn new(manifest_path: String, vfs: blc_vfs::ManifestVFS, hgmmap_base_path: String, ab_file_prefix: String) -> Self {
        Self {
            manifest_path,
            vfs,
            current_dir: String::new(),
            hgmmap_base_path,
            ab_file_prefix,
        }
    }

    /// 获取当前目录下的虚拟条目
    pub fn list_current_entries(&self) -> Vec<VirtualFileEntry> {
        self.vfs.list_directory(&self.current_dir)
            .into_iter()
            .map(|e| VirtualFileEntry {
                name: e.name,
                is_dir: e.is_dir,
                full_path: e.full_path,
                size: e.size,
                bundle_index: e.bundle_index,
            })
            .collect()
    }

    /// 进入子目录
    pub fn enter_directory(&mut self, dir: &str) {
        if self.current_dir.is_empty() {
            self.current_dir = dir.to_string();
        } else {
            self.current_dir = format!("{}/{}", self.current_dir, dir);
        }
    }

    /// 返回上级目录
    pub fn go_up(&mut self) {
        if let Some(last_slash) = self.current_dir.rfind('/') {
            self.current_dir = self.current_dir[..last_slash].to_string();
        } else {
            self.current_dir.clear();
        }
    }

    /// 获取虚拟路径对应的 AB 文件在 VFS 中的路径
    /// 返回 VFS 路径，如 "Data/Bundles/Windows/main/xxx.ab"
    pub fn get_ab_vfs_path(&self, virtual_path: &str) -> Option<String> {
        println!("[DEBUG] get_ab_vfs_path: virtual_path='{}'", virtual_path);
        println!("[DEBUG] get_ab_vfs_path: ab_file_prefix='{}'", self.ab_file_prefix);
        
        let bundle_name = self.vfs.get_ab_for_virtual_path(virtual_path)?;
        println!("[DEBUG] get_ab_vfs_path: bundle_name='{}'", bundle_name);

        // bundle_name 可能是 "main/5b7b7432eb36ad6af0c92e04.ab" 这样的格式
        // 构建 VFS 路径: prefix + bundle_name
        let vfs_path = if self.ab_file_prefix.is_empty() {
            bundle_name.clone()
        } else {
            format!("{}/{}", self.ab_file_prefix.trim_end_matches('/'), bundle_name)
        };
        
        println!("[DEBUG] get_ab_vfs_path: vfs_path='{}'", vfs_path);
        Some(vfs_path)
    }

    /// 获取虚拟路径对应的 AB 文件实际路径（文件系统路径，用于向后兼容）
    /// 需要在 hgmmap 目录及其子目录中查找
    #[allow(dead_code)]
    pub fn get_ab_path(&self, virtual_path: &str) -> Option<String> {
        println!("[DEBUG] get_ab_path: virtual_path='{}'", virtual_path);
        println!("[DEBUG] get_ab_path: hgmmap_base_path='{}'", self.hgmmap_base_path);
        
        let bundle_name = self.vfs.get_ab_for_virtual_path(virtual_path)?;
        println!("[DEBUG] get_ab_path: bundle_name='{}'", bundle_name);

        // 首先尝试直接拼接路径
        let direct_path = format!("{}/{}", self.hgmmap_base_path, bundle_name);
        println!("[DEBUG] get_ab_path: trying direct_path='{}'", direct_path);
        if std::path::Path::new(&direct_path).exists() {
            println!("[DEBUG] get_ab_path: found at direct_path");
            return Some(direct_path);
        }

        // 如果 bundle_name 包含子目录（如 "main/xxx.ab"），尝试在 hgmmap 目录下查找
        let bundle_file_name = std::path::Path::new(bundle_name)
            .file_name()
            .and_then(|n| n.to_str())?;
        println!("[DEBUG] get_ab_path: bundle_file_name='{}'", bundle_file_name);

        // 在 hgmmap 目录及其子目录中递归查找
        let result = self.find_ab_file_in_hgmmap(&self.hgmmap_base_path, bundle_file_name);
        if result.is_none() {
            println!("[DEBUG] get_ab_path: not found in hgmmap directory");
        }
        result
    }

    /// 在 hgmmap 目录中递归查找 AB 文件
    fn find_ab_file_in_hgmmap(&self, dir: &str, file_name: &str) -> Option<String> {
        let path = std::path::Path::new(dir);

        // 首先检查当前目录
        let direct_file = path.join(file_name);
        if direct_file.exists() {
            return direct_file.to_str().map(|s| s.to_string());
        }

        // 递归检查子目录
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        if let Some(found) = self.find_ab_file_in_hgmmap(
                            entry.path().to_str()?,
                            file_name
                        ) {
                            return Some(found);
                        }
                    }
                }
            }
        }

        None
    }

    /// 检查是否有上级目录
    pub fn has_parent(&self) -> bool {
        !self.current_dir.is_empty()
    }
}

/// 虚拟文件条目（用于 manifest 视图）
#[derive(Clone)]
pub struct VirtualFileEntry {
    pub name: String,
    pub is_dir: bool,
    pub full_path: String,
    pub size: i64,
    pub bundle_index: Option<i64>,
}
