use eframe::egui;

pub struct PackagesWindowUI;

pub struct PackagesWindowResponse {
    pub should_close: bool,
    pub packages_to_unload: Vec<String>,
}

impl PackagesWindowUI {
    pub fn render(
        ctx: &egui::Context,
        show: &mut bool,
        packages: &[(String, usize)],
    ) -> PackagesWindowResponse {
        let mut response = PackagesWindowResponse {
            should_close: false,
            packages_to_unload: Vec::new(),
        };

        let mut window_open = *show;
        egui::Window::new("Mounted Packages")
            .default_width(450.0)
            .open(&mut window_open)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(format!("📦 {} Package(s)", packages.len()));
                });
                
                ui.add_space(5.0);
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(350.0)
                    .stick_to_right(true)
                    .show(ui, |ui| {
                        ui.set_min_width(420.0);
                        
                        for (package_name, file_count) in packages {
                            ui.horizontal(|ui| {
                                ui.add_space(10.0);
                                
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("📦");
                                        ui.strong(package_name);
                                        ui.separator();
                                        ui.label(format!("{} files", file_count));
                                    });
                                });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("❌ Unload").clicked() {
                                        response.packages_to_unload.push(package_name.clone());
                                    }
                                });
                            });
                            
                            ui.add_space(3.0);
                        }
                    });

                ui.separator();
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label(format!("Total: {} packages", packages.len()));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !packages.is_empty() && ui.button("🗑️ Unload All").clicked() {
                            for (package_name, _) in packages {
                                response.packages_to_unload.push(package_name.clone());
                            }
                        }
                    });
                });
            });

        if !window_open {
            response.should_close = true;
        }

        response
    }
}
