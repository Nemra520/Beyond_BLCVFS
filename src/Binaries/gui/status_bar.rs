use eframe::egui;

pub struct StatusBarUI;

pub struct StatusBarResponse {
    pub extract_current_dir_clicked: bool,
    pub extract_selected_clicked: bool,
    pub clear_selection_clicked: bool,
    pub extract_manifest_current_dir_clicked: bool,
    pub extract_manifest_selected_clicked: bool,
    pub clear_manifest_selection_clicked: bool,
}

impl StatusBarUI {
    pub fn render(
        ui: &mut egui::Ui,
        package_count: usize,
        total_file_count: usize,
        current_dir: &str,
        selected_count: usize,
        is_pck_view: bool,
        is_manifest_view: bool,
        manifest_current_dir: &str,
        manifest_selected_count: usize,
    ) -> StatusBarResponse {
        let mut response = StatusBarResponse {
            extract_current_dir_clicked: false,
            extract_selected_clicked: false,
            clear_selection_clicked: false,
            extract_manifest_current_dir_clicked: false,
            extract_manifest_selected_clicked: false,
            clear_manifest_selection_clicked: false,
        };

        if is_manifest_view {
            // Manifest 视图的状态栏
            ui.horizontal(|ui| {
                ui.label("Manifest View");
                ui.separator();
                ui.label("Path:");
                ui.label(format!("/{}", manifest_current_dir));
                ui.separator();
                ui.label("Selected:");
                ui.label(format!("{}", manifest_selected_count));
            });
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("📥 Extract All").clicked() {
                    response.extract_manifest_current_dir_clicked = true;
                }

                if manifest_selected_count > 0 {
                    if ui.button(format!("📥 Extract Selected ({})", manifest_selected_count)).clicked() {
                        response.extract_manifest_selected_clicked = true;
                    }

                    if ui.button("❌ Clear Selection").clicked() {
                        response.clear_manifest_selection_clicked = true;
                    }
                }
            });
            ui.separator();
        } else {
            // 普通 VFS 视图的状态栏
            ui.horizontal(|ui| {
                ui.label("Packages:");
                ui.label(format!("{}", package_count));
                ui.separator();
                ui.label("Files:");
                ui.label(format!("{}", total_file_count));
                ui.separator();
                ui.label("Path:");
                ui.label(format!("/{}", current_dir));
                ui.separator();
                ui.label("Selected:");
                ui.label(format!("{}", selected_count));
            });
            ui.separator();

            if !is_pck_view {
                ui.horizontal(|ui| {
                    if ui.button("📥 Extract Current Dir").clicked() {
                        response.extract_current_dir_clicked = true;
                    }

                    if selected_count > 0 {
                        if ui.button(format!("📥 Extract Selected ({})", selected_count)).clicked() {
                            response.extract_selected_clicked = true;
                        }

                        if ui.button("❌ Clear Selection").clicked() {
                            response.clear_selection_clicked = true;
                        }
                    }
                });
                ui.separator();
            }
        }

        response
    }
}
