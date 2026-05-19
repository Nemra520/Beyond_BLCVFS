use eframe::egui;

pub struct SearchPanelUI;

pub struct SearchPanelResponse {
    pub should_close: bool,
    pub navigate_to: Option<String>,
    pub extract_file: Option<String>,
    pub extract_all_results: bool,
}

impl SearchPanelUI {
    #[allow(dead_code)]
    pub fn render(
        ctx: &egui::Context,
        show_search: &mut bool,
        search_query: &mut String,
        search_results: &[String],
        _vfs: &Option<blc_vfs::MultiVFS>,
    ) -> SearchPanelResponse {
        let mut response = SearchPanelResponse {
            should_close: false,
            navigate_to: None,
            extract_file: None,
            extract_all_results: false,
        };
        
        let mut show = *show_search;
        egui::Window::new("Search Files")
            .open(&mut show)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Query:");
                    let text_edit = ui.text_edit_singleline(search_query);
                    if text_edit.changed() && !search_query.is_empty() {
                        // Search results are updated through the main app
                    }
                    if ui.button("Search").clicked() || text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        // Trigger search
                    }
                });
                
                ui.separator();

                // Use virtualized rendering for search results
                let row_height = 24.0;
                let total_rows = search_results.len();

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
                                    response.navigate_to = Some(result.clone());
                                }
                                if ui.small_button("Extract").clicked() {
                                    response.extract_file = Some(result.clone());
                                }
                            });
                        }
                    });
                
                if !search_results.is_empty() {
                    ui.separator();
                    ui.label(format!("Found {} results", search_results.len()));
                    if ui.button("Extract All Results").clicked() {
                        response.extract_all_results = true;
                    }
                }
            });
        
        if !show {
            response.should_close = true;
        }
        
        response
    }
}
