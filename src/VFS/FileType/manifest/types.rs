use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetInfo {
    pub asset_size: i64,
    pub bundle_index: i64,
    pub path: String,
    pub path_hash_head: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BundleInfo {
    pub bundle_flags: i32,
    pub bundle_index: i64,
    pub category: i32,
    #[serde(default)]
    pub dependencies: Vec<i64>,
    #[serde(default)]
    pub direct_dependencies: Vec<i64>,
    #[serde(default)]
    pub direct_reverse_dependencies: Vec<i64>,
    pub hash_name: i64,
    pub hash_version: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManifestJson {
    #[serde(default)]
    pub assets: Vec<AssetInfo>,
    #[serde(default)]
    pub bundles: Vec<BundleInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(rename = "perforceCL", skip_serializing_if = "Option::is_none")]
    pub perforce_cl: Option<String>,
    #[serde(rename = "m_assetInfoAddress", skip_serializing_if = "Option::is_none")]
    pub m_asset_info_address: Option<i64>,
    #[serde(rename = "m_bundleAddress", skip_serializing_if = "Option::is_none")]
    pub m_bundle_address: Option<i64>,
    #[serde(rename = "m_bundleArrayAddress", skip_serializing_if = "Option::is_none")]
    pub m_bundle_array_address: Option<i64>,
    #[serde(rename = "m_dataAddress", skip_serializing_if = "Option::is_none")]
    pub m_data_address: Option<i64>,
}

/// 虚拟文件系统中的文件/目录项
#[derive(Debug, Clone)]
pub struct VirtualEntry {
    pub name: String,
    pub full_path: String,
    pub is_dir: bool,
    pub size: i64,
    pub bundle_index: Option<i64>,
    pub asset_info: Option<AssetInfo>,
}

/// Manifest 虚拟文件系统
#[derive(Debug, Clone)]
pub struct ManifestVFS {
    pub manifest: ManifestJson,
    /// bundle_index -> bundle_name
    pub bundle_index_map: HashMap<i64, String>,
    /// bundle_name -> 该bundle包含的所有assets
    pub bundle_assets: HashMap<String, Vec<AssetInfo>>,
    /// 虚拟路径 -> 实际AB文件路径
    pub virtual_to_ab: HashMap<String, String>,
}

impl ManifestVFS {
    pub fn new(manifest: ManifestJson) -> Self {
        let start_time = std::time::Instant::now();
        
        let mut bundle_index_map = HashMap::new();
        let mut bundle_assets: HashMap<String, Vec<AssetInfo>> = HashMap::new();
        let mut virtual_to_ab = HashMap::new();

        // 建立 bundle_index -> bundle_name 映射
        for bundle in &manifest.bundles {
            bundle_index_map.insert(bundle.bundle_index, bundle.name.clone());
            bundle_assets.insert(bundle.name.clone(), Vec::new());
        }

        // 将 assets 按 bundle 分组
        for asset in &manifest.assets {
            if let Some(bundle_name) = bundle_index_map.get(&asset.bundle_index) {
                if let Some(assets) = bundle_assets.get_mut(bundle_name) {
                    assets.push(asset.clone());
                }
                // 建立虚拟路径到AB文件的映射
                let virtual_path = format!("{}.ab", asset.path);
                virtual_to_ab.insert(virtual_path, bundle_name.clone());
            }
        }

        let elapsed = start_time.elapsed();
        println!("[DEBUG] ManifestVFS::new: {} bundles, {} assets, built in {:?}", 
            manifest.bundles.len(), manifest.assets.len(), elapsed);

        Self {
            manifest,
            bundle_index_map,
            bundle_assets,
            virtual_to_ab,
        }
    }

    /// 列出指定虚拟目录下的条目
    pub fn list_directory(&self, current_dir: &str) -> Vec<VirtualEntry> {
        use std::collections::HashSet;

        let mut entries: HashSet<String> = HashSet::new();
        let mut dir_entries: HashSet<String> = HashSet::new();

        let current_prefix = if current_dir.is_empty() {
            String::new()
        } else {
            format!("{}/", current_dir.trim_matches('/'))
        };

        // 遍历所有 assets 的虚拟路径
        for asset in &self.manifest.assets {
            let virtual_path = format!("{}.ab", asset.path);

            if current_dir.is_empty() || virtual_path.starts_with(&current_prefix) {
                let remaining = if current_dir.is_empty() {
                    virtual_path.as_str()
                } else {
                    &virtual_path[current_prefix.len()..]
                };

                if let Some(slash_pos) = remaining.find('/') {
                    let dir_name = remaining[..slash_pos].to_string();
                    entries.insert(dir_name.clone());
                    dir_entries.insert(dir_name);
                } else if !remaining.is_empty() {
                    entries.insert(remaining.to_string());
                }
            }
        }

        entries.into_iter().map(|name| {
            let full_path = if current_dir.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", current_dir, name)
            };

            let is_dir = dir_entries.contains(&name);

            // 如果是文件，查找对应的 asset 信息
            let (size, bundle_index, asset_info) = if !is_dir {
                let virtual_path = if current_dir.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", current_dir, name)
                };

                if let Some(ab_path) = self.virtual_to_ab.get(&virtual_path) {
                    // 查找该 asset 的信息
                    if let Some(assets) = self.bundle_assets.get(ab_path) {
                        // 从 virtual_path 移除 .ab 后缀来匹配
                        let asset_path = virtual_path.strip_suffix(".ab").unwrap_or(&virtual_path);
                        if let Some(asset) = assets.iter().find(|a| a.path == asset_path) {
                            (asset.asset_size, Some(asset.bundle_index), Some(asset.clone()))
                        } else {
                            (0, None, None)
                        }
                    } else {
                        (0, None, None)
                    }
                } else {
                    (0, None, None)
                }
            } else {
                (0, None, None)
            };

            VirtualEntry {
                name,
                full_path,
                is_dir,
                size,
                bundle_index,
                asset_info,
            }
        }).collect()
    }

    /// 获取虚拟路径对应的 AB 文件名
    pub fn get_ab_for_virtual_path(&self, virtual_path: &str) -> Option<&String> {
        self.virtual_to_ab.get(virtual_path)
    }

    /// 获取所有虚拟文件路径
    pub fn get_all_virtual_paths(&self) -> Vec<String> {
        self.virtual_to_ab.keys().cloned().collect()
    }

    /// 获取指定目录及其子目录下的所有虚拟文件路径
    pub fn get_all_files_in_dir(&self, dir: &str) -> Vec<String> {
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{}/", dir.trim_matches('/'))
        };

        self.virtual_to_ab
            .keys()
            .filter(|path| path.starts_with(&prefix))
            .cloned()
            .collect()
    }

    /// 获取 AB 文件包含的所有虚拟路径
    pub fn get_virtual_paths_for_ab(&self, ab_name: &str) -> Vec<String> {
        self.bundle_assets
            .get(ab_name)
            .map(|assets| {
                assets.iter()
                    .map(|a| format!("{}.ab", a.path))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取所有根级目录
    pub fn get_root_dirs(&self) -> Vec<String> {
        use std::collections::HashSet;
        let mut roots: HashSet<String> = HashSet::new();

        for asset in &self.manifest.assets {
            if let Some(first_slash) = asset.path.find('/') {
                roots.insert(asset.path[..first_slash].to_string());
            }
        }

        roots.into_iter().collect()
    }
}
