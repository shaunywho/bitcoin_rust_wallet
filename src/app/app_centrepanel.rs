use crate::bitcoin_wallet::{
    generate_qrcode_from_address, get_transaction_details, TransactionDirection,
};
use egui::Ui;
use egui_extras::{Column, TableBuilder};

use super::{CentralPanelState, DialogBox, DialogBoxEnum, MyApp, SidePanelState};

use chrono::prelude::*;

use zxcvbn::zxcvbn;

impl MyApp {
    pub fn render_wallet_panel(&mut self, enabled: bool, ui: &mut Ui) {
        ui.set_enabled(enabled);
        ui.vertical_centered(|ui| {
            let wallet = self.wallet_model.get_selected_wallet_data();
            ui.heading(&wallet.wallet_name.to_owned());
            ui.add_space(10.0);
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

            let wallet = self.wallet_model.get_selected_wallet_data();
            ui.add_space(10.0);
            ui.heading(format!("Wallet Balance: {:?}", wallet.get_total()));
            ui.add_space(50.0);
            let public_key = &wallet.pub_key;
            ui.horizontal(|ui| {
                ui.label(format!("Public Key: {}", &public_key));
                if ui.button("📋").on_hover_text("Click to copy").clicked() {
                    ui.output_mut(|o| o.copied_text = public_key.to_string());
                }
            });

            TableBuilder::new(ui)
                .column(Column::exact(200.0).resizable(false))
                .column(Column::exact(70.0))
                .column(Column::exact(150.0))
                .column(Column::exact(350.0))
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
                    header.col(|ui| {
                        ui.heading("Destination");
                    });
                })
                .body(|mut body| {
                    let wallet = self.wallet_model.get_selected_wallet_data();
                    if let Some(transactions) = wallet.sorted_transactions.clone() {
                        for transaction_details in transactions.iter() {
                            let (
                                transaction_direction,
                                address,
                                txid,
                                transaction_total,
                                fee,
                                confirmation_time,
                            ) = get_transaction_details(transaction_details.clone());
                            body.row(30.0, |mut row| {
                                row.col(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("📋").on_hover_text("Click to copy").clicked()
                                        {
                                            ui.output_mut(|o| o.copied_text = txid.clone());
                                        }
                                        let shortened_txid = txid.clone()[0..10].to_string()
                                            + "..."
                                            + &txid[txid.len() - 10..txid.len()];

                                        ui.label(format!("{}", shortened_txid)).on_hover_text(txid)
                                    });
                                });

                                row.col(|ui| {
                                    let transaction_string = match transaction_direction {
                                        TransactionDirection::To => {
                                            format!("{} (fee: {})", transaction_total, fee)
                                        }
                                        TransactionDirection::From => {
                                            format!("+{}", transaction_total)
                                        }
                                    };
                                    ui.label(transaction_string);
                                });

                                row.col(|ui| {
                                    let confirmation_time_str = match confirmation_time {
                                        Some(confirmation_time) => {
                                            let confirmation_time_local = Local
                                                .timestamp_opt(
                                                    confirmation_time.timestamp as i64,
                                                    0,
                                                )
                                                .unwrap();
                                            confirmation_time_local
                                                .format("%d/%m/%y %H:%M:%S")
                                                .to_string()
                                        }
                                        None => "Pending".to_string(),
                                    };

                                    ui.label(confirmation_time_str);
                                });
                                row.col(|ui| {
                                    let destination_string = match transaction_direction {
                                        TransactionDirection::To => format!(
                                            "To {}",
                                            self.wallet_model.get_wallet_name(&address)
                                        ),
                                        TransactionDirection::From => format!(
                                            "From {}",
                                            self.wallet_model.get_wallet_name(&address)
                                        ),
                                    };

                                    ui.label(destination_string);
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
            self.wallet_model.get_selected_wallet_data().get_total()
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
        let wallet = self.wallet_model.get_selected_wallet_data();
        let public_key = &wallet.pub_key;
        ui.vertical_centered(|ui| {
            ui.label(format!("Public Key: {:?}", public_key));
            // Encode some data into bits.

            let img = ui.ctx().load_texture(
                "my-image",
                generate_qrcode_from_address(&public_key).unwrap(),
                Default::default(),
            );

            ui.add(egui::Image::from_texture(&img));
        });
    }

    pub fn render_contacts_panel(&mut self, enabled: bool, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            TableBuilder::new(ui)
                .column(Column::exact(250.0).resizable(false))
                .column(Column::exact(350.0))
                .column(Column::exact(100.0))
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Wallet Name");
                    });
                    header.col(|ui| {
                        ui.heading("Public Key");
                    });
                    header.col(|ui| {});
                })
                .body(|mut body| {
                    let contacts = &self.wallet_model.json_wallet_data.contacts;
                    for contact in contacts.iter() {
                        body.row(30.0, |mut row| {
                            row.col(|ui| {
                                ui.horizontal(|ui| {
                                    if ui
                                        .button(contact.wallet_name.clone())
                                        .on_hover_text("Change")
                                        .clicked()
                                    {
                                        let wallet_name = contact.wallet_name.clone();
                                        self.dialog_box = Some(DialogBox {
                                            dialog_box_enum: DialogBoxEnum::ChangeContactName {
                                                pub_key: contact.pub_key.clone(),
                                            },
                                            title: "Change Wallet Name",
                                            message: Some(
                                                format!("Wallet name for {}", contact.pub_key)
                                                    .into(),
                                            ),
                                            line_edit: Some(wallet_name),
                                            optional: true,
                                        });
                                    }
                                });
                            });

                            row.col(|ui| {
                                ui.label(contact.pub_key.clone());
                            });
                            row.col(|ui| {
                                if ui.button("📋").on_hover_text("Click to copy").clicked() {
                                    ui.output_mut(|o| o.copied_text = contact.pub_key.clone());
                                }
                            });
                        });
                    }
                });
        });
    }

    pub fn render_create_wallet_panel(&mut self, ui: &mut Ui, mnemonic_string: &str) {
        ui.heading("Write down the following mnemonic");
        ui.horizontal(|ui| {
            ui.label(mnemonic_string);
            if ui.button("📋").on_hover_text("Copy Mnemonic").clicked() {
                ui.output_mut(|o| o.copied_text = mnemonic_string.to_string());
            }
        });
        ui.label("Type and confirm the mnemonic above");
        ui.text_edit_singleline(&mut self.confirm_mnemonic_string);
        if ui.button("Confirm").clicked() {
            if self.confirm_mnemonic_string == mnemonic_string {
                self.wallet_model
                    .add_wallet_from_mnemonic(&mnemonic_string)
                    .unwrap();
                self.dialog_box = Some(DialogBox {
                    dialog_box_enum: DialogBoxEnum::WalletCreated,
                    title: "Wallet Created",
                    message: None,
                    line_edit: None,
                    optional: false,
                });
                self.central_panel_state = CentralPanelState::WalletAvailable {
                    last_interaction_time: chrono::offset::Local::now(),
                };
            } else {
                self.dialog_box = Some(DialogBox {
                    dialog_box_enum: DialogBoxEnum::IncorrectMnemonic,
                    title: "Incorrect Mnemonic",
                    message: Some("Check your entry and type in the mnemonic again".into()),
                    line_edit: None,
                    optional: false,
                })
            }
        }
    }
    pub fn render_create_password_panel(&mut self, ui: &mut Ui) {
        ui.label("Type Password");
        let password_entry =
            egui::TextEdit::singleline(&mut self.password_entry_string).password(true);
        ui.add(password_entry);
        ui.label("Type your password again");
        let password_entry_confirmation =
            egui::TextEdit::singleline(&mut self.password_entry_confirmation_string).password(true);
        ui.add(password_entry_confirmation);
        let mut password_strength = 0;

        if let Ok(password_strength_estimate) = zxcvbn(&self.password_entry_string, &[]) {
            password_strength = password_strength_estimate.score();
        }

        if self.password_entry_string != self.password_entry_confirmation_string {
            ui.label("passwords not the same");
        } else {
            let password_strength_bar =
                egui::widgets::ProgressBar::new(password_strength as f32 / 4.0);
            ui.add(password_strength_bar);
            let password_strength_string: &str;
            match password_strength {
                0 => password_strength_string = "Very Weak",
                1 => password_strength_string = "Weak",
                2 => password_strength_string = "So-so",
                3 => password_strength_string = "Good",
                4 => password_strength_string = "Great",
                _ => password_strength_string = "NP-Badboy",
            }
            ui.label(password_strength_string);
            if ui.button("Enter").clicked() {
                self.wallet_model
                    .create_passworded_file(self.password_entry_string.clone())
                    .unwrap();
            }
        };
    }

    pub fn render_enter_password_panel(&mut self, ui: &mut Ui) {
        ui.set_enabled(true);
        let password_entry =
            egui::TextEdit::singleline(&mut self.password_entry_string).password(true);
        ui.add(password_entry);
        if ui.button("Enter").clicked() {
            if self
                .wallet_model
                .validate_password(&self.password_entry_string)
            {
                self.central_panel_state = CentralPanelState::WalletNotInitialised;
            } else {
                self.password_entry_string = String::new();
            }
        }
    }

    pub fn render_settings_panel(&mut self, enabled: bool, ui: &mut Ui) {}

    pub fn render_centrepanel(
        &mut self,
        enabled: bool,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| match &self.central_panel_state {
            CentralPanelState::WalletFileNotAvailable => self.render_create_password_panel(ui),
            CentralPanelState::NoWalletsInWalletFile { mnemonic_string } => {
                let mnemonic_string = mnemonic_string.clone();
                self.render_create_wallet_panel(ui, &mnemonic_string);
            }
            CentralPanelState::WalletNotInitialised => {}
            CentralPanelState::PasswordNeeded => self.render_enter_password_panel(ui),
            CentralPanelState::WalletAvailable {
                last_interaction_time,
            } => match self.side_panel_state {
                SidePanelState::Wallet => self.render_wallet_panel(enabled, ui),
                SidePanelState::Sending => self.render_sending_panel(enabled, ui),
                SidePanelState::Receiving => self.render_receiving_panel(enabled, ui),
                SidePanelState::Contacts => self.render_contacts_panel(enabled, ui),
                SidePanelState::Settings => self.render_settings_panel(enabled, ui),
            },
        });
    }
}
