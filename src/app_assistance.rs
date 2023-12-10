use std::sync::mpsc;
use std::thread::JoinHandle;

use eframe::egui;

// fn main() -> Result<(), eframe::Error> {
//     env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
//     let options = eframe::NativeOptions {
//         viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
//         ..Default::default()
//     };
//     eframe::run_native(
//         "My parallel egui App",
//         options,
//         Box::new(|_cc| Box::new(MyApp::new())),
//     )
// }

/// State per thread.
pub struct WalletThreadState {
    wallet_priv_key: String,
    balance: f32,
    transactions: String,
}

impl WalletThreadState {
    fn new(priv_key: &str) -> Self {
        let wallet_priv_key: priv_key;
        let balance = 0;
        let transactions = String::new();
        Self {
            wallet_priv_key,
            thread_nr,
            balance,
            transactions,
        }
    }

    fn update(&mut self) {
        self.wallet_priv_key
    }
}

fn wallet_sync_worker(
    thread_nr: usize,
    on_done_tx: mpsc::SyncSender<()>,
) -> (JoinHandle<()>, mpsc::SyncSender<egui::Context>) {
    let (show_tx, show_rc) = mpsc::sync_channel(0);
    let handle = std::thread::Builder::new()
        .name(format!("Wallet Sync Worker"))
        .spawn(move || {
            let mut state = ThreadState::new(thread_nr);
            while show_rc.recv() {
                state.update();
                let _ = on_done_tx.send(());
            }
        })
        .expect("failed to spawn thread");
    (handle, show_tx)
}

struct MyApp {
    threads: Vec<(JoinHandle<()>, mpsc::SyncSender<egui::Context>)>,
    on_done_tx: mpsc::SyncSender<()>,
    on_done_rc: mpsc::Receiver<()>,
}

impl MyApp {
    fn spawn_thread(&mut self) {
        let thread_nr = self.threads.len();
        self.threads
            .push(wallet_sync_worker(thread_nr, self.on_done_tx.clone()));
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
        egui::Window::new("Main thread").show(ctx, |ui| {
            if ui.button("Spawn another thread").clicked() {
                self.spawn_thread();
            }
        });

        for (_handle, show_tx) in &self.threads {
            let _ = show_tx.send(ctx.clone());
        }

        for _ in 0..self.threads.len() {
            let _ = self.on_done_rc.recv();
        }
    }
}
