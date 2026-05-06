use blc_vfs::MultiVFS;
use eframe::egui;
use std::collections::HashSet;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("BLC Virtual File System"),
        ..Default::default()
    };
    
    eframe::run_native("BLC VFS", options, Box::new(|_cc| Ok(Box::new(BlcVfsApp::default()))))
}

struct BlcVfsApp {
    vfs: Option<MultiVFS>,
    current_dir: String,
    selected_file: Option<String>,
    selected_files: HashSet<String>,
    entries: Vec<FileEntry>,
    status_message: String,
    extract_progress: Option<ExtractProgress>,
    search_query: String,
    search_results: Vec<String>,
    show_search: bool,
    pending_extraction: Option<Vec<String>>,
}

#[derive(Clone)]
struct FileEntry {
    name: String,
    is_dir: bool,
    full_path: String,
}

struct ExtractProgress {
    current: usize,
    total: usize,
    success: usize,
    failed: usize,
}

impl Default for BlcVfsApp {
    fn default() -> Self {
        Self {
            vfs: None,
            current_dir: String::new(),
            selected_file: None,
            selected_files: HashSet::new(),
            entries: Vec::new(),
            status_message: "Click 'Mount' to select a VFS folder".to_string(),
            extract_progress: None,
            search_query: String::new(),
            search_results: Vec::new(),
            show_search: false,
            pending_extraction: None,
        }
    }
}

impl eframe::App for BlcVfsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("BLC Virtual File System v0.1.0");
                ui.separator();
                
                if ui.button("📁 Mount").clicked() {
                    self.mount_folder(ctx);
                }
                
                if self.vfs.is_some() {
                    if ui.button("🏠 Root").clicked() {
                        self.current_dir.clear();
                        self.selected_files.clear();
                        self.refresh_entries();
                    }
                    
                    if !self.current_dir.is_empty() {
                        if ui.button("⬆ Up").clicked() {
                            self.go_up();
                        }
                    }
                    
                    if ui.button("� Search").clicked() {
                        self.show_search = true;
                    }
                    
                    if ui.button("📦 Packages").clicked() {
                        self.show_packages(ctx);
                    }
                }
            });
            
            ui.separator();
            
            if let Some(vfs) = &self.vfs {
                ui.horizontal(|ui| {
                    ui.label("Packages:");
                    ui.label(format!("{}", vfs.get_package_count()));
                    ui.separator();
                    ui.label("Files:");
                    ui.label(format!("{}", vfs.get_total_file_count()));
                    ui.separator();
                    ui.label("Path:");
                    ui.label(format!("/{}", self.current_dir));
                    ui.separator();
                    ui.label("Selected:");
                    ui.label(format!("{}", self.selected_files.len()));
                });
                ui.separator();
                
                ui.horizontal(|ui| {
                    if ui.button("📥 Extract Current Dir").clicked() {
                        self.extract_current_dir();
                    }
                    
                    if !self.selected_files.is_empty() {
                        if ui.button(format!("📥 Extract Selected ({})", self.selected_files.len())).clicked() {
                            self.extract_selected_files();
                        }
                        
                        if ui.button("❌ Clear Selection").clicked() {
                            self.selected_files.clear();
                        }
                    }
                });
                ui.separator();
            }
            
            if self.show_search {
                self.render_search_panel(ctx);
            }
            
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.entries.is_empty() {
                    if self.vfs.is_some() {
                        ui.label("(empty directory)");
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(100.0);
                            ui.heading("No VFS mounted");
                            ui.label("Click 'Mount' button to select a folder containing BLC packages");
                        });
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut false, false, "Name");
                        ui.selectable_value(&mut false, false, "Type");
                    });
                    ui.separator();
                    
                    let mut new_dir = None;
                    let mut extract_file = None;
                    let mut toggle_selection = None;
                    
                    for entry in &self.entries {
                        ui.horizontal(|ui| {
                            let mut is_selected = self.selected_files.contains(&entry.full_path);
                            
                            if ui.checkbox(&mut is_selected, "").clicked() {
                                toggle_selection = Some(entry.full_path.clone());
                            }
                            
                            let icon = if entry.is_dir { "📁" } else { "📄" };
                            let text = format!("{} {}", icon, entry.name);
                            
                            if ui.selectable_label(self.selected_file.as_ref() == Some(&entry.full_path), text).clicked() {
                                if entry.is_dir {
                                    new_dir = Some(entry.full_path.clone());
                                } else {
                                    self.selected_file = Some(entry.full_path.clone());
                                }
                            }
                            
                            if !entry.is_dir {
                                if ui.small_button("Extract").clicked() {
                                    extract_file = Some(entry.full_path.clone());
                                }
                            }
                        });
                    }
                    
                    if let Some(path) = toggle_selection {
                        if self.selected_files.contains(&path) {
                            self.selected_files.remove(&path);
                        } else {
                            self.selected_files.insert(path);
                        }
                    }
                    
                    if let Some(dir) = new_dir {
                        self.current_dir = dir;
                        self.refresh_entries();
                    }
                    
                    if let Some(file) = extract_file {
                        self.extract_single_file(&file);
                    }
                }
            });
            
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
            });
            
            if let Some(progress) = &self.extract_progress {
                let percentage = if progress.total > 0 {
                    (progress.current as f32 / progress.total as f32 * 100.0) as i32
                } else {
                    0
                };
                
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("Extracting... {}% ({}/{})", percentage, progress.current, progress.total));
                    ui.label(format!("✓ {}  ✗ {}", progress.success, progress.failed));
                });
            }
        });
        
        // Process pending extraction outside of UI rendering
        if let Some(files) = self.pending_extraction.take() {
            self.process_extraction(files);
            ctx.request_repaint();
        }
    }
}

impl BlcVfsApp {
    fn mount_folder(&mut self, ctx: &egui::Context) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.status_message = format!("Mounting: {}", folder.display());
            ctx.request_repaint();
            
            match MultiVFS::mount_folder(&folder) {
                Ok(vfs) => {
                    self.status_message = format!(
                        "✓ Mounted {} packages, {} files",
                        vfs.get_package_count(),
                        vfs.get_total_file_count()
                    );
                    self.vfs = Some(vfs);
                    self.current_dir.clear();
                    self.selected_files.clear();
                    self.refresh_entries();
                }
                Err(e) => {
                    self.status_message = format!("✗ Mount failed: {}", e);
                }
            }
        }
    }
    
    fn refresh_entries(&mut self) {
        if let Some(vfs) = &self.vfs {
            self.entries = list_directory(vfs, &self.current_dir);
            self.entries.sort_by(|a, b| {
                b.is_dir.cmp(&a.is_dir)
                    .then_with(|| a.name.cmp(&b.name))
            });
        }
    }
    
    fn go_up(&mut self) {
        if let Some(last_slash) = self.current_dir.rfind('/') {
            self.current_dir = self.current_dir[..last_slash].to_string();
        } else {
            self.current_dir.clear();
        }
        self.refresh_entries();
    }
    
    fn extract_single_file(&mut self, file_path: &str) {
        if let Some(vfs) = &self.vfs {
            let output_dir = PathBuf::from("output");
            
            match extract_file(vfs, file_path, &output_dir) {
                Ok(()) => {
                    self.status_message = format!("✓ Extracted: {}", file_path);
                }
                Err(e) => {
                    self.status_message = format!("✗ Failed: {}", e);
                }
            }
        }
    }
    
    fn extract_current_dir(&mut self) {
        if let Some(vfs) = &self.vfs {
            let all_files = vfs.list_all_files();
            println!("[DEBUG] Total files in VFS: {}", all_files.len());
            println!("[DEBUG] Current directory: '{}'", self.current_dir);
            
            let dir_prefix = if self.current_dir.is_empty() {
                String::new()
            } else {
                format!("{}/", self.current_dir.trim_matches('/'))
            };
            println!("[DEBUG] Directory prefix: '{}'", dir_prefix);
            
            // Show first 5 files for debugging
            for (i, f) in all_files.iter().take(5).enumerate() {
                println!("[DEBUG] Sample file {}: '{}'", i, f);
            }
            
            // Build a set of all directory paths to filter them out
            let mut dir_paths = HashSet::new();
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
            
            let files_to_extract: Vec<String> = all_files
                .into_iter()
                .filter(|f| {
                    // Skip if this is a directory path
                    if dir_paths.contains(*f) {
                        println!("[DEBUG] Skipping directory: '{}'", f);
                        return false;
                    }
                    
                    let matches = if self.current_dir.is_empty() {
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
                .collect();
            
            println!("[DEBUG] Files to extract: {}", files_to_extract.len());
            self.pending_extraction = Some(files_to_extract);
            self.status_message = format!("Queued {} files for extraction", self.pending_extraction.as_ref().unwrap().len());
        }
    }
    
    fn extract_selected_files(&mut self) {
        if let Some(vfs) = &self.vfs {
            let all_files = vfs.list_all_files();
            
            // Build a set of all directory paths
            let mut dir_paths = HashSet::new();
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
            
            for selected in &self.selected_files {
                if dir_paths.contains(selected) {
                    // This is a directory, extract all files under it
                    let dir_prefix = format!("{}/", selected.trim_matches('/'));
                    for file in &all_files {
                        if file.starts_with(&dir_prefix) && !dir_paths.contains(&file.to_string()) {
                            files_to_extract.push(file.to_string());
                        }
                    }
                    println!("[DEBUG] Expanded directory '{}' to {} files", selected, 
                        all_files.iter().filter(|f| f.starts_with(&dir_prefix) && !dir_paths.contains(&f.to_string())).count());
                } else {
                    // This is a file, extract it directly
                    files_to_extract.push(selected.clone());
                }
            }
            
            // Remove duplicates
            files_to_extract.sort();
            files_to_extract.dedup();
            
            println!("[DEBUG] Selected {} items, expanded to {} files", self.selected_files.len(), files_to_extract.len());
            
            self.pending_extraction = Some(files_to_extract);
            self.selected_files.clear();
            self.status_message = format!("Queued {} files for extraction", self.pending_extraction.as_ref().unwrap().len());
        }
    }
    
    fn process_extraction(&mut self, files: Vec<String>) {
        println!("[DEBUG] process_extraction called with {} files", files.len());
        
        if files.is_empty() {
            self.status_message = "No files to extract".to_string();
            return;
        }
        
        let total = files.len();
        let output_dir = PathBuf::from("output");
        println!("[DEBUG] Output directory: {:?}", output_dir);
        
        let mut success = 0;
        let mut failed = 0;
        let mut failed_files = Vec::new();
        
        self.extract_progress = Some(ExtractProgress {
            current: 0,
            total,
            success: 0,
            failed: 0,
        });
        
        if let Some(vfs) = &self.vfs {
            for (i, file_path) in files.iter().enumerate() {
                println!("[DEBUG] Extracting {}: '{}'", i + 1, file_path);
                
                match extract_file(vfs, file_path, &output_dir) {
                    Ok(()) => {
                        println!("[DEBUG] Success: '{}'", file_path);
                        success += 1;
                    }
                    Err(e) => {
                        println!("[DEBUG] Failed: '{}' - Error: {}", file_path, e);
                        failed += 1;
                        failed_files.push((file_path.clone(), e.to_string()));
                    }
                }
                
                self.extract_progress = Some(ExtractProgress {
                    current: i + 1,
                    total,
                    success,
                    failed,
                });
            }
        } else {
            println!("[DEBUG] Error: VFS not mounted");
            self.status_message = "Error: VFS not mounted".to_string();
            self.extract_progress = None;
            return;
        }
        
        if failed > 0 {
            if failed <= 3 {
                let failures = failed_files.iter()
                    .map(|(f, e)| format!("{}: {}", f, e))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.status_message = format!("✓ {} files, ✗ {} failed ({})", success, failed, failures);
            } else {
                let first_few = failed_files.iter().take(3)
                    .map(|(f, e)| format!("{}: {}", f, e))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.status_message = format!("✓ {} files, ✗ {} failed. Examples: {}", success, failed, first_few);
            }
        } else {
            self.status_message = format!("✓ Successfully extracted {} files", success);
        }
        
        self.extract_progress = None;
    }
    
    fn render_search_panel(&mut self, ctx: &egui::Context) {
        let mut show = self.show_search;
        egui::Window::new("Search Files")
            .open(&mut show)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Query:");
                    ui.text_edit_singleline(&mut self.search_query);
                    if ui.button("Search").clicked() {
                        self.perform_search();
                    }
                });
                
                ui.separator();
                
                egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                    for result in &self.search_results.clone() {
                        ui.horizontal(|ui| {
                            ui.label("📄");
                            if ui.selectable_label(false, result).clicked() {
                                self.navigate_to_file(result);
                            }
                            if ui.small_button("Extract").clicked() {
                                self.extract_single_file(result);
                            }
                        });
                    }
                });
                
                if !self.search_results.is_empty() {
                    ui.separator();
                    ui.label(format!("Found {} results", self.search_results.len()));
                    if ui.button("Extract All Results").clicked() {
                        self.pending_extraction = Some(self.search_results.clone());
                        self.status_message = format!("Queued {} files for extraction", self.pending_extraction.as_ref().unwrap().len());
                    }
                }
            });
        
        self.show_search = show;
    }
    
    fn perform_search(&mut self) {
        if let Some(vfs) = &self.vfs {
            let query = self.search_query.to_lowercase();
            self.search_results = vfs.list_all_files()
                .into_iter()
                .filter(|f| f.to_lowercase().contains(&query))
                .map(|s| s.to_string())
                .collect();
        }
    }
    
    fn navigate_to_file(&mut self, file_path: &str) {
        if let Some(last_slash) = file_path.rfind('/') {
            self.current_dir = file_path[..last_slash].to_string();
            self.selected_file = Some(file_path.to_string());
            self.refresh_entries();
        }
    }
    
    fn show_packages(&mut self, ctx: &egui::Context) {
        if let Some(vfs) = &self.vfs {
            let packages: Vec<_> = vfs.list_packages().iter().map(|name| {
                let count = vfs.get_package(name).map(|p| p.get_file_count()).unwrap_or(0);
                (name.to_string(), count)
            }).collect();
            
            let mut show = true;
            egui::Window::new("Packages")
                .open(&mut show)
                .show(ctx, |ui| {
                    for (name, count) in packages {
                        ui.horizontal(|ui| {
                            ui.label("📦");
                            ui.label(&name);
                            ui.label(format!("({} files)", count));
                        });
                    }
                });
        }
    }
}

fn list_directory(vfs: &MultiVFS, current_dir: &str) -> Vec<FileEntry> {
    let all_files = vfs.list_all_files();
    let current_prefix = if current_dir.is_empty() {
        String::new()
    } else {
        format!("{}/", current_dir.trim_matches('/'))
    };
    
    let mut entries = HashSet::new();
    
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

fn extract_file(vfs: &MultiVFS, file_path: &str, output_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
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
