use egui::{InnerResponse, Ui};
use egui_extras::{Column, TableBuilder};

use super::{
    generate_qrcode_from_address, CentralPanelState, DialogBox, DialogBoxEnum, MyApp,
    SidePanelState,
};
use crate::bitcoin_wallet::generate_mnemonic_string;
use chrono::prelude::*;
use chrono_tz::Tz;

impl MyApp {
    pub fn render_wallet_panel(&mut self, enabled: bool, ui: &mut Ui) {
        ui.set_enabled(enabled);
        ui.vertical_centered(|ui| {
            let wallet = self.wallet_data.get_selected_wallet_element();
            ui.heading(format!("Wallet Name: {}", &wallet.wallet_name.to_owned()));
            // let wallet_name = self
            if ui.button("Rename Wallet").clicked() {
                self.rename_wallet_string = wallet.wallet_name.to_string();
                self.dialog_box = Some(DialogBox {
                    dialog_box_enum: DialogBoxEnum::ChangeWalletName,
                    title: "Change Wallet Name",
                    message: Some("Enter new wallet name".into()),
                    line_edit: Some(self.rename_wallet_string.clone()),
                    optional: true,
                });
            }
        });
        let wallet = self.wallet_data.get_selected_wallet_element();
        ui.heading(format!("Wallet Balance: {:?}", wallet.get_total()));
        ui.add_space(50.0);
        let public_key = &wallet.address;

        ui.label(format!("Public Key: {:?}", &public_key));
        if ui.button("Copy").clicked() {
            ui.output_mut(|o| o.copied_text = public_key.to_string());
        };

        ui.vertical_centered(|ui| {
            TableBuilder::new(ui)
                .column(Column::exact(400.0).resizable(false))
                .column(Column::exact(200.0))
                .column(Column::exact(200.0))
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Txid");
                    });
                    header.col(|ui| {
                        ui.heading("Amount");
                    });
                    header.col(|ui| {
                        ui.heading("Date");
                    });
                })
                .body(|mut body| {
                    let wallet = self.wallet_data.get_selected_wallet_element();
                    if let Some(transactions) = wallet.sorted_transactions.clone() {
                        for transaction in transactions.iter() {
                            body.row(30.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(format!("{}", transaction.txid));
                                });

                                row.col(|ui| {
                                    let transaction_total =
                                        transaction.received as i64 - transaction.sent as i64;
                                    let transaction_string = if transaction_total < 0 {
                                        format!(
                                            "{} (fee: {})",
                                            transaction_total,
                                            transaction.fee.unwrap()
                                        )
                                    } else {
                                        format!("+{}", transaction_total)
                                    };
                                    ui.label(transaction_string);
                                });

                                row.col(|ui| {
                                    let confirmation_time_str: String;
                                    if let Some(confirmation_time) = &transaction.confirmation_time
                                    {
                                        let confirmation_time_local = Local
                                            .timestamp_opt(confirmation_time.timestamp as i64, 0)
                                            .unwrap();

                                        confirmation_time_str = format!(
                                            "{:?} Confirmed",
                                            confirmation_time_local.to_string()
                                        );
                                    } else {
                                        confirmation_time_str = "Pending".to_string();
                                    }
                                    ui.label(confirmation_time_str);
                                });
                            });
                        }
                    }
                });
        });
    }
    pub fn render_sending_panel(&mut self, enabled: bool, ui: &mut Ui) {
        ui.set_enabled(enabled);
        ui.heading(format!(
            "Wallet Balance: {:?}",
            self.wallet_data.get_selected_wallet_element().get_total()
        ));
        ui.add_space(50.0);
        ui.vertical_centered(|ui| {
            ui.heading("Recipient Address");
            ui.text_edit_singleline(&mut self.recipient_address_string);
        });
        ui.add_space(50.0);

        ui.vertical_centered(|ui| {
            ui.heading("Amount to send");
            ui.text_edit_singleline(&mut self.amount_to_send_string);
            ui.label("Sats");
        });
        if ui.button("Send").clicked() {
            let (valid, invalid_vec) = self.is_valid_transaction_request();

            if valid {
                self.dialog_box = Some(DialogBox {
                    dialog_box_enum: DialogBoxEnum::ConfirmSend,
                    title: "Confirm Transaction",
                    message: Some(
                        format!(
                            "Are you sure you want to send {} Sats to {}?",
                            &self.amount_to_send_string, &self.recipient_address_string
                        )
                        .into(),
                    ),
                    line_edit: None,
                    optional: true,
                });
            } else {
                let invalid_message = invalid_vec.join("\n");
                self.dialog_box = Some(DialogBox {
                    dialog_box_enum: DialogBoxEnum::InvalidTransaction,
                    title: "Invalid Transaction",
                    message: Some(invalid_message),
                    line_edit: None,
                    optional: false,
                })
            }
        }
    }

    pub fn render_receiving_panel(&mut self, enabled: bool, ui: &mut Ui) {
        ui.set_enabled(enabled);
        let wallet = self.wallet_data.get_selected_wallet_element();
        let address = &wallet.address;
        ui.label(format!("Public Key: {:?}", address));
        // Encode some data into bits.

        let img = ui.ctx().load_texture(
            "my-image",
            generate_qrcode_from_address(&address).unwrap(),
            Default::default(),
        );

        ui.add(egui::Image::from_texture(&img));
    }

    pub fn render_contacts_panel(&mut self, enabled: bool, ui: &mut Ui) {}

    pub fn render_create_wallet_panel(&mut self, ui: &mut Ui) {
        if ui.button("Create Wallet").clicked() {
            // self.string_scratchpad[0] = generate_mnemonic_string().unwrap();
            let new_mnemonic = generate_mnemonic_string().unwrap();
            self.dialog_box = Some(DialogBox {
                dialog_box_enum: DialogBoxEnum::NewMnemonic,
                title: "New Wallet Mnemonic",
                message: Some(new_mnemonic),
                line_edit: None,
                optional: false,
            });
        }
    }

    pub fn render_password_panel(&mut self, ui: &mut Ui) {
        ui.set_enabled(true);
        let password_entry =
            egui::TextEdit::singleline(&mut self.password_entry_string).password(true);
        ui.add(password_entry);
        if ui.button("Enter").clicked() {
            self.password_entry_string = String::new();
            self.central_panel_state = CentralPanelState::WalletAvailable;
        }
    }

    pub fn render_centrepanel(
        &mut self,
        enabled: bool,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| match self.central_panel_state {
            CentralPanelState::WalletFileNotAvailable => self.render_create_wallet_panel(ui),
            CentralPanelState::WalletNotInitialised => {}
            CentralPanelState::PasswordNeeded => self.render_password_panel(ui),
            _ => match self.side_panel_state {
                SidePanelState::Wallet => self.render_wallet_panel(enabled, ui),
                SidePanelState::Sending => self.render_sending_panel(enabled, ui),
                SidePanelState::Receiving => self.render_receiving_panel(enabled, ui),
                SidePanelState::Contacts => self.render_contacts_panel(enabled, ui),
            },
        });
    }
}
