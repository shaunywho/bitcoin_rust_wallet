use egui::InnerResponse;

use super::{MyApp, SidePanelState};
impl MyApp {
    pub fn render_toppanel(
        &mut self,
        enabled: bool,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        egui::TopBottomPanel::top("Headerbar")
            .exact_height(70.0)
            .show(ctx, |ui| {
                ui.set_enabled(enabled);
                if self.dialog_box.is_some() {
                    ui.set_enabled(false);
                }
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);

                    ui.heading("Rust Bitcoin Wallet");

                    ui.add_space(10.0);
                    let title = match self.side_panel_state {
                        SidePanelState::Wallet => "Wallet",
                        SidePanelState::Sending => "Send Transaction",
                        SidePanelState::Receiving => "Receive Transaction",
                        SidePanelState::Contacts => "Contacts",
                        SidePanelState::Settings => "Settings",
                    };
                    ui.heading(title);
                })
            });
    }
}
