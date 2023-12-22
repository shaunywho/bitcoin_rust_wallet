use egui::InnerResponse;

use super::MyApp;
impl MyApp {
    pub fn render_toppanel(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) -> InnerResponse<()> {
        let response = egui::TopBottomPanel::top("Headerbar")
            .exact_height(50.0)
            .show(ctx, |ui| {
                if self.dialog_box.is_some() {
                    ui.set_enabled(false);
                }
                ui.heading("Rust Bitcoin Wallet");
            });
        return response;
    }
}
