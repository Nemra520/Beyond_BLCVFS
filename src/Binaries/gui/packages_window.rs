use eframe::egui;

pub struct PackagesWindowUI;

pub struct PackagesWindowResponse {
    pub should_close: bool,
}

impl PackagesWindowUI {
    pub fn render(
        ctx: &egui::Context,
        show: &mut bool,
        packages: &[(String, usize)],
    ) -> PackagesWindowResponse {
        let mut response = PackagesWindowResponse {
            should_close: false,
        };

        let mut window_open = *show;
        egui::Window::new("Mounted Packages")
            .open(&mut window_open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        for (package_name, file_count) in packages {
                            ui.horizontal(|ui| {
                                ui.label("📦");
                                ui.label(package_name);
                                ui.separator();
                                ui.label(format!("{} files", file_count));
                            });
                        }
                    });

                ui.separator();
                ui.label(format!("Total: {} packages", packages.len()));
            });

        if !window_open {
            response.should_close = true;
        }

        response
    }
}
