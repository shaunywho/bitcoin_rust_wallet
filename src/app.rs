use crate::bitcoin_wallet::{
    bitcoin_test, generate_mnemonic_string, generate_xpriv, is_valid_bitcoin_address,
    make_transaction,
};

use crate::wallet_file_manager::{SyncData, WalletData, WalletElement};
use bdk::{Balance, TransactionDetails};
use egui::Ui;

use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;

const FILENAME: &str = "./wallet.txt";

use qrcode_generator::QrCodeEcc;
use std::num::ParseIntError;
#[derive(PartialEq)]
enum WalletFileState {
    WalletFileNotAvailable,
    WalletNotInitialised,
    WalletAvailable,
}

enum InvalidTransactionTypes {
    InvalidBitcoinAddress,
    InvalidAmountNotNumeric,
    InvalidAmountNotEnough,
    InvalidOwnAddress,
}

#[derive(PartialEq)]
enum SidePanelState {
    Wallet,
    Sending,
    Receiving,
    Contacts,
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
    NewMnemonic,
    ChangeWalletName,
    ConfirmSend,
    InvalidTransaction,
}

struct WalletApp {
    state: WalletFileState,
    wallet_data: Option<WalletData>,
}

pub struct MyApp {
    state: WalletFileState,
    side_panel_state: SidePanelState,
    wallet_data: WalletData,
    selected_wallet: Option<String>,
    sync_data_receiver: mpsc::Receiver<SyncData>,
    sync_data_sender: mpsc::Sender<SyncData>,
    active_threads: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    rename_wallet_string: String,
    recipient_address_string: String,
    amount_to_send_string: String,
    dialog_box: Option<DialogBox>,
}

impl MyApp {
    fn accept_process(&mut self, line_edit: Option<String>) {
        if let Some(dialog_box) = &self.dialog_box {
            match dialog_box.dialog_box_enum {
                DialogBoxEnum::NewMnemonic => {
                    let xkey = generate_xpriv(&dialog_box.message.clone().unwrap()).unwrap();
                    let _ = self.wallet_data.add_wallet(xkey);
                }
                DialogBoxEnum::ChangeWalletName => {
                    self.rename_wallet_string = line_edit.unwrap();

                    self.wallet_data
                        .wallets
                        .get_mut(&self.wallet_data.get_selected_walletx_string())
                        .unwrap()
                        .wallet_name = self.rename_wallet_string.clone();
                    self.wallet_data.rename_wallet(
                        &self.wallet_data.get_selected_walletx_string(),
                        &self.rename_wallet_string,
                    );
                }
                DialogBoxEnum::ConfirmSend { .. } => {
                    let recipient_addr = self.recipient_address_string.clone();
                    let amount = (&self.amount_to_send_string).parse().unwrap();
                    let wallet = self.get_selected_wallet_element();
                    wallet.send_transaction(&recipient_addr, amount);
                }
                DialogBoxEnum::InvalidTransaction { .. } => {}
            }
            self.dialog_box = None;
        }
    }

    fn render_dialog_box(&mut self, ctx: &egui::Context) {
        egui::Window::new(self.dialog_box.as_ref().unwrap().title)
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
    }
}

impl MyApp {
    pub fn new() -> Self {
        let mut state = WalletFileState::WalletFileNotAvailable;
        let side_panel_state = SidePanelState::Wallet;
        let mut wallet_data = WalletData::new(FILENAME);
        let mut selected_wallet = Option::None;
        let (sync_data_sender, sync_data_receiver) = mpsc::channel();
        let rename_wallet_string = String::new();
        let recipient_address_string = String::new();
        let amount_to_send_string = String::new();
        let dialog_box = None;
        let active_threads = Arc::new(Mutex::new(HashMap::new()));

        let slf = Self {
            state,
            side_panel_state,
            wallet_data,
            selected_wallet,
            sync_data_sender,
            sync_data_receiver,
            active_threads,

            rename_wallet_string,
            recipient_address_string,
            amount_to_send_string,
            dialog_box,
        };

        slf
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_wallet_state();
        if let Some(_private_key) = &mut self.selected_wallet {
            self.wallet_poll();
            // self.update_from_wallet_sync();
        }

        self.render_window(ctx, _frame);
        // bitcoin_test();
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
                let total = self.get_selected_wallet_element().get_total();
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
        let address = self.get_selected_wallet_element().address.clone();
        return self.recipient_address_string == address;
    }

    fn wallet_poll(&mut self) {
        let selected_wallet_priv_key = self.wallet_data.get_selected_walletx_string();
        println!("Length = {}", self.active_threads.lock().unwrap().len());
        let sync_data_channel_clone = self.sync_data_sender.clone();

        while let Ok(sync_data) = self.sync_data_receiver.try_recv() {
            let wallet = self.get_selected_wallet_element();
            wallet.balance = Some(sync_data.balance);
            wallet.transactions = Some(sync_data.transactions);
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
        let wallet_element = self.get_selected_wallet_element();
        let handle = wallet_element.start_wallet_syncing_worker(sync_data_channel_clone);
        self.active_threads
            .lock()
            .unwrap()
            .insert(selected_wallet_priv_key, handle);
    }
}

impl MyApp {
    fn get_selected_wallet_element(&mut self) -> &mut WalletElement {
        self.wallet_data
            .get_wallet_element(&self.selected_wallet.as_ref().unwrap())
    }
    fn update_wallet_state(&mut self) {
        match self.state {
            WalletFileState::WalletNotInitialised => {
                self.wallet_data.initialise_from_wallet_file();
                if !self.wallet_data.wallets.is_empty() {
                    self.state = WalletFileState::WalletAvailable;
                    let selected_wallet_xpriv_str = self.wallet_data.get_first_wallet_xpriv_str();
                    let selected_wallet_element = self
                        .wallet_data
                        .get_wallet_element(&selected_wallet_xpriv_str);
                    self.selected_wallet = Option::Some(selected_wallet_xpriv_str.clone());
                }
                self.state = WalletFileState::WalletAvailable;
            }
            WalletFileState::WalletFileNotAvailable => {
                if self.wallet_data.does_file_exist() {
                    self.state = WalletFileState::WalletNotInitialised;
                }
            }
            _ => (),
        }
    }

    fn render_window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_sidepanel(ctx, _frame);
        self.render_toppanel(ctx, _frame);
        self.render_centrepanel(ctx, _frame);

        if let Some(_) = self.dialog_box {
            self.render_dialog_box(ctx);
        }
    }
    pub fn render_sidepanel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
            match self.dialog_box {
                None => (),
                _ => ui.set_enabled(false),
            }

            let side_panel_state = SidePanelState::Wallet;
            ui.add_space(10.0);
            if ui
                .add_sized(
                    [150.0, 150.0],
                    egui::ImageButton::new(egui::include_image!("../assets/wallet.png"))
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
                    [150.0, 150.0],
                    egui::ImageButton::new(egui::include_image!("../assets/send.png"))
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
                    [150.0, 150.0],
                    egui::ImageButton::new(egui::include_image!("../assets/receive.png"))
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
                    [150.0, 150.0],
                    egui::ImageButton::new(egui::include_image!("../assets/contacts.png"))
                        .rounding(5.0)
                        .selected(self.side_panel_state == side_panel_state),
                )
                .clicked()
            {
                self.side_panel_state = side_panel_state;
            };
        });
    }
    pub fn render_wallet_panel(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            let wallet = self.get_selected_wallet_element();
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
        let wallet = self.get_selected_wallet_element();
        ui.heading(format!("Wallet Balance: {:?}", wallet.get_total()));
        ui.add_space(50.0);
        let public_key = &wallet.address;

        ui.label(format!("Public Key: {:?}", &public_key));
        if ui.button("Copy").clicked() {
            ui.output_mut(|o| o.copied_text = public_key.to_string());
        };
    }
    pub fn render_sending_panel(&mut self, ui: &mut Ui) {
        ui.heading(format!(
            "Wallet Balance: {:?}",
            self.get_selected_wallet_element().get_total()
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

    pub fn render_receiving_panel(&mut self, ui: &mut Ui) {
        let wallet = self.get_selected_wallet_element();
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

    pub fn render_contacts_panel(&mut self, ui: &mut Ui) {}

    pub fn render_centrepanel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.dialog_box.is_some() {
                ui.set_enabled(false)
            }

            match self.state {
                WalletFileState::WalletFileNotAvailable => {
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
                WalletFileState::WalletNotInitialised => {}
                _ => match self.side_panel_state {
                    SidePanelState::Wallet => self.render_wallet_panel(ui),
                    SidePanelState::Sending => self.render_sending_panel(ui),
                    SidePanelState::Receiving => self.render_receiving_panel(ui),
                    SidePanelState::Contacts => self.render_contacts_panel(ui),
                },
            }
        });
    }

    pub fn render_toppanel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("Headerbar")
            .exact_height(50.0)
            .show(ctx, |ui| {
                if self.dialog_box.is_some() {
                    ui.set_enabled(false);
                }
                ui.heading("Rust Bitcoin Wallet");
            });
    }
}

pub fn generate_qrcode_from_address(address: &str) -> Result<egui::ColorImage, image::ImageError> {
    let result = qrcode_generator::to_png_to_vec(address, QrCodeEcc::Medium, 100).unwrap();
    let dynamic_image = image::load_from_memory(&result).unwrap();
    let size = [dynamic_image.width() as _, dynamic_image.height() as _];
    let image_buffer = dynamic_image.to_luma8();
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_gray(size, pixels.as_slice()))
}
