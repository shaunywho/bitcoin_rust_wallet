use crate::bitcoin_wallet::{generate_mnemonic_string, generate_xpriv};

use bdk::{Balance, TransactionDetails};

use crate::wallet_file_manager::{SyncData, WalletData, WalletElement};
use egui::Ui;

use std::sync::mpsc;

const FILENAME: &str = "./wallet.txt";

use qrcode_generator::QrCodeEcc;

#[derive(PartialEq)]
enum WalletFileState {
    WalletFileNotAvailable,
    WalletNotInitialised,
    WalletAvailable,
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
}

struct WalletApp {
    state: WalletFileState,
    wallet_data: Option<WalletData>,
}

pub struct MyApp {
    state: WalletFileState,
    side_panel_state: SidePanelState,
    wallet_data: WalletData,
    selected_wallet: Option<(String, WalletElement)>,
    threads: Vec<String>,
    sync_data_tx: mpsc::SyncSender<(String, SyncData)>,
    sync_data_rx: mpsc::Receiver<(String, SyncData)>,
    rename_wallet_string: String,
    recipient_address_string: String,
    amount_to_send_string: String,
    dialog_box: Option<DialogBox>,
    balance: Option<Balance>,
    transactions: Option<Vec<TransactionDetails>>,
}

impl MyApp {
    fn accept_process(&mut self, line_edit: Option<String>) {
        if let Some(dialog_box) = &self.dialog_box {
            match dialog_box.dialog_box_enum {
                DialogBoxEnum::NewMnemonic => {
                    let xkey = generate_xpriv(&dialog_box.message.clone().unwrap()).unwrap();
                    let _ = self.wallet_data.add_wallet(xkey);
                    self.dialog_box = None;
                }
                DialogBoxEnum::ChangeWalletName => {
                    self.rename_wallet_string = line_edit.unwrap();

                    self.wallet_data
                        .wallets
                        .get_mut(&self.selected_wallet.as_mut().unwrap().0)
                        .unwrap()
                        .wallet_name = self.rename_wallet_string.clone();
                    self.wallet_data.rename_wallet(
                        &self.selected_wallet.clone().unwrap().0,
                        &self.rename_wallet_string,
                    );
                    self.dialog_box = None;
                }
                DialogBoxEnum::ConfirmSend { .. } => {
                    // Implement the logic for accepting ConfirmSend
                    println!("HI");
                }
            }
        }
    }

    fn render_dialog_box(&mut self, ctx: &egui::Context) {
        // let dialog_box = self.dialog_box.as_mut().unwrap();
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

        let threads = Vec::with_capacity(3);
        let (sync_data_tx, sync_data_rx) = mpsc::sync_channel(0);

        let string_scratchpad = [String::new(), String::new(), String::new()];
        let rename_wallet_string = String::new();
        let recipient_address_string = String::new();
        let amount_to_send_string = String::new();
        let dialog_box = None;
        let balance = None;
        let transactions = None;
        let slf = Self {
            state,
            side_panel_state,
            wallet_data,
            selected_wallet,
            threads,
            sync_data_tx,
            sync_data_rx,
            rename_wallet_string,
            recipient_address_string,
            amount_to_send_string,
            dialog_box,
            balance,
            transactions,
        };

        slf
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_wallet_state();
        if let Some((_private_key, _wallet_element)) = &mut self.selected_wallet {
            self.wallet_poll();
            self.update_from_wallet_sync();
        }
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
    fn get_selected_wallet_element(&mut self) -> WalletElement {
        self.wallet_data
            .get_wallet_element(&self.selected_wallet.as_ref().unwrap().0)
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
                    self.selected_wallet =
                        Option::Some((selected_wallet_xpriv_str.clone(), selected_wallet_element));
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
        let wallet = self.get_selected_wallet_element();

        ui.vertical_centered(|ui| {
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
        ui.heading(format!("Wallet Balance: {:?}", self.get_total()));
        ui.add_space(50.0);
        let public_key = wallet.address;

        ui.label(format!("Public Key: {:?}", &public_key));
        if ui.button("Copy").clicked() {
            ui.output_mut(|o| o.copied_text = public_key);
        };
    }
    pub fn render_sending_panel(&mut self, ui: &mut Ui) {
        ui.heading(format!("Wallet Balance: {:?}", self.get_total()));
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
        }
    }
    pub fn render_receiving_panel(&mut self, ui: &mut Ui) {
        let wallet = self.get_selected_wallet_element();
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
