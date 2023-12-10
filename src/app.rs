use crate::bitcoin_wallet::{
    bitcoin_test, generate_mnemonic_string, generate_wallet, generate_xpriv, get_balance,
    get_transactions,
};
use bdk::bitcoin::bip32::ExtendedPrivKey;
use bdk::bitcoin::Transaction;
use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client;
use bdk::wallet::{self, AddressIndex};
use bdk::Balance;
use bdk::{SyncOptions, Wallet};

use crate::wallet_file_manager::WalletData;
use egui::{Context, Image, ImageButton, ImageData, Ui, Visuals};
use qrcode::render::unicode::Dense1x2;
use std::str::FromStr;
use std::thread;

use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Instant;
const FILENAME: &str = "./wallet.txt";
const IMAGE: &str = "../assets/wallet.png";

use image::{ImageBuffer, Luma, Rgb};
use qrcode::{Color, QrCode};
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
}

struct WalletApp {
    state: AppState,
    wallet_data: Option<WalletData>,
}

pub struct MyApp {
    state: AppState,
    blockchain: ElectrumBlockchain,
    side_panel_state: SidePanelState,
    wallet_data: WalletData,
    selected_wallet: Option<(String, Rc<wallet::Wallet<MemoryDatabase>>)>,
    threads: Vec<(JoinHandle<()>, mpsc::SyncSender<egui::Context>)>,
    on_done_tx: mpsc::SyncSender<()>,
    on_done_rc: mpsc::Receiver<()>,
    amount_to_send: String,
    balance: Option<Balance>,
    transactions: Option<Vec<Transaction>>,

    string_scratchpad_0: String,
    string_scratchpad_1: String,
    dialog_box: Option<DialogBoxEnum>,
}

impl MyApp {
    pub fn new() -> Self {
        let mut state = AppState::WalletNotAvailable;
        let client = Client::new("ssl://electrum.blockstream.info:60002").unwrap();
        let blockchain = ElectrumBlockchain::from(client);
        let mut side_panel_state = SidePanelState::Wallet;
        let mut wallet_data = WalletData::new(FILENAME);
        let mut selected_wallet = Option::None;

        if wallet_data.initialise_from_wallet_file().unwrap() {
            state = AppState::WalletAvailable;
            let selected_wallet_xpriv_str = wallet_data.get_first_wallet_xpriv_str();
            selected_wallet = Option::Some((
                selected_wallet_xpriv_str.clone(),
                wallet_data
                    .get_wallet_from_xpriv_str(selected_wallet_xpriv_str)
                    .unwrap(),
            ))
        };
        let threads = Vec::with_capacity(3);
        let (on_done_tx, on_done_rc) = mpsc::sync_channel(0);
        let amount_to_send = format!("{}", 0);
        let balance = None;
        let transactions = None;
        let string_scratchpad_0 = String::new();
        let string_scratchpad_1 = String::new();
        let dialog_box = None;

        let mut slf = Self {
            state,
            blockchain,
            side_panel_state,
            wallet_data,
            selected_wallet,
            threads,
            on_done_tx,
            on_done_rc,
            amount_to_send,
            balance,
            transactions,
            string_scratchpad_0,
            string_scratchpad_1,
            dialog_box,
        };

        slf
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_for_wallet();
        self.render_window(ctx, _frame);
    }
}

struct WalletSyncWorkerState {
    balance: u64,
    transactions: String,
    public_key: String,
    private_key: String,
    now: Instant,
}

impl MyApp {
    // fn start_wallet_syncing_worker(&mut self) {
    //     let priv_key = ExtendedPrivKey::from_str(&self.selected_wallet.clone().unwrap().0).unwrap();
    //     let wallet = generate_wallet(priv_key).unwrap();

    //     thread::spawn(|| loop {
    //         wallet.sync(&self.blockchain, SyncOptions::default());
    //     });
    // }
    fn get_selected_wallet(&mut self) -> Rc<Wallet<MemoryDatabase>> {
        self.wallet_data
            .get_wallet_from_xpriv_str(self.selected_wallet.clone().unwrap().0)
            .unwrap()
    }
    fn check_for_wallet(&mut self) {
        match self.state {
            AppState::WalletNotAvailable => {
                if !self.wallet_data.wallets.is_empty() {
                    self.state = AppState::WalletAvailable;
                    let selected_wallet_xpriv_str = self.wallet_data.get_first_wallet_xpriv_str();
                    self.selected_wallet = Option::Some((
                        selected_wallet_xpriv_str.clone(),
                        self.wallet_data
                            .get_wallet_from_xpriv_str(selected_wallet_xpriv_str)
                            .unwrap(),
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
            None => (),
        }

        for (_handle, show_tx) in &self.threads {
            let _ = show_tx.send(ctx.clone());
        }

        for _ in 0..self.threads.len() {
            let _ = self.on_done_rc.recv();
        }
    }
    pub fn render_sidepanel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
            match self.dialog_box {
                None => (),
                _ => ui.set_enabled(false),
            }

            ui.label("Wallets");
            let side_panel_state = SidePanelState::Wallet;
            ui.add_space(10.0);
            if ui
                .add_sized(
                    [100.0, 100.0],
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
                    [100.0, 100.0],
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
                    [100.0, 100.0],
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
                    [100.0, 100.0],
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
            let wallet_element = self
                .wallet_data
                .wallets
                .get(&self.selected_wallet.clone().unwrap().0)
                .unwrap();
            ui.label(format!(
                "Wallet Name: {}",
                wallet_element.wallet_name.to_owned()
            ));
            // let wallet_name = self
            if ui.button("Rename Wallet").clicked() {
                self.dialog_box = Some(DialogBoxEnum::ChangeWalletName);
                self.string_scratchpad_0 = wallet_element.wallet_name.to_string();
            }
        });
        ui.label(format!("Wallet Balance: {:?}", get_balance(&wallet)));
        ui.add_space(50.0);
        let public_key = wallet
            .get_address(AddressIndex::Peek(0))
            .unwrap()
            .to_string();

        ui.label(format!("Public Key: {:?}", &public_key));
        if ui.button("Copy").clicked() {
            ui.output_mut(|o| o.copied_text = public_key);
        };
    }
    pub fn render_sending_panel(&mut self, ui: &mut Ui) {
        let wallet = self.get_selected_wallet();
        ui.label(format!("Wallet Balance: {:?}", get_balance(&wallet)));
        ui.add_space(50.0);
        ui.label("Amount");
        let mut amount = "0.0".to_string();
        ui.text_edit_singleline(&mut amount);
        ui.label("BTC");
        let mut description = "Enter Description Here".to_string();
        ui.label("Description");
        ui.add_space(50.0);
        ui.text_edit_singleline(&mut description);
    }
    pub fn render_receiving_panel(&mut self, ui: &mut Ui) {
        let wallet = self.get_selected_wallet();
        let address = wallet.get_address(AddressIndex::Peek(0));
        ui.label(format!(
            "Public Key: {:?}",
            address.as_ref().unwrap().address
        ));
        // Encode some data into bits.

        let img = ui.ctx().load_texture(
            "my-image",
            generate_qrcode_from_address(&address.unwrap().address.to_string()).unwrap(),
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
                        self.wallet_data.add_wallet(xkey);
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
                        self.wallet_data.rename_wallet(
                            &self.selected_wallet.clone().unwrap().0,
                            &self.string_scratchpad_0,
                        );
                        self.dialog_box = None;
                    }
                });
            });
    }
}

pub fn get_image(filepath: &str, ix: u32, iy: u32, iw: u32, ih: u32) -> ImageData {
    let fp = Path::new(filepath);
    let color_image = load_image_from_path(&fp).unwrap();
    let img = ImageData::from(color_image);
    img
}
fn load_image_from_path(path: &std::path::Path) -> Result<egui::ColorImage, image::ImageError> {
    let image_reader = image::io::Reader::open(path);
    let image = image_reader?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_luma8();
    let pixels = image_buffer.as_flat_samples();

    Ok(egui::ColorImage::from_gray(size, pixels.as_slice()))
}

pub fn generate_qrcode_from_address(address: &str) -> Result<egui::ColorImage, image::ImageError> {
    let result = qrcode_generator::to_png_to_vec(address, QrCodeEcc::Medium, 100).unwrap();
    let dynamic_image = image::load_from_memory(&result).unwrap();
    let size = [dynamic_image.width() as _, dynamic_image.height() as _];
    let image_buffer = dynamic_image.to_luma8();
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_gray(size, pixels.as_slice()))
}
