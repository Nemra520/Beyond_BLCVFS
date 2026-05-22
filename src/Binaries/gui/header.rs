use eframe::egui;

pub struct HeaderUI;

pub struct HeaderResponse {
    pub mount_clicked: bool,
    pub back_clicked: bool,
    pub root_clicked: bool,
    pub up_clicked: bool,
    pub search_clicked: bool,
    pub packages_clicked: bool,
}

impl HeaderUI {
    pub fn render(
        ui: &mut egui::Ui,
        has_vfs: bool,
        is_pck_view: bool,
        has_parent_dir: bool,
    ) -> HeaderResponse {
        let mut response = HeaderResponse {
            mount_clicked: false,
            back_clicked: false,
            root_clicked: false,
            up_clicked: false,
            search_clicked: false,
            packages_clicked: false,
        };

        ui.horizontal(|ui| {
            ui.heading("BLC Virtual File System v0.1.0");
            ui.separator();

            if ui.button("📁 Mount").clicked() {
                response.mount_clicked = true;
            }

            if has_vfs {
                if is_pck_view {
                    if ui.button("⬅ Back to VFS").clicked() {
                        response.back_clicked = true;
                    }
                } else {
                    if ui.button("🏠 Root").clicked() {
                        response.root_clicked = true;
                    }

                    if has_parent_dir {
                        if ui.button("⬆ Up").clicked() {
                            response.up_clicked = true;
                        }
                    }
                }

                if ui.button("🔍 Search").clicked() {
                    response.search_clicked = true;
                }

                if ui.button("📦 Packages").clicked() {
                    response.packages_clicked = true;
                }
            }
        });

        response
    }
}
