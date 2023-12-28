use crate::bitcoin_wallet::{bitcoin_test, generate_mnemonic_string, is_valid_bitcoin_address};

mod app_centrepanel;
mod app_sidepanel;
mod app_toppanel;

use crate::wallet_file_manager::{encryption_test, EntryType, SyncData, WalletModel};

use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;

const FILENAME: &str = "./wallet.txt";
const PASSWORD_NEEDED_TIMEOUT_S: i64 = 60;
use chrono::{DateTime, Duration};

use egui::InnerResponse;
use std::num::ParseIntError;
#[derive(PartialEq)]
enum CentralPanelState {
    WalletFileNotAvailable,
    NoWalletsInWalletFile {
        mnemonic_string: String,
    },
    WalletNotInitialised,
    PasswordNeeded,
    // PasswordEntered,
    WalletAvailable {
        last_interaction_time: DateTime<chrono::Local>,
    },
}

#[derive(PartialEq)]
enum SidePanelState {
    Wallet,
    Sending,
    Receiving,
    Contacts,
    Settings,
}
#[derive(Clone)]
pub struct DialogBox {
    pub dialog_box_enum: DialogBoxEnum,
    pub title: &'static str,
    pub message: Option<String>,
    pub line_edit: Option<String>,
    pub optional: bool,
}
#[derive(Clone)]
pub enum DialogBoxEnum {
    IncorrectMnemonic,
    WalletCreated,
    ChangeWalletName,
    ConfirmSend,
    InvalidTransaction,
    ChangeContactName { pub_key: String },
}

pub struct MyApp {
    central_panel_state: CentralPanelState,
    side_panel_state: SidePanelState,
    wallet_model: WalletModel,
    sync_data_receiver: mpsc::Receiver<SyncData>,
    sync_data_sender: mpsc::Sender<SyncData>,
    active_threads: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    rename_wallet_string: String,
    recipient_address_string: String,
    amount_to_send_string: String,
    password_entry_string: String,
    password_entry_confirmation_string: String,
    dialog_box: Option<DialogBox>,
    confirm_mnemonic_string: String,
}

impl MyApp {
    fn accept_process(&mut self, line_edit: Option<String>) {
        let Some(dialog_box) = &self.dialog_box else {
            return;
        };
        match &dialog_box.dialog_box_enum {
            DialogBoxEnum::WalletCreated => {}
            DialogBoxEnum::IncorrectMnemonic => {}
            DialogBoxEnum::ChangeContactName { pub_key } => {
                let wallet_name = line_edit.unwrap();
                let _ = self
                    .wallet_model
                    .rename_wallet(EntryType::Contact, pub_key, &wallet_name);
            }

            DialogBoxEnum::ChangeWalletName => {
                let new_wallet_name = line_edit.unwrap();

                let selected_priv_key = self.wallet_model.get_selected_wallet_string();
                let _ = self.wallet_model.rename_wallet(
                    EntryType::Wallet,
                    &selected_priv_key,
                    &new_wallet_name,
                );
            }
            DialogBoxEnum::ConfirmSend { .. } => {
                let recipient_addr = self.recipient_address_string.clone();
                let amount = (&self.amount_to_send_string).parse().unwrap();
                self.wallet_model.send_transaction(&recipient_addr, amount);
            }
            DialogBoxEnum::InvalidTransaction { .. } => {}
        }
        self.dialog_box = None;
    }

    fn render_dialog_box(&mut self, ctx: &egui::Context) -> InnerResponse<Option<()>> {
        let response = egui::Window::new(self.dialog_box.as_ref().unwrap().title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if let Some(message) = self.dialog_box.as_ref().unwrap().clone().message {
                    ui.vertical_centered(|ui| {
                        ui.label(message);
                    });
                }
                let mut edited_line: Option<String> = None;
                if let Some(line_edit) = &mut self.dialog_box.as_mut().unwrap().line_edit {
                    ui.vertical_centered(|ui| {
                        ui.text_edit_singleline(line_edit);
                        edited_line = Some(line_edit.to_string());
                    });
                }
                ui.vertical_centered(|ui| {
                    if self.dialog_box.as_ref().unwrap().optional {
                        if ui.button("Cancel").clicked() {
                            self.dialog_box = None;
                        }
                    }

                    if ui.button("Accept").clicked() {
                        self.accept_process(edited_line);
                    }
                });
            });
        return response.unwrap();
    }
}

impl MyApp {
    pub fn new() -> Self {
        let central_panel_state = CentralPanelState::WalletFileNotAvailable;
        let side_panel_state = SidePanelState::Wallet;
        let wallet_model = WalletModel::new(FILENAME);
        let (sync_data_sender, sync_data_receiver) = mpsc::channel();
        let rename_wallet_string = String::new();
        let recipient_address_string = String::new();
        let amount_to_send_string = String::new();
        let password_entry_string = String::new();
        let password_entry_confirmation_string = String::new();
        let dialog_box = None;
        let active_threads = Arc::new(Mutex::new(HashMap::new()));
        let confirm_mnemonic_string = String::new();

        let slf = Self {
            central_panel_state,
            side_panel_state,
            wallet_model,
            sync_data_sender,
            sync_data_receiver,
            active_threads,
            rename_wallet_string,
            recipient_address_string,
            amount_to_send_string,
            password_entry_string,
            password_entry_confirmation_string,
            dialog_box,

            confirm_mnemonic_string,
        };

        slf
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        encryption_test();
        self.update_wallet_state();

        if let Some(_private_key) = &self.wallet_model.selected_wallet {
            self.wallet_poll();
        }

        self.render_window(ctx, _frame);
    }
}

impl MyApp {
    fn is_valid_transaction_request(&mut self) -> (bool, Vec<String>) {
        let mut valid = true;
        let mut invalid_transaction_vec = Vec::new();
        if !is_valid_bitcoin_address(&self.recipient_address_string) {
            valid = false;
            invalid_transaction_vec.push("Invalid Bitcoin Address".to_string());
        }

        if self.is_own_address() {
            valid = false;
            invalid_transaction_vec.push("Can't send to own address".to_string());
        }

        let result: Result<u64, ParseIntError> = self.amount_to_send_string.parse();
        match result {
            Ok(amount) => {
                let total = self.wallet_model.get_selected_wallet_data().get_total();
                if amount > total {
                    valid = false;
                    invalid_transaction_vec
                        .push("Insufficient funds in wallet for requested transaction".to_string())
                }
            }
            Err(_) => {
                valid = false;
                invalid_transaction_vec.push("Amount needs to be a number".to_string());
            }
        }

        return (valid, invalid_transaction_vec);
    }

    fn is_own_address(&mut self) -> bool {
        let address = self.wallet_model.get_selected_wallet_string();

        return self.recipient_address_string == address;
    }

    fn wallet_poll(&mut self) {
        let selected_wallet_priv_key = self.wallet_model.get_selected_wallet_string();

        let sync_data_channel_clone = self.sync_data_sender.clone();

        while let Ok(mut sync_data) = self.sync_data_receiver.try_recv() {
            sync_data.transactions.sort_by(|a, b| {
                match (&a.confirmation_time, &b.confirmation_time) {
                    (Some(a), Some(b)) => b.cmp(&a),

                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,

                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
            let _ = self.wallet_model.sync_wallet(
                &sync_data.priv_key,
                Some(sync_data.balance),
                Some(sync_data.transactions),
            );
        }
        self.active_threads
            .lock()
            .unwrap()
            .retain(|_, handle| !handle.is_finished());

        if self
            .active_threads
            .lock()
            .unwrap()
            .contains_key(&selected_wallet_priv_key)
        {
            return;
        }
        let handle = self
            .wallet_model
            .sync_current_wallet(sync_data_channel_clone);
        self.active_threads
            .lock()
            .unwrap()
            .insert(selected_wallet_priv_key, handle);
    }
}

impl MyApp {
    fn update_wallet_state(&mut self) {
        let new_state = match &self.central_panel_state {
            CentralPanelState::WalletNotInitialised => {
                if self.wallet_model.key.is_none() {
                    Some(CentralPanelState::PasswordNeeded)
                } else {
                    self.wallet_model.initialise_from_wallet_file().unwrap();
                    if self.wallet_model.json_wallet_data.wallets.is_empty() {
                        Some(CentralPanelState::NoWalletsInWalletFile {
                            mnemonic_string: generate_mnemonic_string().unwrap(),
                        })
                    } else {
                        Some(CentralPanelState::WalletAvailable {
                            last_interaction_time: chrono::offset::Local::now(),
                        })
                    }
                }
            }
            CentralPanelState::WalletFileNotAvailable => {
                if self.wallet_model.does_file_exist() {
                    Some(CentralPanelState::WalletNotInitialised)
                } else {
                    None
                }
            }
            CentralPanelState::WalletAvailable {
                last_interaction_time,
            } => {
                let current_time = chrono::offset::Local::now();
                if (current_time - last_interaction_time)
                    > Duration::seconds(PASSWORD_NEEDED_TIMEOUT_S)
                {
                    Some(CentralPanelState::PasswordNeeded)
                } else {
                    Some(CentralPanelState::WalletAvailable {
                        last_interaction_time: current_time,
                    })
                }
            }
            _ => None,
        };

        if let Some(state) = new_state {
            self.central_panel_state = state;
        }
    }

    fn render_window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let enabled: bool;

        match (&self.dialog_box, &self.central_panel_state) {
            (
                Some(_),
                CentralPanelState::WalletAvailable {
                    last_interaction_time: _,
                },
            ) => {
                self.render_dialog_box(ctx);
                enabled = false;
            }
            (
                None,
                CentralPanelState::WalletAvailable {
                    last_interaction_time: _,
                },
            ) => {
                enabled = true;
            }
            (_, _) => {
                self.dialog_box = None;
                enabled = false;
            }
        }

        self.render_sidepanel(enabled, ctx, _frame);
        self.render_toppanel(enabled, ctx, _frame);
        self.render_centrepanel(enabled, ctx, _frame);
    }
}
