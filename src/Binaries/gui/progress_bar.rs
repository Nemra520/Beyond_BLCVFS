use eframe::egui;
use super::types::ExtractProgress;

pub struct ProgressBarUI;

impl ProgressBarUI {
    pub fn render(ui: &mut egui::Ui, progress: &ExtractProgress) {
        let percentage = if progress.total > 0 {
            progress.current as f32 / progress.total as f32
        } else {
            0.0
        };

        ui.separator();

        // Progress bar
        let progress_bar = egui::ProgressBar::new(percentage)
            .text(format!("Extracting... {:.0}% ({}/{})"
                , percentage * 100.0
                , progress.current
                , progress.total));
        ui.add(progress_bar);

        // Status text
        ui.horizontal(|ui| {
            ui.label(format!("✓ Success: {}  ✗ Failed: {}", progress.success, progress.failed));
        });
    }
}
