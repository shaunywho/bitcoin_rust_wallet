use egui::InnerResponse;

use super::{MyApp, SidePanel};
impl MyApp {
    pub fn render_toppanel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("Headerbar")
            .exact_height(70.0)
            .show(ctx, |ui| {
                if self.dialog_box.is_some() {
                    ui.set_enabled(false);
                }
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);

                    ui.heading("Rust Bitcoin Wallet");

                    ui.add_space(10.0);
                    let title = match self.side_panel_selected {
                        SidePanel::Wallet => "Wallet",
                        SidePanel::Sending => "Send Transaction",
                        SidePanel::Receiving => "Receive Transaction",
                        SidePanel::Contacts => "Contacts",
                        SidePanel::Settings => "Settings",
                    };
                    ui.heading(title);
                })
            });
    }
}
