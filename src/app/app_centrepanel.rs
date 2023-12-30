use std::str::FromStr;

use crate::{
    bitcoin_wallet::{
        generate_mnemonic_string, generate_qrcode_from_address, generate_wallet, generate_xpriv,
        get_transaction_details, is_valid_bitcoin_address, TransactionDirection,
    },
    wallet_file_manager::EntryType,
};
use bdk::bitcoin::bip32::ExtendedPrivKey;
use egui::Ui;
use egui_extras::{Column, TableBuilder};

use super::{CentralPanelState, DialogBox, DialogBoxEnum, DialogLineEdit, MyApp};

use chrono::prelude::*;

use zxcvbn::zxcvbn;

impl MyApp {
    pub fn render_wallet_main_panel(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        let wallet = self.wallet_model.get_active_wallet_data();
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.heading(&wallet.wallet_name.to_owned());
        });
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Selected Wallet")
                .selected_text(format!(
                    "{}",
                    self.wallet_model.get_active_wallet_data().wallet_name
                ))
                .show_ui(ui, |ui| {
                    for wallet in self.wallet_model.json_wallet_data.wallets.iter() {
                        if ui
                            .selectable_value(
                                &mut self.wallet_model.active_wallet.clone().unwrap(),
                                wallet.pub_key.clone(),
                                wallet.wallet_name.clone(),
                            )
                            .clicked()
                        {
                            self.wallet_model.active_wallet = Some(wallet.pub_key.clone());
                        };
                    }
                });
            if ui.button("Show Mnemonic").clicked() {
                self.change_state(CentralPanelState::WalletSecret);
            }
            if ui.button("Rename Wallet").clicked() {
                self.change_state(CentralPanelState::WalletRename);
            }
            if self.wallet_model.json_wallet_data.wallets.len() > 1 {
                if ui.button("Delete Wallet").clicked() {
                    self.change_state(CentralPanelState::WalletDelete);
                }
            }
            if ui.button("Add New Wallet").clicked() {
                let mnemonic_string = generate_mnemonic_string().unwrap();
                self.change_state(CentralPanelState::WalletNewWallet {
                    mnemonic_string: mnemonic_string,
                })
            }
            if ui.button("Add Existing Wallet").clicked() {
                self.change_state(CentralPanelState::WalletExistingWallet)
            }
        });
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            let wallet = self.wallet_model.get_active_wallet_data();
            ui.add_space(10.0);
            ui.heading(format!("Wallet Balance: {:?}", wallet.get_total()));
            ui.add_space(50.0);

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
                        ui.heading("Recipient");
                    });
                })
                .body(|mut body| {
                    let wallet = self.wallet_model.get_active_wallet_data();
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
                                        TransactionDirection::To => self
                                            .wallet_model
                                            .get_wallet_name(&address)
                                            .unwrap_or_else(|| address.clone()),
                                        TransactionDirection::From => self
                                            .wallet_model
                                            .get_wallet_name(
                                                &self.wallet_model.get_active_wallet_pub_key(),
                                            )
                                            .unwrap_or_else(|| {
                                                self.wallet_model.get_active_wallet_pub_key()
                                            }),
                                    };

                                    ui.label(destination_string);
                                });
                            });
                        }
                    }
                });
        });
    }
    pub fn render_sending_panel(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.heading(format!(
                "Wallet Balance: {:?}",
                self.wallet_model.get_active_wallet_data().get_total()
            ));
            ui.add_space(50.0);

            ui.heading("Recipient Address");
            ui.text_edit_singleline(&mut self.string_scratchpad[0]);

            ui.add_space(50.0);

            ui.heading("Amount to send");
            ui.text_edit_singleline(&mut self.string_scratchpad[1]);
            ui.label("Sats");

            if ui.button("Send").clicked() {
                let (valid, invalid_vec) = self.is_valid_transaction_request(
                    &self.string_scratchpad[0],
                    &self.string_scratchpad[1],
                );

                if valid {
                    self.dialog_box = Some(DialogBox {
                        dialog_box_enum: DialogBoxEnum::ConfirmSend,
                        title: "Confirm Transaction",
                        dialog_line_edit: Vec::from([DialogLineEdit {
                            message: Some(
                                format!(
                                    "Are you sure you want to send {} Sats to {}?",
                                    &self.string_scratchpad[1], &self.string_scratchpad[0]
                                )
                                .into(),
                            ),
                            line_edit: None,
                        }]),

                        optional: true,
                    });
                } else {
                    let invalid_message = invalid_vec.join("\n");
                    self.dialog_box = Some(DialogBox {
                        dialog_box_enum: DialogBoxEnum::InvalidTransaction,
                        title: "Invalid Transaction",
                        dialog_line_edit: Vec::from([DialogLineEdit {
                            message: Some(invalid_message),
                            line_edit: None,
                        }]),
                        optional: false,
                    })
                }
            }
        });
    }

    pub fn render_receiving_panel(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        ui.add_space(20.0);
        let wallet = self.wallet_model.get_active_wallet_data();
        let pub_key = wallet.pub_key;
        ui.vertical_centered(|ui| {
            // Encode some data into bits.
            ui.heading("Public Key");
            ui.add_space(10.0);

            ui.heading(&pub_key);
            ui.add_space(10.0);

            let img = ui.ctx().load_texture(
                "my-image",
                generate_qrcode_from_address(&pub_key).unwrap(),
                Default::default(),
            );

            ui.add(egui::Image::from_texture(&img));

            ui.add_space(10.0);
            if ui.button("Copy Public Key").clicked() {
                ui.output_mut(|o| o.copied_text = pub_key);
            }
        });
    }

    pub fn render_contacts_panel(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        ui.add_space(20.0);

        ui.vertical_centered(|ui| {
            ui.heading("Contact List");
            ui.add_space(10.0);
            if ui.button("Add Contact").clicked() {
                self.change_state(CentralPanelState::ContactsNewContact);
            }
            ui.add_space(10.0);
            TableBuilder::new(ui)
                .column(Column::exact(250.0).resizable(false))
                .column(Column::exact(350.0))
                .column(Column::exact(100.0))
                .column(Column::exact(100.0))
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Wallet Name");
                    });
                    header.col(|ui| {
                        ui.heading("Public Key");
                    });
                    header.col(|ui| {
                        ui.heading("Last Transaction Txid");
                    });
                })
                .body(|mut body| {
                    let contacts = self.wallet_model.json_wallet_data.contacts.clone();
                    for contact in contacts.iter() {
                        body.row(30.0, |mut row| {
                            row.col(|ui| {
                                ui.horizontal(|ui| {
                                    if ui
                                        .button(contact.wallet_name.clone())
                                        .on_hover_text("Change")
                                        .clicked()
                                    {
                                        self.change_state(CentralPanelState::ContactsRename {
                                            pub_key: contact.pub_key.clone(),
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

                            row.col(|ui| {
                                let last_transaction =
                                    self.wallet_model.last_transaction(&contact.pub_key.clone());
                                let mut last_transaction_string = "None".to_string();
                                if let Some(last_transaction) = last_transaction {
                                    last_transaction_string = format!("{}", last_transaction.txid);
                                }
                                ui.label(last_transaction_string);
                            });
                        });
                    }
                });
        });
    }

    pub fn render_new_wallet_creation(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
        destination: CentralPanelState,
        mnemonic_string: &str,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("Write down the following mnemonic");
            ui.add_space(20.0);

            ui.strong(mnemonic_string);
            ui.add_space(30.0);
            if ui.button("Copy Mnemonic").clicked() {
                ui.output_mut(|o| o.copied_text = mnemonic_string.to_string());
            }
            ui.add_space(20.0);
            ui.label("Type and confirm the mnemonic above");
            ui.add_space(10.0);
            ui.text_edit_singleline(&mut self.string_scratchpad[0]);
            ui.add_space(10.0);
            ui.label("Wallet Name");
            ui.add_space(20.0);
            ui.text_edit_singleline(&mut self.string_scratchpad[1]);
            if ui.button("Confirm").clicked() {
                let priv_key = generate_xpriv(&mnemonic_string).unwrap().to_string();
                let copied_correctly = self.string_scratchpad[0] == mnemonic_string;
                let wallet_in_use = self.wallet_model.contains_wallet(&priv_key);
                let title = match (copied_correctly, wallet_in_use) {
                    (true, false) => {
                        self.wallet_model
                            .add_wallet(
                                &priv_key,
                                &self.string_scratchpad[0],
                                &self.string_scratchpad[1],
                            )
                            .unwrap();
                        self.change_state(destination);
                        "Wallet Created"
                    }
                    (true, true) => {
                        self.change_state(CentralPanelState::WalletNewWallet {
                            mnemonic_string: generate_mnemonic_string().unwrap(),
                        });
                        "Wallet Already In Use"
                    }
                    (false, _) => "Incorrectly Copied",
                };

                self.dialog_box = Some(DialogBox {
                    dialog_box_enum: DialogBoxEnum::IncorrectMnemonic,
                    title: title,
                    dialog_line_edit: Vec::from([DialogLineEdit {
                        message: None,
                        line_edit: None,
                    }]),
                    optional: false,
                })
            }
        });
    }

    pub fn render_new_wallet_existing(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
        destination: CentralPanelState,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("Type in the mnemonic for an existing wallet");
            ui.add_space(20.0);
            ui.text_edit_singleline(&mut self.string_scratchpad[0]);
            ui.add_space(50.0);
            ui.heading("Wallet Name");
            ui.add_space(20.0);
            ui.text_edit_singleline(&mut self.string_scratchpad[1]);

            if ui.button("Confirm").clicked() {
                let parse_result = generate_xpriv(&self.string_scratchpad[0]);

                let title = match &parse_result {
                    Ok(xprv) => {
                        let wallet_in_use =
                            self.wallet_model.wallets_contain_wallet(&xprv.to_string());
                        if !wallet_in_use {
                            self.wallet_model
                                .add_wallet(
                                    &xprv.to_string(),
                                    &self.string_scratchpad[0],
                                    &self.string_scratchpad[1],
                                )
                                .unwrap();
                            self.change_state(destination);
                            "Wallet Added"
                        } else {
                            "Wallet Already In Use"
                        }
                    }
                    Err(_) => "Mnemonic Incorrect",
                };

                self.dialog_box = Some(DialogBox {
                    dialog_box_enum: DialogBoxEnum::WalletCreated,
                    title: title,
                    dialog_line_edit: Vec::from([DialogLineEdit {
                        message: None,
                        line_edit: None,
                    }]),
                    optional: false,
                })
            }
        });
    }

    pub fn render_new_contact(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
        destination: CentralPanelState,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("Public Key");
            ui.add_space(20.0);
            ui.text_edit_singleline(&mut self.string_scratchpad[0]);
            ui.add_space(50.0);
            ui.heading("Wallet Name");
            ui.add_space(20.0);
            ui.text_edit_singleline(&mut self.string_scratchpad[1]);

            if ui.button("Confirm").clicked() {
                let valid_bitcoin_addr = is_valid_bitcoin_address(&self.string_scratchpad[0]);
                let mut title = "Wallet Created";
                let wallet_in_use = self
                    .wallet_model
                    .contains_wallet(&self.string_scratchpad[0]);
                let title = match (valid_bitcoin_addr, wallet_in_use) {
                    (true, false) => {
                        self.wallet_model
                            .add_contact(&self.string_scratchpad[0], &self.string_scratchpad[1]);
                        self.change_state(destination);
                        "Wallet Created"
                    }
                    (true, true) => "Wallet Already In Use",
                    (false, _) => "Invalid Bitcoin Address",
                };
                self.dialog_box = Some(DialogBox {
                    dialog_box_enum: DialogBoxEnum::WalletCreated,
                    title: title,
                    dialog_line_edit: Vec::from([DialogLineEdit {
                        message: None,
                        line_edit: None,
                    }]),
                    optional: false,
                });
            }
        });
    }

    pub fn render_create_password_panel(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
        destination: CentralPanelState,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label("Type New Password");
            let password_entry =
                egui::TextEdit::singleline(&mut self.string_scratchpad[0]).password(true);

            ui.add(password_entry);
            ui.add_space(20.0);
            ui.label("Confirm Password");
            let password_entry_confirmation =
                egui::TextEdit::singleline(&mut self.string_scratchpad[1]).password(true);
            ui.add(password_entry_confirmation);
            let mut password_strength = 0;

            if let Ok(password_strength_estimate) = zxcvbn(&self.string_scratchpad[0], &[]) {
                password_strength = password_strength_estimate.score();
            }

            if self.string_scratchpad[0] != self.string_scratchpad[1] {
                ui.add_space(30.0);
                ui.label("Passwords not the same");
            } else {
                let password_strength_bar =
                    egui::widgets::ProgressBar::new(password_strength as f32 / 4.0);
                ui.add_space(30.0);
                ui.add_sized([285.0, 0.0], password_strength_bar);
                let password_strength_string: &str;
                match password_strength {
                    0 => password_strength_string = "Very Weak",
                    1 => password_strength_string = "Weak",
                    2 => password_strength_string = "So-so",
                    3 => password_strength_string = "Good",
                    4 => password_strength_string = "Great",
                    _ => password_strength_string = "NP-Badboy",
                }
                ui.add_space(10.0);
                ui.label(password_strength_string);
                ui.add_space(30.0);
                if ui.button("Enter").clicked() {
                    self.wallet_model
                        .create_passworded_file(self.string_scratchpad[0].clone())
                        .unwrap();

                    self.change_state(destination);
                }
            };
        });
    }

    pub fn render_enter_password_panel(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
        destination: CentralPanelState,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            let password_entry =
                egui::TextEdit::singleline(&mut self.string_scratchpad[0]).password(true);
            ui.heading("Enter Password");
            ui.add_space(20.0);
            ui.add(password_entry);
            ui.add_space(20.0);

            if ui.button("Enter").clicked() {
                if self
                    .wallet_model
                    .validate_password(&self.string_scratchpad[0])
                {
                    self.initialise_last_interaction_time();
                    self.change_state(destination);
                } else {
                    self.clear_string_scratchpad();
                }
            }
        });
    }

    pub fn render_settings_panel(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        if ui.button("Change Password").clicked() {
            self.change_state(CentralPanelState::SettingsChangePassword);
        }
    }

    pub fn render_delete_wallet_panel(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
        destination: CentralPanelState,
        pub_key: String,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        let (entry_type, wallet) = self.wallet_model.get_wallet_data(&pub_key);
        let wallet_name = wallet.wallet_name.clone();
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);

            ui.heading(format!(
                "Are you sure you want to delete wallet {}?",
                wallet_name
            ));
            ui.strong(&pub_key);
            if ui.button("Confirm").clicked() {
                match entry_type {
                    EntryType::Wallet => {
                        self.wallet_model.delete_wallet(&pub_key);
                    }
                    EntryType::Contact => {
                        self.wallet_model.delete_contact(&pub_key);
                    }
                }
                self.change_state(destination);
            }
        });
    }
    pub fn render_rename_wallet_panel(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
        destination: CentralPanelState,
        pub_key: String,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        ui.vertical_centered(|ui| {
            let (entry_type, wallet) = self.wallet_model.get_wallet_data(&pub_key);

            ui.add_space(50.0);
            ui.heading("Public Key");
            ui.add_space(20.0);

            ui.label(wallet.pub_key.clone());

            ui.add_space(20.0);
            ui.heading("Current Wallet Name");

            ui.add_space(10.0);
            ui.label(wallet.wallet_name.clone());
            ui.add_space(20.0);
            ui.heading("New Wallet Name");
            ui.text_edit_singleline(&mut self.string_scratchpad[0]);

            if ui.button("Confirm").clicked() {
                self.wallet_model
                    .rename_wallet(entry_type, &pub_key, &self.string_scratchpad[0]);
                self.change_state(destination);
            }
        });
    }

    pub fn render_wallet_secret_panel(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: Option<CentralPanelState>,
    ) {
        self.boiler_plate_render(ui, watch, &source);
        ui.vertical_centered(|ui| {
            let wallet = self.wallet_model.get_active_wallet_data();
            let mnemonic_string = wallet.mnemonic.unwrap();
            let priv_key = wallet.priv_key.unwrap();
            ui.add_space(50.0);
            ui.heading("Mnemonic");
            ui.add_space(20.0);

            ui.strong(&mnemonic_string);
            if ui.button("Copy Mnemonic").clicked() {
                ui.output_mut(|o| o.copied_text = mnemonic_string);
            }
            ui.add_space(20.0);
            ui.heading("Private Key");
            ui.add_space(20.0);
            ui.strong(&priv_key);
            if ui.button("Copy Private Key").clicked() {
                ui.output_mut(|o| o.copied_text = priv_key);
            }
        });
    }

    pub fn render_centrepanel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| match &self.central_panel_state {
            CentralPanelState::WalletFileNotAvailable => self.render_create_password_panel(
                ui,
                false,
                None,
                CentralPanelState::WalletNotInitialised,
            ),
            CentralPanelState::NoWalletsInWalletFile { mnemonic_string } => self
                .render_new_wallet_creation(
                    ui,
                    false,
                    None,
                    CentralPanelState::WalletNotInitialised,
                    &mnemonic_string.clone(),
                ),
            CentralPanelState::WalletNotInitialised => self.wallet_initialisation(),
            CentralPanelState::PasswordNeeded { destination } => {
                self.render_enter_password_panel(ui, false, None, *destination.clone())
            }
            CentralPanelState::WalletMain => self.render_wallet_main_panel(ui, true, None),
            CentralPanelState::SendingMain => self.render_sending_panel(ui, true, None),
            CentralPanelState::ReceivingMain => self.render_receiving_panel(ui, true, None),
            CentralPanelState::ContactsMain => self.render_contacts_panel(ui, true, None),
            CentralPanelState::SettingsMain => self.render_settings_panel(ui, true, None),
            CentralPanelState::WalletDelete => self.render_delete_wallet_panel(
                ui,
                true,
                Some(CentralPanelState::WalletMain),
                CentralPanelState::WalletMain,
                self.wallet_model.get_active_wallet_pub_key(),
            ),
            CentralPanelState::WalletRename => self.render_rename_wallet_panel(
                ui,
                true,
                Some(CentralPanelState::WalletMain),
                CentralPanelState::WalletMain,
                self.wallet_model.get_active_wallet_data().pub_key,
            ),

            CentralPanelState::WalletSecret => {
                self.render_wallet_secret_panel(ui, true, Some(CentralPanelState::WalletMain))
            }
            CentralPanelState::SettingsChangePassword => self.render_create_password_panel(
                ui,
                true,
                Some(CentralPanelState::SettingsMain),
                CentralPanelState::SettingsChangePassword,
            ),

            CentralPanelState::WalletNewWallet { mnemonic_string } => self
                .render_new_wallet_creation(
                    ui,
                    true,
                    Some(CentralPanelState::WalletMain),
                    CentralPanelState::WalletMain,
                    &mnemonic_string.clone(),
                ),
            CentralPanelState::WalletExistingWallet => self.render_new_wallet_existing(
                ui,
                true,
                Some(CentralPanelState::WalletMain),
                CentralPanelState::WalletMain,
            ),
            CentralPanelState::ContactsNewContact => self.render_new_contact(
                ui,
                true,
                Some(CentralPanelState::ContactsMain),
                CentralPanelState::ContactsMain,
            ),

            CentralPanelState::ContactsRename { pub_key } => self.render_rename_wallet_panel(
                ui,
                true,
                Some(CentralPanelState::ContactsMain),
                CentralPanelState::ContactsMain,
                pub_key.to_string(),
            ),

            CentralPanelState::ContactsDelete { pub_key } => self.render_delete_wallet_panel(
                ui,
                true,
                Some(CentralPanelState::ContactsMain),
                CentralPanelState::ContactsMain,
                self.wallet_model.get_active_wallet_pub_key(),
            ),
        });
    }

    pub fn wallet_initialisation(&mut self) {
        if !self.wallet_model.does_file_exist() {
            self.central_panel_state = CentralPanelState::WalletFileNotAvailable;
            return;
        }

        if self.wallet_model.key.is_none() {
            self.central_panel_state = CentralPanelState::PasswordNeeded {
                destination: Box::new(CentralPanelState::WalletNotInitialised),
            };
            return;
        } else {
            self.wallet_model.initialise_from_wallet_file().unwrap();
            if self.wallet_model.json_wallet_data.wallets.is_empty() {
                self.central_panel_state = CentralPanelState::NoWalletsInWalletFile {
                    mnemonic_string: generate_mnemonic_string().unwrap(),
                };
                return;
            } else {
                self.initialise_last_interaction_time();
                self.central_panel_state = CentralPanelState::WalletMain;
                return;
            }
        }
    }

    pub fn clear_string_scratchpad(&mut self) {
        self.string_scratchpad = [String::new(), String::new(), String::new()];
    }
    pub fn initialise_last_interaction_time(&mut self) {
        self.last_interaction_time = chrono::offset::Local::now();
    }

    pub fn change_state(&mut self, state: CentralPanelState) {
        self.clear_string_scratchpad();
        self.central_panel_state = state;
    }

    pub fn boiler_plate_render(
        &mut self,
        ui: &mut Ui,
        watch: bool,
        source: &Option<CentralPanelState>,
    ) {
        if let Some(_) = self.dialog_box {
            ui.set_enabled(false);
        }
        if watch {
            self.password_needed_watchdog_timer();
        }
        if let Some(source) = source.clone() {
            if ui.button("Back").clicked() {
                self.change_state(source);
            }
        }
    }
}
