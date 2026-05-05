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
    entries: Vec<FileEntry>,
    status_message: String,
    extract_progress: Option<ExtractProgress>,
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
            entries: Vec::new(),
            status_message: "Click 'Mount' to select a VFS folder".to_string(),
            extract_progress: None,
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
                        self.refresh_entries();
                    }
                    
                    if !self.current_dir.is_empty() {
                        if ui.button("⬆ Up").clicked() {
                            self.go_up();
                        }
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
                });
                ui.separator();
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
                    
                    for entry in &self.entries {
                        ui.horizontal(|ui| {
                            let icon = if entry.is_dir { "📁" } else { "📄" };
                            let text = format!("{} {}", icon, entry.name);
                            
                            if ui.selectable_label(false, text).clicked() {
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
    let data = vfs.read_file(file_path)?;
    let output_path = output_dir.join(file_path);
    
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    std::fs::write(&output_path, data)?;
    
    Ok(())
}
