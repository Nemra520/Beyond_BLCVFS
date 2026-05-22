use eframe::egui;

pub struct StatusBarUI;

pub struct StatusBarResponse {
    pub extract_current_dir_clicked: bool,
    pub extract_selected_clicked: bool,
    pub clear_selection_clicked: bool,
}

impl StatusBarUI {
    pub fn render(
        ui: &mut egui::Ui,
        package_count: usize,
        total_file_count: usize,
        current_dir: &str,
        selected_count: usize,
        is_pck_view: bool,
    ) -> StatusBarResponse {
        let mut response = StatusBarResponse {
            extract_current_dir_clicked: false,
            extract_selected_clicked: false,
            clear_selection_clicked: false,
        };

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

        response
    }
}
