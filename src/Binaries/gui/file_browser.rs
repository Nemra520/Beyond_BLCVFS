use eframe::egui;
use std::collections::HashSet;

use super::types::FileEntry;

pub struct FileBrowserUI;

pub struct FileBrowserResponse {
    pub new_dir: Option<String>,
    pub extract_file: Option<String>,
    pub toggle_selection: Option<String>,
    pub open_pck: Option<String>,
}

impl FileBrowserUI {
    pub fn render(
        ui: &mut egui::Ui,
        entries: &[FileEntry],
        selected_file: &Option<String>,
        selected_files: &mut HashSet<String>,
    ) -> FileBrowserResponse {
        let mut response = FileBrowserResponse {
            new_dir: None,
            extract_file: None,
            toggle_selection: None,
            open_pck: None,
        };
        
        if entries.is_empty() {
            ui.label("(empty directory)");
            return response;
        }
        
        ui.horizontal(|ui| {
            ui.selectable_value(&mut false, false, "Name");
            ui.selectable_value(&mut false, false, "Type");
        });
        ui.separator();
        
        for entry in entries {
            ui.horizontal(|ui| {
                let mut is_selected = selected_files.contains(&entry.full_path);
                
                if ui.checkbox(&mut is_selected, "").clicked() {
                    response.toggle_selection = Some(entry.full_path.clone());
                }
                
                let icon = if entry.is_dir { "📁" } else { "📄" };
                let text = format!("{} {}", icon, entry.name);
                
                let is_pck = entry.name.ends_with(".pck");
                let is_selected_file = selected_file.as_ref() == Some(&entry.full_path);
                
                let selectable_response = ui.selectable_label(is_selected_file, &text);
                if selectable_response.double_clicked() {
                    if entry.is_dir {
                        response.new_dir = Some(entry.full_path.clone());
                    } else if is_pck {
                        response.open_pck = Some(entry.full_path.clone());
                    }
                } else if selectable_response.clicked() {
                    if entry.is_dir {
                        response.new_dir = Some(entry.full_path.clone());
                    }
                }
                
                if !entry.is_dir {
                    if is_pck {
                        if ui.small_button("Open").clicked() {
                            response.open_pck = Some(entry.full_path.clone());
                        }
                    }
                    if ui.small_button("Extract").clicked() {
                        response.extract_file = Some(entry.full_path.clone());
                    }
                }
            });
        }
        
        response
    }
}

pub fn list_directory(vfs: &blc_vfs::MultiVFS, current_dir: &str) -> Vec<FileEntry> {
    let all_files = vfs.list_all_files();
    let current_prefix = if current_dir.is_empty() {
        String::new()
    } else {
        format!("{}/", current_dir.trim_matches('/'))
    };
    
    let mut entries: HashSet<String> = HashSet::new();
    
    for file in &all_files {
        let file_str = file.to_string();
        
        if current_dir.is_empty() || file_str.starts_with(&current_prefix) {
            let remaining = if current_dir.is_empty() {
                file_str.as_str()
            } else {
                &file_str[current_prefix.len()..]
            };
            
            if let Some(slash_pos) = remaining.find('/') {
                entries.insert(remaining[..slash_pos].to_string());
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
        
        let is_dir = all_files.iter().any(|f| f.starts_with(&format!("{}/", full_path)));
        
        FileEntry { name, is_dir, full_path }
    }).collect()
}
