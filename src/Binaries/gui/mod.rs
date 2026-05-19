use eframe::egui;
use std::collections::HashSet;
use std::path::PathBuf;

mod types;
mod pck_view;
mod file_browser;
mod search_panel;
mod extraction;

pub use types::*;
pub use pck_view::PckViewUI;
pub use file_browser::{FileBrowserUI, list_directory};
pub use extraction::ExtractionManager;

// SearchPanelUI is defined but currently unused - kept for future use
#[allow(unused_imports)]
pub use search_panel::SearchPanelUI;

pub struct BlcVfsApp {
    vfs: Option<blc_vfs::MultiVFS>,
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
    pck_view: Option<PckView>,
    // Async extraction state
    extraction_state: Option<ExtractionState>,
}

struct ExtractionState {
    files: Vec<String>,
    current_index: usize,
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
            pck_view: None,
            extraction_state: None,
        }
    }
}

impl eframe::App for BlcVfsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Progress bar at top (if extracting) - render before CentralPanel
        if self.extract_progress.is_some() {
            egui::TopBottomPanel::top("progress_panel").show(ctx, |ui| {
                self.render_progress(ui);
            });
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header with buttons
            self.render_header(ui, ctx);
            ui.separator();

            // Status bar
            self.render_status_bar(ui);

            // Search panel
            if self.show_search {
                self.render_search_panel(ctx);
            }

            // Main content area
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(ref mut pck_view) = self.pck_view {
                    PckViewUI::render(ui, pck_view, &mut self.status_message, &self.vfs);
                } else {
                    self.render_file_browser(ui);
                }
            });

            // Bottom status
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
            });
        });
        
        // Process pending extraction - initialize async state
        if let Some(files) = self.pending_extraction.take() {
            let total = files.len();
            self.extract_progress = Some(ExtractProgress {
                current: 0,
                total,
                success: 0,
                failed: 0,
            });
            self.extraction_state = Some(ExtractionState {
                files,
                current_index: 0,
                success: 0,
                failed: 0,
            });
            ctx.request_repaint();
        }

        // Process one file per frame to keep UI responsive
        if let Some(ref mut state) = self.extraction_state {
            if state.current_index < state.files.len() {
                if let Some(ref vfs) = self.vfs {
                    let output_dir = PathBuf::from("output");
                    let file = &state.files[state.current_index];

                    match ExtractionManager::extract_single_file(vfs, file, &output_dir) {
                        Ok(()) => state.success += 1,
                        Err(_) => state.failed += 1,
                    }

                    state.current_index += 1;

                    // Update progress
                    self.extract_progress = Some(ExtractProgress {
                        current: state.current_index,
                        total: state.files.len(),
                        success: state.success,
                        failed: state.failed,
                    });

                    // Request repaint to show progress
                    ctx.request_repaint();
                }
            } else {
                // Extraction complete
                self.status_message = format!(
                    "✓ Extracted {} files, ✗ {} failed",
                    state.success, state.failed
                );
                self.extract_progress = None;
                self.extraction_state = None;
            }
        }
    }
}

impl BlcVfsApp {
    fn render_header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.heading("BLC Virtual File System v0.1.0");
            ui.separator();
            
            if ui.button("📁 Mount").clicked() {
                self.mount_folder(ctx);
            }
            
            if self.vfs.is_some() {
                if self.pck_view.is_some() {
                    if ui.button("⬅ Back to VFS").clicked() {
                        self.pck_view = None;
                    }
                } else {
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
                }
                
                if ui.button("🔍 Search").clicked() {
                    self.show_search = true;
                }
                
                if ui.button("📦 Packages").clicked() {
                    self.show_packages(ctx);
                }
            }
        });
    }
    
    fn render_status_bar(&mut self, ui: &mut egui::Ui) {
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
            
            if self.pck_view.is_none() {
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
        }
    }
    
    fn render_file_browser(&mut self, ui: &mut egui::Ui) {
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
            return;
        }
        
        let response = FileBrowserUI::render(
            ui,
            &self.entries,
            &self.selected_file,
            &mut self.selected_files,
        );
        
        // Handle file browser responses
        if let Some(path) = response.toggle_selection {
            if self.selected_files.contains(&path) {
                self.selected_files.remove(&path);
            } else {
                self.selected_files.insert(path);
            }
        }
        
        if let Some(dir) = response.new_dir {
            self.current_dir = dir;
            self.refresh_entries();
        }
        
        if let Some(file) = response.extract_file {
            self.extract_single_file(&file);
        }
        
        if let Some(pck_path) = response.open_pck {
            self.open_pck_view(&pck_path);
        }
    }
    
    fn render_search_panel(&mut self, ctx: &egui::Context) {
        let mut show = self.show_search;
        egui::Window::new("Search Files")
            .open(&mut show)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Query:");
                    let text_edit = ui.text_edit_singleline(&mut self.search_query);
                    if text_edit.changed() && !self.search_query.is_empty() {
                        self.perform_search();
                    }
                    if ui.button("Search").clicked() {
                        self.perform_search();
                    }
                });
                
                ui.separator();

                // Use virtualized rendering for search results
                let row_height = 24.0;
                let total_rows = self.search_results.len();
                let search_results = self.search_results.clone();

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show_rows(ui, row_height, total_rows, |ui, row_range| {
                        for row_index in row_range {
                            if row_index >= search_results.len() {
                                break;
                            }
                            let result = &search_results[row_index];

                            ui.horizontal(|ui| {
                                ui.set_min_height(row_height);
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
    
    fn render_progress(&self, ui: &mut egui::Ui) {
        if let Some(progress) = &self.extract_progress {
            let percentage = if progress.total > 0 {
                progress.current as f32 / progress.total as f32
            } else {
                0.0
            };

            ui.separator();

            // Progress bar
            let progress_bar = egui::ProgressBar::new(percentage)
                .text(format!("Extracting... {:.0}% ({}/{})", percentage * 100.0, progress.current, progress.total));
            ui.add(progress_bar);

            // Status text
            ui.horizontal(|ui| {
                ui.label(format!("✓ Success: {}  ✗ Failed: {}", progress.success, progress.failed));
            });
        }
    }
    
    // Helper methods
    fn mount_folder(&mut self, ctx: &egui::Context) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.status_message = format!("Mounting: {}", folder.display());
            ctx.request_repaint();
            
            match blc_vfs::MultiVFS::mount_folder(&folder) {
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
            
            match ExtractionManager::extract_single_file(vfs, file_path, &output_dir) {
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
            let files_to_extract = ExtractionManager::get_files_in_current_dir(vfs, &self.current_dir);
            
            println!("[DEBUG] Files to extract: {}", files_to_extract.len());
            self.pending_extraction = Some(files_to_extract);
            self.status_message = format!("Queued {} files for extraction", self.pending_extraction.as_ref().unwrap().len());
        }
    }
    
    fn extract_selected_files(&mut self) {
        if let Some(vfs) = &self.vfs {
            let files_to_extract = ExtractionManager::get_selected_files(vfs, &self.selected_files);
            
            self.pending_extraction = Some(files_to_extract);
            self.status_message = format!("Queued {} files for extraction", self.pending_extraction.as_ref().unwrap().len());
        }
    }
    
    fn perform_search(&mut self) {
        if let Some(vfs) = &self.vfs {
            let query = self.search_query.to_lowercase();
            const MAX_RESULTS: usize = 1000;

            self.search_results = vfs.list_all_files()
                .into_iter()
                .filter(|f| f.to_lowercase().contains(&query))
                .take(MAX_RESULTS)
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
    
    fn open_pck_view(&mut self, pck_path: &str) {
        if let Some(vfs) = &self.vfs {
            match vfs.read_file(pck_path) {
                Ok(data) => {
                    match blc_vfs::PckExtractor::extract_pck(&data) {
                        Ok(result) => {
                            let entries: Vec<PckEntryView> = result.entries.iter().map(|e| {
                                let entry_type = match e.entry_type {
                                    blc_vfs::VFS::FileType::PCK::pck_extractor::PckEntryType::Wem => "WEM".to_string(),
                                    blc_vfs::VFS::FileType::PCK::pck_extractor::PckEntryType::WemX => "WEM".to_string(),
                                    blc_vfs::VFS::FileType::PCK::pck_extractor::PckEntryType::Bnk => "BNK".to_string(),
                                    blc_vfs::VFS::FileType::PCK::pck_extractor::PckEntryType::Plg => "PLG".to_string(),
                                    blc_vfs::VFS::FileType::PCK::pck_extractor::PckEntryType::Unknown => "BIN".to_string(),
                                };
                                PckEntryView {
                                    file_id: e.file_id,
                                    entry_type,
                                    size: e.data.len(),
                                }
                            }).collect();
                            
                            let parent_dir = if let Some(last_slash) = pck_path.rfind('/') {
                                pck_path[..last_slash].to_string()
                            } else {
                                String::new()
                            };
                            
                            self.pck_view = Some(PckView {
                                pck_path: pck_path.to_string(),
                                entries,
                                parent_dir,
                                selected_entries: HashSet::new(),
                            });
                            self.status_message = format!("✓ Opened PCK: {} ({} entries)", pck_path, result.entries.len());
                        }
                        Err(e) => {
                            self.status_message = format!("✗ Failed to parse PCK: {}", e);
                        }
                    }
                }
                Err(e) => {
                    self.status_message = format!("✗ Failed to read PCK: {}", e);
                }
            }
        }
    }
    
    fn show_packages(&self, ctx: &egui::Context) {
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

pub fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("BLC Virtual File System"),
        ..Default::default()
    };
    
    eframe::run_native("BLC VFS", options, Box::new(|_cc| Ok(Box::new(BlcVfsApp::default()))))
}
