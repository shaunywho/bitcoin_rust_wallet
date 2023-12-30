use crate::bitcoin_wallet::{bitcoin_test, generate_mnemonic_string, is_valid_bitcoin_address};

mod app_centrepanel;
mod app_sidepanel;
mod app_toppanel;

use crate::wallet_file_manager::{encryption_test, EntryType, SyncData, WalletModel};

use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;

const FILENAME: &str = "./wallet.txt";
const PASSWORD_NEEDED_TIMEOUT_S: i64 = 300;
use chrono::{DateTime, Duration};

use egui::InnerResponse;
use std::num::ParseIntError;
#[derive(PartialEq, Clone)]
pub enum CentralPanelState {
    WalletFileNotAvailable,
    NoWalletsInWalletFile { mnemonic_string: String },
    WalletNotInitialised,
    PasswordNeeded { destination: Box<CentralPanelState> },
    WalletMain,
    SendingMain,
    ReceivingMain,
    ContactsMain,
    SettingsMain,
    WalletDelete,
    WalletRename,
    WalletSecret,
    WalletNewWallet { mnemonic_string: String },
    WalletExistingWallet,
    SettingsChangePassword,
    ContactsNewContact,
    ContactsRename { pub_key: String },
    ContactsDelete { pub_key: String },
}

#[derive(PartialEq)]
pub enum SidePanel {
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
    pub dialog_line_edit: Vec<DialogLineEdit>,
    pub optional: bool,
}
#[derive(Clone)]
pub struct DialogLineEdit {
    pub message: Option<String>,
    pub line_edit: Option<String>,
}
#[derive(Clone)]
pub enum DialogBoxEnum {
    IncorrectMnemonic,
    WalletCreated,
    ConfirmSend,
    InvalidTransaction,
    ChangeContactName { pub_key: String },
}

pub struct MyApp {
    central_panel_state: CentralPanelState,
    side_panel_active: SidePanel,
    wallet_model: WalletModel,
    sync_data_receiver: mpsc::Receiver<SyncData>,
    sync_data_sender: mpsc::Sender<SyncData>,
    active_threads: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    string_scratchpad: [String; 3],
    dialog_box: Option<DialogBox>,
    last_interaction_time: DateTime<chrono::Local>,
}

impl MyApp {
    fn accept_process(&mut self, edited_lines: Vec<String>) {
        let Some(dialog_box) = &self.dialog_box else {
            return;
        };
        match &dialog_box.dialog_box_enum {
            DialogBoxEnum::ChangeContactName { pub_key } => {
                let wallet_name = &edited_lines[0];
                let _ = self
                    .wallet_model
                    .rename_wallet(EntryType::Contact, pub_key, &wallet_name);
            }

            DialogBoxEnum::ConfirmSend { .. } => {
                let recipient_addr = self.string_scratchpad[0].clone();
                let amount = (&self.string_scratchpad[1]).parse().unwrap();
                self.wallet_model.send_transaction(&recipient_addr, amount);
                self.clear_string_scratchpad();
            }
            _ => {}
        }
        self.dialog_box = None;
    }

    fn render_dialog_box(&mut self, ctx: &egui::Context) -> InnerResponse<Option<()>> {
        let response = egui::Window::new(self.dialog_box.as_ref().unwrap().title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let mut edited_lines = Vec::new();
                for dialog_line_edit in &mut self.dialog_box.as_mut().unwrap().dialog_line_edit {
                    if let Some(message) = &dialog_line_edit.message {
                        ui.vertical_centered(|ui| {
                            ui.label(message);
                        });
                    }

                    if let Some(mut line_edit) = dialog_line_edit.line_edit.as_mut() {
                        ui.vertical_centered(|ui| {
                            ui.text_edit_singleline(line_edit);
                            edited_lines.push(line_edit.to_string());
                        });
                    }
                }
                ui.vertical_centered(|ui| {
                    if self.dialog_box.as_ref().unwrap().optional {
                        if ui.button("Cancel").clicked() {
                            self.dialog_box = None;
                        }
                    }

                    if ui.button("Accept").clicked() {
                        self.accept_process(edited_lines);
                    }
                });
            });
        return response.unwrap();
    }
}

impl MyApp {
    pub fn new() -> Self {
        let central_panel_state = CentralPanelState::WalletNotInitialised;
        let side_panel_active = SidePanel::Wallet;
        let wallet_model = WalletModel::new(FILENAME);
        let (sync_data_sender, sync_data_receiver) = mpsc::channel();

        let recipient_address_string = String::new();
        let amount_to_send_string = String::new();
        let dialog_box = None;
        let active_threads = Arc::new(Mutex::new(HashMap::new()));
        let last_interaction_time = chrono::offset::Local::now();
        let string_scratchpad = [String::new(), String::new(), String::new()];

        let slf = Self {
            central_panel_state: central_panel_state,
            side_panel_active: side_panel_active,
            wallet_model: wallet_model,
            sync_data_sender: sync_data_sender,
            sync_data_receiver: sync_data_receiver,
            active_threads: active_threads,
            dialog_box: dialog_box,
            last_interaction_time: last_interaction_time,
            string_scratchpad: string_scratchpad,
        };

        slf
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        encryption_test();

        if let Some(_pub_key) = &self.wallet_model.active_wallet {
            self.wallet_poll();
        }

        self.render_window(ctx, _frame);
    }
}

impl MyApp {
    fn is_valid_transaction_request(
        &self,
        recipient_address_string: &str,
        amount_to_send_string: &str,
    ) -> (bool, Vec<String>) {
        let mut valid = true;
        let mut invalid_transaction_vec = Vec::new();
        if !is_valid_bitcoin_address(recipient_address_string) {
            valid = false;
            invalid_transaction_vec.push("Invalid Bitcoin Address".to_string());
        }

        if self.is_own_address(recipient_address_string) {
            valid = false;
            invalid_transaction_vec.push("Can't send to own address".to_string());
        }

        let result: Result<u64, ParseIntError> = amount_to_send_string.parse();
        match result {
            Ok(amount) => {
                let total = self.wallet_model.get_active_wallet_data().get_total();
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

    fn is_own_address(&self, recipient_address_string: &str) -> bool {
        let address = self.wallet_model.get_active_wallet_pub_key();

        return recipient_address_string == address;
    }

    fn wallet_poll(&mut self) {
        let active_wallet_pub_key = self.wallet_model.get_active_wallet_pub_key();

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
                &sync_data.pub_key,
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
            .contains_key(&active_wallet_pub_key)
        {
            return;
        }
        let handle = self
            .wallet_model
            .sync_current_wallet(sync_data_channel_clone);
        self.active_threads
            .lock()
            .unwrap()
            .insert(active_wallet_pub_key, handle);
    }
}

impl MyApp {
    fn render_window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(_) = self.dialog_box {
            self.render_dialog_box(ctx);
        }
        self.render_sidepanel(ctx, _frame);
        self.render_toppanel(ctx, _frame);
        self.render_centrepanel(ctx, _frame);
    }

    fn password_needed_watchdog_timer(&mut self) {
        let current_time = chrono::offset::Local::now();
        if (current_time - self.last_interaction_time)
            > Duration::seconds(PASSWORD_NEEDED_TIMEOUT_S)
        {
            self.central_panel_state = CentralPanelState::PasswordNeeded {
                destination: Box::new(self.central_panel_state.clone()),
            };
        }
        self.last_interaction_time = current_time;
    }
}
