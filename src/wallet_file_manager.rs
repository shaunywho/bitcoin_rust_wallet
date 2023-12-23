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
use std::io;
use std::io::BufRead;
use std::io::Read;
use std::io::Write;

use std::str::FromStr;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;

use crate::bitcoin_wallet::generate_wallet;
use crate::bitcoin_wallet::generate_xpriv;
use crate::bitcoin_wallet::make_transaction;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct JsonWalletFile {
    password: String,
    wallets: Vec<JsonWallet>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonWallet {
    priv_key: String,
    wallet_name: String,
}
const FILENAME: &str = "./wallet.txt";

pub struct SyncData {
    pub balance: Balance,
    pub transactions: Vec<TransactionDetails>,
}

pub struct WalletFileData {
    pub wallets: HashMap<String, WalletElement>,
    filename: String,
    blockchain: Arc<ElectrumBlockchain>,
    pub selected_wallet: Option<String>,
    pub password: Option<String>,
}

pub struct WalletElement {
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
            let wallet_locked = wallet_clone.lock().unwrap();
            wallet_locked
                .sync(&blockchain, SyncOptions::default())
                .unwrap();
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

impl WalletFileData {
    pub fn does_file_exist(&self) -> bool {
        let result = fs::metadata(FILENAME);
        if let Ok(_metadata) = result {
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
            password: None,
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
    pub fn initialise_from_wallet_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = self.get_file();
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let json_wallet_file: JsonWalletFile = serde_json::from_str(&contents)?;

        self.password = Some(json_wallet_file.password);
        for wallet in json_wallet_file.wallets {
            self.wallets.insert(
                wallet.priv_key.clone(),
                WalletElement::new(&wallet.priv_key, &wallet.wallet_name),
            );
        }
        if self.wallets.len() > 0 {
            self.selected_wallet = Some(self.get_first_wallet_xpriv_str());
        }

        Ok(())
    }

    fn append_to_wallet_file(
        &mut self,
        priv_key: &str,
        wallet_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json_wallet = JsonWallet {
            priv_key: priv_key.to_string(),
            wallet_name: wallet_name.to_string(),
        };

        let mut json_wallet_file = self.to_json_wallet_file()?;
        json_wallet_file.wallets.push(json_wallet);

        let json = serde_json::to_string(&json_wallet_file)?;
        fs::write(&self.filename, json)?;

        return Ok(());
    }

    fn to_json_wallet_file(&self) -> Result<JsonWalletFile, Box<dyn std::error::Error>> {
        let wallets = self
            .wallets
            .iter()
            .map(|(priv_key, wallet_element)| JsonWallet {
                priv_key: priv_key.clone(),
                wallet_name: wallet_element.wallet_name.clone(),
            })
            .collect();

        return Ok(JsonWalletFile {
            password: self.password.clone().unwrap(),
            wallets,
        });
    }

    pub fn add_wallet(&mut self, xpriv: ExtendedPrivKey) -> Result<(), Box<dyn std::error::Error>> {
        let priv_key = xpriv.to_string();
        if self.wallets.contains_key(&priv_key) {
            panic!("Wallet already exists");
        } else {
            let wallet_name = "New Wallet Name";
            self.append_to_wallet_file(&priv_key, wallet_name)?;
            self.wallets.insert(
                priv_key.clone(),
                WalletElement::new(&priv_key, &wallet_name),
            );
        }

        return Ok(());
    }

    pub fn add_wallet_from_mnemonic(
        &mut self,
        mnemonic: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let xpriv = generate_xpriv(mnemonic).unwrap();
        return self.add_wallet(xpriv);
    }

    pub fn add_password(&mut self, password: String) -> Result<(), Box<dyn std::error::Error>> {
        self.password = Some(password);
        let json_wallet_file = self.to_json_wallet_file()?;
        let json = serde_json::to_string(&json_wallet_file)?;
        fs::write(&self.filename, json)?;
        return Ok(());
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
