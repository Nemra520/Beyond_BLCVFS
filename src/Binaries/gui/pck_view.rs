use eframe::egui;

use super::types::PckView;

pub struct PckViewUI;

impl PckViewUI {
    pub fn render(
        ui: &mut egui::Ui,
        pck_view: &mut PckView,
        status_message: &mut String,
        vfs: &Option<blc_vfs::MultiVFS>,
    ) {
        let pck_path = pck_view.pck_path.clone();
        let selected_count = pck_view.selected_entries.len();
        let has_selection = selected_count > 0;
        
        ui.horizontal(|ui| {
            ui.heading(format!("📦 PCK: {}", pck_path));
        });
        ui.separator();
        
        // Export buttons
        ui.horizontal(|ui| {
            if ui.button("☑ Select All").clicked() {
                pck_view.selected_entries = pck_view.entries.iter().map(|e| e.file_id).collect();
            }
            
            if has_selection {
                if ui.button(format!("📥 Export Selected ({})", selected_count)).clicked() {
                    Self::export_selected_entries(pck_view, vfs, status_message);
                }
                if ui.button("❌ Clear Selection").clicked() {
                    pck_view.selected_entries.clear();
                }
            }
            if ui.button("📥 Export All").clicked() {
                Self::export_all_entries(pck_view, vfs, status_message);
            }
        });
        ui.separator();
        
        // Header
        ui.horizontal(|ui| {
            ui.label("☑");
            ui.separator();
            ui.label("File ID");
            ui.separator();
            ui.label("Type");
            ui.separator();
            ui.label("Size");
        });
        ui.separator();

        // Use ScrollArea with virtualized rows for large PCK files
        let row_height = 24.0;
        let total_rows = pck_view.entries.len();

        let mut toggle_entry = None;
        let mut export_single = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                for row_index in row_range {
                    if row_index >= pck_view.entries.len() {
                        break;
                    }
                    let entry = &pck_view.entries[row_index];

                    ui.horizontal(|ui| {
                        ui.set_min_height(row_height);

                        let mut is_selected = pck_view.selected_entries.contains(&entry.file_id);
                        if ui.checkbox(&mut is_selected, "").clicked() {
                            toggle_entry = Some(entry.file_id);
                        }
                        ui.separator();

                        let icon = match entry.entry_type.as_str() {
                            "WEM" => "🔊",
                            "BNK" => "🎵",
                            _ => "📄",
                        };
                        ui.label(format!("{} {}", icon, entry.file_id));
                        ui.separator();
                        ui.label(&entry.entry_type);
                        ui.separator();
                        ui.label(format!("{} bytes", entry.size));

                        if ui.small_button("Export").clicked() {
                            export_single = Some(entry.file_id);
                        }
                    });
                }
            });

        if let Some(file_id) = toggle_entry {
            if pck_view.selected_entries.contains(&file_id) {
                pck_view.selected_entries.remove(&file_id);
            } else {
                pck_view.selected_entries.insert(file_id);
            }
        }

        if let Some(file_id) = export_single {
            Self::export_single_entry(pck_view, vfs, file_id, status_message);
        }
    }
    
    fn export_single_entry(
        pck_view: &PckView,
        vfs: &Option<blc_vfs::MultiVFS>,
        file_id: u64,
        status_message: &mut String,
    ) {
        if let Some(vfs) = vfs {
            let pck_path = &pck_view.pck_path;
            
            match vfs.read_file(pck_path) {
                Ok(data) => {
                    match blc_vfs::PckExtractor::extract_pck(&data) {
                        Ok(result) => {
                            if let Some(entry) = result.entries.iter().find(|e| e.file_id == file_id) {
                                // Use VFS virtual path as output directory structure
                                let parent_dir = &pck_view.parent_dir;
                                let pck_file_name = std::path::Path::new(pck_path)
                                    .file_stem()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("pck");
                                
                                let output_dir = if parent_dir.is_empty() {
                                    std::path::PathBuf::from("output").join(pck_file_name)
                                } else {
                                    std::path::PathBuf::from("output")
                                        .join(parent_dir.replace('/', "\\"))
                                        .join(pck_file_name)
                                };
                                
                                let file_name = format!("{}{}", entry.file_id, entry.entry_type.extension());
                                let output_path = output_dir.join(&file_name);
                                
                                if let Err(e) = std::fs::create_dir_all(&output_dir) {
                                    *status_message = format!("✗ Failed to create directory: {}", e);
                                    return;
                                }
                                
                                match std::fs::write(&output_path, &entry.data) {
                                    Ok(()) => {
                                        *status_message = format!("✓ Exported: {} ({} bytes)", file_name, entry.data.len());
                                    }
                                    Err(e) => {
                                        *status_message = format!("✗ Failed to write {}: {}", file_name, e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            *status_message = format!("✗ Failed to extract PCK: {}", e);
                        }
                    }
                }
                Err(e) => {
                    *status_message = format!("✗ Failed to read PCK: {}", e);
                }
            }
        }
    }
    
    fn export_selected_entries(
        pck_view: &PckView,
        vfs: &Option<blc_vfs::MultiVFS>,
        status_message: &mut String,
    ) {
        let selected: Vec<u64> = pck_view.selected_entries.iter().copied().collect();
        Self::export_entries(pck_view, vfs, &selected, status_message);
    }
    
    fn export_all_entries(
        pck_view: &PckView,
        vfs: &Option<blc_vfs::MultiVFS>,
        status_message: &mut String,
    ) {
        let all: Vec<u64> = pck_view.entries.iter().map(|e| e.file_id).collect();
        Self::export_entries(pck_view, vfs, &all, status_message);
    }
    
    fn export_entries(
        pck_view: &PckView,
        vfs: &Option<blc_vfs::MultiVFS>,
        file_ids: &[u64],
        status_message: &mut String,
    ) {
        if let Some(vfs) = vfs {
            let pck_path = &pck_view.pck_path;
            
            match vfs.read_file(pck_path) {
                Ok(data) => {
                    match blc_vfs::PckExtractor::extract_pck(&data) {
                        Ok(result) => {
                            // Use VFS virtual path as output directory structure
                            let parent_dir = &pck_view.parent_dir;
                            let pck_file_name = std::path::Path::new(pck_path)
                                .file_stem()
                                .and_then(|n| n.to_str())
                                .unwrap_or("pck");
                            
                            let output_dir = if parent_dir.is_empty() {
                                std::path::PathBuf::from("output").join(pck_file_name)
                            } else {
                                std::path::PathBuf::from("output")
                                    .join(parent_dir.replace('/', "\\"))
                                    .join(pck_file_name)
                            };
                            
                            // Create output directory
                            if let Err(e) = std::fs::create_dir_all(&output_dir) {
                                *status_message = format!("✗ Failed to create output directory: {}", e);
                                return;
                            }
                            
                            let mut success = 0;
                            let mut failed = 0;
                            
                            for file_id in file_ids {
                                if let Some(entry) = result.entries.iter().find(|e| e.file_id == *file_id) {
                                    let ext = entry.entry_type.extension();
                                    let file_name = format!("{}{}", entry.file_id, ext);
                                    let output_path = output_dir.join(&file_name);
                                    
                                    match std::fs::write(&output_path, &entry.data) {
                                        Ok(()) => {
                                            println!("[DEBUG] Exported: {:?} ({} bytes)", output_path, entry.data.len());
                                            success += 1;
                                        }
                                        Err(e) => {
                                            println!("[DEBUG] Failed to write {:?}: {}", output_path, e);
                                            failed += 1;
                                        }
                                    }
                                } else {
                                    println!("[DEBUG] Entry not found for file_id: {}", file_id);
                                }
                            }
                            
                            let output_display = output_dir.to_string_lossy();
                            if failed > 0 {
                                *status_message = format!("✓ {} files, ✗ {} failed in {}/", success, failed, output_display);
                            } else {
                                *status_message = format!("✓ Exported {} files to {}/", success, output_display);
                            }
                        }
                        Err(e) => {
                            *status_message = format!("✗ Failed to extract PCK: {}", e);
                        }
                    }
                }
                Err(e) => {
                    *status_message = format!("✗ Failed to read PCK: {}", e);
                }
            }
        }
    }
}
