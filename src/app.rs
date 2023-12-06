use crate::bitcoin_wallet::{generate_key, generate_mnemonic, generate_mnemonic_string};
use crate::wallet_file_manager::{self, WalletData};
use std::sync::mpsc;
use std::thread::JoinHandle;

const FILENAME: &str = "./wallet.txt";
enum AppState {
    WalletAvailable,
    TransactionSending,
    WalletSyncing,
    NetworkError,
}

struct WalletApp {
    state: Option<AppState>,
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
    state: Option<AppState>,
    wallet_data: WalletData,
    selected_wallet: usize,
    threads: Vec<(JoinHandle<()>, mpsc::SyncSender<egui::Context>)>,
    on_done_tx: mpsc::SyncSender<()>,
    on_done_rc: mpsc::Receiver<()>,
    amount_to_send: String,
    wallet_amount: usize,
}

impl MyApp {
    pub fn new() -> Self {
        let mut state = Option::None;
        let mut wallet_data = WalletData::new(FILENAME);
        let mut selected_wallet = 0;
        if wallet_data.initialise_from_wallet_file().unwrap() {
            state = Option::Some(AppState::WalletAvailable)
        }
        // let mnemonic = generate_mnemonic_string().unwrap();
        // let xkey = generate_key(&mnemonic).unwrap();
        // wallet_data.add_wallet(xkey);
        let threads = Vec::with_capacity(3);
        let (on_done_tx, on_done_rc) = mpsc::sync_channel(0);
        let amount_to_send = format!("{}", 0);
        let wallet_amount = 0;
        let mut slf = Self {
            state,
            wallet_data,
            selected_wallet,
            threads,
            on_done_tx,
            on_done_rc,
            amount_to_send,
            wallet_amount,
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
        egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
            ui.label("Wallets");
            for (i, j) in self.wallet_data.wallets.values().enumerate() {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.selected_wallet, i, format!("{}", j));
                });
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello World!");
        });

        for (_handle, show_tx) in &self.threads {
            let _ = show_tx.send(ctx.clone());
        }

        for _ in 0..self.threads.len() {
            let _ = self.on_done_rc.recv();
        }
    }
}
