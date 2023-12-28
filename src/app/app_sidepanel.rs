use egui::InnerResponse;

use super::{MyApp, SidePanelState};
const DIMENSIONS: f32 = 130.0;
impl MyApp {
    pub fn render_sidepanel(
        &mut self,
        enabled: bool,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
            ui.set_enabled(enabled);

            let side_panel_state = SidePanelState::Wallet;
            ui.add_space(10.0);
            if ui
                .add_sized(
                    [DIMENSIONS, DIMENSIONS],
                    egui::ImageButton::new(egui::include_image!("../../assets/wallet.png"))
                        .rounding(5.0)
                        .selected(self.side_panel_state == side_panel_state),
                )
                .clicked()
            {
                self.side_panel_state = side_panel_state;
            }
            ui.add_space(10.0);
            let side_panel_state = SidePanelState::Sending;
            if ui
                .add_sized(
                    [DIMENSIONS, DIMENSIONS],
                    egui::ImageButton::new(egui::include_image!("../../assets/send.png"))
                        .rounding(5.0)
                        .selected(self.side_panel_state == side_panel_state),
                )
                .clicked()
            {
                self.side_panel_state = side_panel_state;
            }

            let side_panel_state = SidePanelState::Receiving;
            ui.add_space(10.0);
            if ui
                .add_sized(
                    [DIMENSIONS, DIMENSIONS],
                    egui::ImageButton::new(egui::include_image!("../../assets/receive.png"))
                        .rounding(5.0)
                        .selected(self.side_panel_state == side_panel_state),
                )
                .clicked()
            {
                self.side_panel_state = side_panel_state;
            }

            let side_panel_state = SidePanelState::Contacts;
            ui.add_space(10.0);
            if ui
                .add_sized(
                    [DIMENSIONS, DIMENSIONS],
                    egui::ImageButton::new(egui::include_image!("../../assets/contacts.png"))
                        .rounding(5.0)
                        .selected(self.side_panel_state == side_panel_state),
                )
                .clicked()
            {
                self.side_panel_state = side_panel_state;
            };
            let side_panel_state = SidePanelState::Settings;
            ui.add_space(10.0);
            if ui
                .add_sized(
                    [DIMENSIONS, DIMENSIONS],
                    egui::ImageButton::new(egui::include_image!("../../assets/settings.png"))
                        .rounding(5.0)
                        .selected(self.side_panel_state == side_panel_state),
                )
                .clicked()
            {
                self.side_panel_state = side_panel_state;
            };
        });
    }
}
