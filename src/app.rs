use crate::bitcoin_wallet::{
    bitcoin_test, generate_mnemonic_string, generate_wallet, generate_xpriv, get_transactions,
};
use bdk::bitcoin::bip32::ExtendedPrivKey;
use bdk::database::MemoryDatabase;
use bdk::wallet::{self, AddressIndex};
use egui::ahash::HashMap;
use egui::{Context, Image, ImageButton, ImageSource, Visuals};
use egui_extras::install_image_loaders;

use crate::wallet_file_manager::WalletData;
use egui::TextStyle;
use egui_extras::image::RetainedImage;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::mpsc;
use std::thread::JoinHandle;
const FILENAME: &str = "./wallet.txt";
const IMAGE: &str = "../assets/wallet.png";
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
    side_panel_state: SidePanelState,
    wallet_data: WalletData,
    selected_wallet: Option<(String, Rc<wallet::Wallet<MemoryDatabase>>)>,
    threads: Vec<(JoinHandle<()>, mpsc::SyncSender<egui::Context>)>,
    on_done_tx: mpsc::SyncSender<()>,
    on_done_rc: mpsc::Receiver<()>,
    amount_to_send: String,
    wallet_amount: usize,
    string_scratchpad: String,
    dialog_box: Option<DialogBoxEnum>,
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
                wallet_data
                    .get_wallet_from_xpriv_str(selected_wallet_xpriv_str)
                    .unwrap(),
            ))
        };
        let threads = Vec::with_capacity(3);
        let (on_done_tx, on_done_rc) = mpsc::sync_channel(0);
        let amount_to_send = format!("{}", 0);
        let wallet_amount = 0;
        let string_scratchpad = String::new();
        let dialog_box = None;

        let mut slf = Self {
            state,
            side_panel_state,
            wallet_data,
            selected_wallet,
            threads,
            on_done_tx,
            on_done_rc,
            amount_to_send,
            wallet_amount,
            string_scratchpad,
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

impl MyApp {
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
        let wallet = self
            .wallet_data
            .get_wallet_from_xpriv_str(self.selected_wallet.clone().unwrap().0)
            .unwrap();
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
            ui.horizontal(|ui| {
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
            });
            let side_panel_state = SidePanelState::Sending;
            ui.horizontal(|ui| {
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
            });
            let side_panel_state = SidePanelState::Receiving;
            ui.horizontal(|ui| {
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
            });
            let side_panel_state = SidePanelState::Contacts;
            ui.horizontal(|ui| {
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
                }
            });
        });
    }
    pub fn render_centrepanel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.dialog_box {
                None => (),
                _ => ui.set_enabled(false),
            }

            if self.state == AppState::WalletNotAvailable {
                if ui.button("Create Wallet").clicked() {
                    self.string_scratchpad = generate_mnemonic_string().unwrap();
                    self.dialog_box = Some(DialogBoxEnum::NewMnemonic);
                }
            } else {
                // if ui.button("Addre"))
                ui.label(format!(
                    "Secret Key: {}",
                    &mut self.selected_wallet.clone().unwrap().0
                ));
                ui.add_space(50.0);
                let wallet = self
                    .wallet_data
                    .get_wallet_from_xpriv_str(self.selected_wallet.clone().unwrap().0)
                    .unwrap();

                ui.label(format!("Wallet Balance: {:?}", wallet.get_balance()));
                ui.add_space(50.0);
                let address = wallet.get_address(AddressIndex::Peek(0));
                ui.label(format!("Public Key: {:?}", address.unwrap().address));
                ui.add_space(50.0);
            }

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
                    self.string_scratchpad = wallet_element.wallet_name.to_string();
                }
            });
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
                    ui.label(self.string_scratchpad.clone());
                });
                ui.horizontal(|ui| {
                    if ui.button("Accept").clicked() {
                        let xkey = generate_xpriv(&self.string_scratchpad).unwrap();
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
                    ui.text_edit_singleline(&mut self.string_scratchpad);
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
                            .wallet_name = self.string_scratchpad.clone();
                        self.wallet_data.rename_wallet(
                            &self.selected_wallet.clone().unwrap().0,
                            &self.string_scratchpad,
                        );
                        self.dialog_box = None;
                    }
                });
            });
    }
}

fn load_image_from_path(path: &std::path::Path) -> Result<egui::ColorImage, image::ImageError> {
    let image = image::io::Reader::open(path)?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
}
