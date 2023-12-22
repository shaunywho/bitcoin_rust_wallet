use bdk::bitcoin::bip32::ExtendedPrivKey;
use bdk::bitcoin::Transaction;
use bdk::blockchain::Blockchain;
use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client;

use bdk::wallet::AddressIndex;
use bdk::wallet::Wallet;
use bdk::Balance;
use bdk::SyncOptions;
use bdk::TransactionDetails;
use csv::ReaderBuilder;

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;

use std::str::FromStr;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;

use crate::bitcoin_wallet::generate_wallet;
use crate::bitcoin_wallet::make_transaction;

const FILENAME: &str = "./wallet.txt";

pub struct SyncData {
    pub balance: Balance,
    pub transactions: Vec<TransactionDetails>,
}

pub struct WalletData {
    pub wallets: HashMap<String, WalletElement>,
    filename: String,
    blockchain: Arc<ElectrumBlockchain>,
    pub selected_wallet: Option<String>,
}

pub struct WalletElement {
    priv_key: String,
    pub wallet_name: String,
    pub address: String,
    pub wallet_obj: Arc<Mutex<Wallet<MemoryDatabase>>>,
    pub balance: Option<Balance>,
    pub sorted_transactions: Option<Vec<TransactionDetails>>,
}

impl WalletElement {
    pub fn new(priv_key: &str, wallet_name: &str) -> Self {
        let wallet_obj = generate_wallet(ExtendedPrivKey::from_str(priv_key).unwrap()).unwrap();

        let wallet_name = wallet_name.to_string();

        let address = wallet_obj
            .get_address(AddressIndex::Peek(0))
            .unwrap()
            .to_string();

        return Self {
            priv_key: priv_key.to_string(),
            wallet_name: wallet_name,
            address: address,
            wallet_obj: Arc::new(Mutex::new(wallet_obj)),
            balance: None,
            sorted_transactions: None,
        };
    }

    pub fn start_wallet_syncing_worker(
        &self,
        blockchain: Arc<ElectrumBlockchain>,
        sync_sender: Sender<SyncData>,
    ) -> JoinHandle<()> {
        let wallet_clone = Arc::clone(&self.wallet_obj);
        let handle = thread::spawn(move || {
            let mut wallet_locked = wallet_clone.lock().unwrap();
            wallet_locked.sync(&blockchain, SyncOptions::default());
            let balance = wallet_locked.get_balance().unwrap();
            let transactions = wallet_locked.list_transactions(true).unwrap();
            let sync_data = SyncData {
                balance,
                transactions,
                // ... other fields you might want to include
            };
            sync_sender
                .send(sync_data)
                .expect("Failed to send sync data");
        });
        return handle;
    }
    pub fn get_total(&self) -> u64 {
        match &self.balance {
            None => 0,
            Some(balance) => balance.clone().get_total(),
        }
    }

    pub fn make_transaction(&self, recipient_address: &str, amount: u64) -> Transaction {
        let wallet_locked = self.wallet_obj.lock().unwrap();
        let transaction = make_transaction(&wallet_locked, recipient_address, amount);
        return transaction;
    }
}

impl WalletData {
    pub fn does_file_exist(&self) -> bool {
        let result = fs::metadata(FILENAME);
        if let Ok(metadata) = result {
            return true;
        } else {
            return false;
        }
    }

    pub fn sync_current_wallet(&mut self, sync_sender: Sender<SyncData>) -> JoinHandle<()> {
        let blockchain = Arc::clone(&self.blockchain);
        let handle = self
            .get_selected_wallet_element()
            .start_wallet_syncing_worker(blockchain, sync_sender);
        return handle;
    }

    pub fn new(filename: &str) -> Self {
        let client = Client::new("ssl://electrum.blockstream.info:60002").unwrap();
        let blockchain = ElectrumBlockchain::from(client);
        let wallet_data = Self {
            wallets: HashMap::new(),
            filename: filename.to_string(),
            blockchain: Arc::new(blockchain),
            selected_wallet: None,
        };

        return wallet_data;
    }

    fn get_file(&mut self) -> File {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(self.filename.clone())
            .unwrap()
    }
    pub fn initialise_from_wallet_file(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let file = self.get_file();
        let mut rdr = ReaderBuilder::new().has_headers(false).from_reader(file);
        let mut found_record = false;
        for result in rdr.records() {
            let record = result?;
            if record.len() == 2 {
                // Assuming the CSV has two columns: private key and wallet name
                let private_key_str = record[0].trim();
                let wallet_name = record[1].trim();

                if !private_key_str.is_empty() {
                    self.wallets.insert(
                        private_key_str.to_owned(),
                        WalletElement::new(private_key_str, wallet_name),
                    );
                }
            }
            found_record = true;
        }
        Ok(found_record)
    }

    pub fn add_wallet(&mut self, xprv: ExtendedPrivKey) -> Result<(), Box<dyn std::error::Error>> {
        let priv_key = xprv.to_string();
        if self.wallets.contains_key(&priv_key) {
            panic!("Wallet already exists");
        } else {
            self.append_to_wallet_file(&priv_key)?;
        }

        Ok(())
    }

    fn append_to_wallet_file(&mut self, priv_key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = self.get_file();
        if let Err(e) = writeln!(file, "{}, {}", priv_key, "New Wallet Name") {
            eprintln!("Couldn't write to file: {}", e);
        }
        Ok(())
    }

    pub fn rename_wallet(
        &mut self,
        selected_priv_key: &str,
        new_wallet_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = self.get_file();
        let mut updated_records: Vec<String> = Vec::new();

        let mut found_record = false;
        for (priv_key, wallet_element) in &self.wallets {
            if selected_priv_key == priv_key {
                updated_records.push(format!("{}, {}", priv_key, new_wallet_name));
                found_record = true;
            } else {
                updated_records.push(format!("{}, {}", priv_key, wallet_element.wallet_name));
            }
        }

        if !found_record {
            return Err("Record not found".into());
        }

        file.set_len(0)?; // Clear the file

        for record in updated_records {
            if let Err(e) = writeln!(file, "{}", record) {
                eprintln!("Couldn't write to file: {}", e);
            }
        }

        Ok(())
    }

    pub fn get_first_wallet_xpriv_str(&mut self) -> String {
        let first_wallet = self.wallets.keys().nth(0).unwrap().to_owned();
        return first_wallet;
    }

    pub fn get_wallet_element(&mut self, xpriv_str: &str) -> &mut WalletElement {
        // Using entry() to ensure the key exists and get a mutable reference
        self.wallets
            .entry(xpriv_str.to_string())
            .or_insert_with(|| WalletElement::new("", "")) // Create a new WalletElement if key doesn't exist
    }

    pub fn get_selected_wallet_string(&self) -> String {
        return self.selected_wallet.clone().unwrap();
    }

    pub fn get_selected_wallet_element(&mut self) -> &mut WalletElement {
        let wallet_string = self.get_selected_wallet_string();
        return self.get_wallet_element(&wallet_string);
    }

    pub fn send_transaction(&mut self, recipient_address: &str, amount: u64) {
        let transaction = self
            .get_selected_wallet_element()
            .make_transaction(recipient_address, amount);
        self.blockchain.broadcast(&transaction).unwrap();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn working() {
        assert!(true);
    }
}
