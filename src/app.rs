use crate::bitcoin_wallet::{
    bitcoin_test, generate_mnemonic_string, generate_wallet, generate_xpriv, get_balance,
    get_transactions,
};
use bdk::blockchain::ElectrumBlockchain;

use bdk::electrum_client::Client;

use bdk::{Balance, TransactionDetails};

use crate::wallet_file_manager::{SyncData, WalletData, WalletElement};
use egui::Ui;

use std::sync::mpsc;

use std::time::Instant;
const FILENAME: &str = "./wallet.txt";

use qrcode_generator::QrCodeEcc;

#[derive(PartialEq)]
enum AppState {
    WalletNotAvailable,
    WalletAvailable,
    DialogBox,
    TransactionSending,
    WalletSyncing,
    NetworkError,
}

#[derive(PartialEq)]
enum SidePanelState {
    Wallet,
    Sending,
    Receiving,
    Contacts,
}

enum DialogBoxEnum {
    NewMnemonic,
    ChangeWalletName,
    ConfirmSend,
}

struct WalletApp {
    state: AppState,
    wallet_data: Option<WalletData>,
}

pub struct MyApp {
    state: AppState,
    side_panel_state: SidePanelState,
    wallet_data: WalletData,
    selected_wallet: Option<(String, WalletElement)>,
    threads: Vec<String>,
    sync_data_tx: mpsc::SyncSender<(String, SyncData)>,
    sync_data_rx: mpsc::Receiver<(String, SyncData)>,
    string_scratchpad_0: String,

    dialog_box: Option<DialogBoxEnum>,
    balance: Option<Balance>,
    transactions: Option<Vec<TransactionDetails>>,
}

impl MyApp {
    pub fn new() -> Self {
        let mut state = AppState::WalletNotAvailable;
        let mut side_panel_state = SidePanelState::Wallet;
        let mut wallet_data = WalletData::new(FILENAME);
        let mut selected_wallet = Option::None;

        if wallet_data.initialise_from_wallet_file().unwrap() {
            state = AppState::WalletAvailable;
            let selected_wallet_xpriv_str = wallet_data.get_first_wallet_xpriv_str();
            selected_wallet = Option::Some((
                selected_wallet_xpriv_str.clone(),
                wallet_data.get_wallet_element_from_xpriv_str(selected_wallet_xpriv_str),
            ))
        };
        let threads = Vec::with_capacity(3);
        let (sync_data_tx, sync_data_rx) = mpsc::sync_channel(0);

        let string_scratchpad_0 = String::new();

        let dialog_box = None;
        let balance = None;
        let transactions = None;
        let mut slf = Self {
            state,
            side_panel_state,
            wallet_data,
            selected_wallet,
            threads,
            sync_data_tx,
            sync_data_rx,
            string_scratchpad_0,
            dialog_box,
            balance,
            transactions,
        };

        slf
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some((private_key, wallet_element)) = &mut self.selected_wallet {
            self.wallet_poll();
            self.update_from_wallet_sync();
        }
        self.check_for_wallet();
        self.render_window(ctx, _frame);
    }
}

impl MyApp {
    fn update_from_wallet_sync(&mut self) {
        match self.sync_data_rx.try_recv() {
            Ok((thread_priv_key, thread_sync_data)) => {
                if thread_priv_key == self.selected_wallet.clone().unwrap().0 {
                    self.balance = Some(thread_sync_data.balance);
                    self.transactions = Some(thread_sync_data.transactions);
                }
                self.threads.retain(|priv_key| priv_key != &thread_priv_key);
                println!("{:?}", self.balance);
                println!("{:?}", self.transactions);
            }
            Err(_) => {}
        }
    }

    fn get_total(&self) -> u64 {
        match &self.balance {
            None => 0,
            Some(balance) => balance.clone().get_total(),
        }
    }

    fn wallet_poll(&mut self) {
        let wallet_element_kv = self.selected_wallet.clone().unwrap();
        let priv_key = wallet_element_kv.0;
        let mut wallet_element = wallet_element_kv.1;
        if self.threads.contains(&priv_key) {
            return;
        }
        wallet_element.start_wallet_syncing_worker(self.sync_data_tx.clone());
        self.threads.push(priv_key)
    }
}

impl MyApp {
    fn get_selected_wallet(&mut self) -> WalletElement {
        self.wallet_data
            .get_wallet_element_from_xpriv_str(self.selected_wallet.clone().unwrap().0)
    }
    fn check_for_wallet(&mut self) {
        match self.state {
            AppState::WalletNotAvailable => {
                if !self.wallet_data.wallets.is_empty() {
                    self.state = AppState::WalletAvailable;
                    let selected_wallet_xpriv_str = self.wallet_data.get_first_wallet_xpriv_str();
                    self.selected_wallet = Option::Some((
                        selected_wallet_xpriv_str.clone(),
                        self.get_selected_wallet(),
                    ));
                }
            }
            _ => (),
        }
    }

    fn render_window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_sidepanel(ctx, _frame);
        self.render_toppanel(ctx, _frame);
        self.render_centrepanel(ctx, _frame);

        match self.dialog_box {
            Some(DialogBoxEnum::NewMnemonic) => self.new_mnemonic_dialog_box_open_window(ctx),
            Some(DialogBoxEnum::ChangeWalletName) => self.rename_wallet_dialog_box_open_window(ctx),
            Some(DialogBoxEnum::ConfirmSend) => self.rename_wallet_dialog_box_open_window(ctx),
            None => (),
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
        let wallet = self.get_selected_wallet();

        ui.horizontal(|ui| {
            ui.heading(format!("Wallet Name: {}", &wallet.wallet_name.to_owned()));
            // let wallet_name = self
            if ui.button("Rename Wallet").clicked() {
                self.dialog_box = Some(DialogBoxEnum::ChangeWalletName);
                self.string_scratchpad_0 = wallet.wallet_name.to_string();
            }
        });
        ui.heading(format!("Wallet Balance: {:?}", self.get_total()));
        ui.add_space(50.0);
        let public_key = wallet.address;

        ui.label(format!("Public Key: {:?}", &public_key));
        if ui.button("Copy").clicked() {
            ui.output_mut(|o| o.copied_text = public_key);
        };
    }
    pub fn render_sending_panel(&mut self, ui: &mut Ui) {
        let wallet = self.get_selected_wallet();
        ui.heading(format!("Wallet Balance: {:?}", self.get_total()));
        ui.add_space(50.0);
        ui.horizontal(|ui| {
            ui.heading("Recipient Address");
            let mut amount = String::new();
            ui.text_edit_singleline(&mut amount);
        });
        ui.add_space(50.0);
        ui.horizontal(|ui| {
            ui.heading("Amount to send");
            let mut amount = "0".to_string();
            ui.text_edit_singleline(&mut amount);
            ui.label("Sats");
        });
        if ui.button("Send").clicked() {
            self.dialog_box = Some(DialogBoxEnum::NewMnemonic);
        }
    }
    pub fn render_receiving_panel(&mut self, ui: &mut Ui) {
        let wallet = self.get_selected_wallet();
        let address = wallet.address;
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

            if self.state == AppState::WalletNotAvailable {
                if ui.button("Create Wallet").clicked() {
                    self.string_scratchpad_0 = generate_mnemonic_string().unwrap();
                    self.dialog_box = Some(DialogBoxEnum::NewMnemonic);
                }
            } else {
                match self.side_panel_state {
                    SidePanelState::Wallet => self.render_wallet_panel(ui),
                    SidePanelState::Sending => self.render_sending_panel(ui),
                    SidePanelState::Receiving => self.render_receiving_panel(ui),
                    SidePanelState::Contacts => self.render_contacts_panel(ui),
                }
            }
        });
    }

    pub fn render_toppanel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("Headerbar")
            .exact_height(50.0)
            .show(ctx, |ui| {
                match self.dialog_box {
                    None => (),
                    _ => ui.set_enabled(false),
                }
                ui.heading("Rust Bitcoin Wallet");
            });
    }
}

impl MyApp {
    pub fn new_mnemonic_dialog_box_open_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("New Generated Mnemonic")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(self.string_scratchpad_0.clone());
                });
                ui.horizontal(|ui| {
                    if ui.button("Accept").clicked() {
                        let xkey = generate_xpriv(&self.string_scratchpad_0).unwrap();
                        let _ = self.wallet_data.add_wallet(xkey);
                        self.dialog_box = None;
                    }
                });
            });
    }
    pub fn rename_wallet_dialog_box_open_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Change Wallet Name?")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.string_scratchpad_0);
                });
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.dialog_box = None;
                    }

                    if ui.button("Accept").clicked() {
                        self.wallet_data
                            .wallets
                            .get_mut(&self.selected_wallet.as_mut().unwrap().0)
                            .unwrap()
                            .wallet_name = self.string_scratchpad_0.clone();
                        let _ = self.wallet_data.rename_wallet(
                            &self.selected_wallet.clone().unwrap().0,
                            &self.string_scratchpad_0,
                        );
                        self.dialog_box = None;
                    }
                });
            });
    }

    pub fn confirm_send_dialog_box_open_window(&self) {}
}

pub fn generate_qrcode_from_address(address: &str) -> Result<egui::ColorImage, image::ImageError> {
    let result = qrcode_generator::to_png_to_vec(address, QrCodeEcc::Medium, 100).unwrap();
    let dynamic_image = image::load_from_memory(&result).unwrap();
    let size = [dynamic_image.width() as _, dynamic_image.height() as _];
    let image_buffer = dynamic_image.to_luma8();
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_gray(size, pixels.as_slice()))
}
