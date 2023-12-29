use super::{CentralPanelState, MyApp, SidePanel};
const DIMENSIONS: f32 = 130.0;
impl MyApp {
    pub fn render_sidepanel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
            match (self.dialog_box.clone(), self.side_panel_app_initialising()) {
                (None, false) => ui.set_enabled(true),
                (_, _) => ui.set_enabled(false),
            }

            let side_panel_active = SidePanel::Wallet;
            ui.add_space(10.0);
            if ui
                .add_sized(
                    [DIMENSIONS, DIMENSIONS],
                    egui::ImageButton::new(egui::include_image!("../../assets/wallet.png"))
                        .rounding(5.0)
                        .selected(self.side_panel_active == side_panel_active),
                )
                .clicked()
            {
                self.side_panel_set_state(side_panel_active, CentralPanelState::WalletMain);
            }
            ui.add_space(10.0);
            let side_panel_active = SidePanel::Sending;
            if ui
                .add_sized(
                    [DIMENSIONS, DIMENSIONS],
                    egui::ImageButton::new(egui::include_image!("../../assets/send.png"))
                        .rounding(5.0)
                        .selected(self.side_panel_active == side_panel_active),
                )
                .clicked()
            {
                self.side_panel_set_state(side_panel_active, CentralPanelState::SendingMain);
            }

            let side_panel_active = SidePanel::Receiving;
            ui.add_space(10.0);
            if ui
                .add_sized(
                    [DIMENSIONS, DIMENSIONS],
                    egui::ImageButton::new(egui::include_image!("../../assets/receive.png"))
                        .rounding(5.0)
                        .selected(self.side_panel_active == side_panel_active),
                )
                .clicked()
            {
                self.side_panel_set_state(side_panel_active, CentralPanelState::ReceivingMain);
            }

            let side_panel_active = SidePanel::Contacts;
            ui.add_space(10.0);
            if ui
                .add_sized(
                    [DIMENSIONS, DIMENSIONS],
                    egui::ImageButton::new(egui::include_image!("../../assets/contacts.png"))
                        .rounding(5.0)
                        .selected(self.side_panel_active == side_panel_active),
                )
                .clicked()
            {
                self.side_panel_set_state(side_panel_active, CentralPanelState::ContactMain);
            };
            let side_panel_active = SidePanel::Settings;
            ui.add_space(10.0);
            if ui
                .add_sized(
                    [DIMENSIONS, DIMENSIONS],
                    egui::ImageButton::new(egui::include_image!("../../assets/settings.png"))
                        .rounding(5.0)
                        .selected(self.side_panel_active == side_panel_active),
                )
                .clicked()
            {
                self.side_panel_set_state(side_panel_active, CentralPanelState::SettingsMain);
            };
        });
    }

    pub fn side_panel_set_state(
        &mut self,
        side_panel_active: SidePanel,
        central_panel_state: CentralPanelState,
    ) {
        self.side_panel_active = side_panel_active;
        self.central_panel_state = central_panel_state;
    }

    pub fn side_panel_app_initialising(&mut self) -> bool {
        match self.central_panel_state {
            CentralPanelState::WalletFileNotAvailable
            | CentralPanelState::NoWalletsInWalletFile { mnemonic_string: _ }
            | CentralPanelState::WalletNotInitialised
            | CentralPanelState::PasswordNeeded { .. } => true,
            _ => false,
        }
    }
}
