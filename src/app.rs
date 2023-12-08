use bdk::bitcoin::bip32::ExtendedPrivKey;

use crate::bitcoin_wallet::{
    bitcoin_test, generate_mnemonic_string, generate_wallet, generate_xpriv,
};

use crate::wallet_file_manager::WalletData;
use std::str::FromStr;
use std::sync::mpsc;
use std::thread::JoinHandle;

const FILENAME: &str = "./wallet.txt";

#[derive(PartialEq)]
enum AppState {
    WalletNotAvailable,
    WalletAvailable,
    DialogBox,
    TransactionSending,
    WalletSyncing,
    NetworkError,
}

enum DialogBoxEnum {
    NewMnemonic,
    ChangeWalletName,
}

struct WalletApp {
    state: AppState,
    wallet_data: Option<WalletData>,
}

/// State per thread.
struct ThreadState {
    thread_nr: usize,
    title: String,
    name: String,
    age: u32,
}

impl ThreadState {
    fn new(thread_nr: usize) -> Self {
        let title = format!("Background thread {thread_nr}");
        Self {
            thread_nr,
            title,
            name: "Arthur".into(),
            age: 12 + thread_nr as u32 * 10,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        let pos = egui::pos2(16.0, 128.0 * (self.thread_nr as f32 + 1.0));
        egui::Window::new(&self.title)
            .default_pos(pos)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Your name: ");
                    ui.text_edit_singleline(&mut self.name);
                });
                ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
                if ui.button("Click each year").clicked() {
                    self.age += 1;
                }
                ui.label(format!("Hello '{}', age {}", self.name, self.age));
            });
    }
}

fn new_worker(
    thread_nr: usize,
    on_done_tx: mpsc::SyncSender<()>,
) -> (JoinHandle<()>, mpsc::SyncSender<egui::Context>) {
    let (show_tx, show_rc) = mpsc::sync_channel(0);
    let handle = std::thread::Builder::new()
        .name(format!("EguiPanelWorker {thread_nr}"))
        .spawn(move || {
            let mut state = ThreadState::new(thread_nr);
            while let Ok(ctx) = show_rc.recv() {
                state.show(&ctx);
                let _ = on_done_tx.send(());
            }
        })
        .expect("failed to spawn thread");
    (handle, show_tx)
}

pub struct MyApp {
    state: AppState,
    wallet_data: WalletData,
    selected_wallet: Option<String>,
    threads: Vec<(JoinHandle<()>, mpsc::SyncSender<egui::Context>)>,
    on_done_tx: mpsc::SyncSender<()>,
    on_done_rc: mpsc::Receiver<()>,
    amount_to_send: String,
    wallet_amount: usize,
    shared_string: String,
    dialog_box: Option<DialogBoxEnum>,
}

impl MyApp {
    pub fn new() -> Self {
        // electrum_wallet_test::electrum_test();
        let mut state = AppState::WalletNotAvailable;
        let mut wallet_data = WalletData::new(FILENAME);
        let mut selected_wallet = Option::None;
        if wallet_data.initialise_from_wallet_file().unwrap() {
            state = AppState::WalletAvailable;
            selected_wallet = Option::Some(wallet_data.wallets.keys().nth(0).unwrap().to_owned());
        }
        let threads = Vec::with_capacity(3);
        let (on_done_tx, on_done_rc) = mpsc::sync_channel(0);
        let amount_to_send = format!("{}", 0);
        let wallet_amount = 0;
        let shared_string = String::new();
        let dialog_box = None;
        let mut slf = Self {
            state,
            wallet_data,
            selected_wallet,
            threads,
            on_done_tx,
            on_done_rc,
            amount_to_send,
            wallet_amount,
            shared_string,
            dialog_box,
        };

        slf
    }

    fn spawn_thread(&mut self) {
        let thread_nr = self.threads.len();
        self.threads
            .push(new_worker(thread_nr, self.on_done_tx.clone()));
    }
}

impl std::ops::Drop for MyApp {
    fn drop(&mut self) {
        for (handle, show_tx) in self.threads.drain(..) {
            std::mem::drop(show_tx);
            handle.join().unwrap();
        }
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
                    self.selected_wallet =
                        Option::Some(self.wallet_data.wallets.keys().nth(0).unwrap().to_owned());
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
            if self.state != AppState::WalletNotAvailable {
                for (secret_key, wallet_element) in &self.wallet_data.wallets {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            self.selected_wallet.as_mut().unwrap(),
                            secret_key.to_owned(),
                            format!("{}", wallet_element.wallet_name),
                        );
                    });
                }
            }
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
                    self.shared_string = generate_mnemonic_string().unwrap();
                    self.dialog_box = Some(DialogBoxEnum::NewMnemonic);
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Secret Key: {}",
                        self.selected_wallet.clone().unwrap()
                    ));
                    ui.horizontal(|ui| {
                        let wallet = self
                            .wallet_data
                            .get_wallet_from_xpriv_str(self.selected_wallet.clone().unwrap());
                        // let xpriv =
                        //     ExtendedPrivKey::from_str(&self.selected_wallet.clone().unwrap())
                        //         .unwrap();
                        // let wallet = self.wallet_data.get_wallet_from_xpriv(xpriv).unwrap();
                        // ui.label(format!("Wallet Balance: {:?}", wallet.get_balance()))
                    })
                });
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
                if self.state != AppState::WalletNotAvailable {
                    ui.horizontal_centered(|ui| {
                        if let Some(wallet_element) = self
                            .wallet_data
                            .wallets
                            .get_mut(&self.selected_wallet.clone().unwrap())
                        {
                            ui.style_mut().spacing.item_spacing = egui::vec2(200.0, 500.0);
                            ui.label("Wallet Name: ");
                            ui.label(wallet_element.wallet_name.to_owned());
                            // let wallet_name = self
                            if ui.button("Rename Wallet").clicked() {
                                self.dialog_box = Some(DialogBoxEnum::ChangeWalletName);
                                self.shared_string = wallet_element.wallet_name.to_string();
                            }
                        }
                    });
                }
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
                    ui.label(self.shared_string.clone());
                });
                ui.horizontal(|ui| {
                    if ui.button("Accept").clicked() {
                        let xkey = generate_xpriv(&self.shared_string).unwrap();
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
                    ui.text_edit_singleline(&mut self.shared_string);
                });
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.dialog_box = None;
                    }

                    if ui.button("Accept").clicked() {
                        self.wallet_data
                            .wallets
                            .get_mut(&self.selected_wallet.clone().unwrap())
                            .unwrap()
                            .wallet_name = self.shared_string.clone();
                        self.wallet_data.rename_wallet(
                            &self.selected_wallet.clone().unwrap(),
                            &self.shared_string,
                        );
                        self.dialog_box = None;
                    }
                });
            });
    }
}
