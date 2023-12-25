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
use magic_crypt::MagicCrypt256;

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;

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

use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonWalletFile {
    wallets: Vec<JsonWallet>,
    contacts: Vec<JsonContact>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonWallet {
    priv_key: String,
    wallet_name: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonContact {
    pub_key: String,
    wallet_name: String,
}

const FILENAME: &str = "./wallet.txt";

pub struct SyncData {
    pub balance: Balance,
    pub transactions: Vec<TransactionDetails>,
}

pub struct WalletFileData {
    pub wallets: HashMap<String, WalletElement>,
    pub contacts: HashMap<String, String>,
    filename: String,
    blockchain: Arc<ElectrumBlockchain>,
    pub selected_wallet: Option<String>,
    pub key: Option<MagicCrypt256>,
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
            contacts: HashMap::new(),
            filename: filename.to_string(),
            blockchain: Arc::new(blockchain),
            selected_wallet: None,
            key: None,
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
        let mut encrypted_contents = String::new();
        file.read_to_string(&mut encrypted_contents)?;
        let contents = self
            .key
            .clone()
            .unwrap()
            .decrypt_base64_to_string(&encrypted_contents)
            .unwrap();
        let json_wallet_file: JsonWalletFile = serde_json::from_str(&contents)?;

        for wallet in json_wallet_file.wallets {
            self.wallets.insert(
                wallet.priv_key.clone(),
                WalletElement::new(&wallet.priv_key, &wallet.wallet_name),
            );
        }
        if self.wallets.len() > 0 {
            self.selected_wallet = Some(self.get_first_wallet_xpriv_str());
        }
        // self.test_addresses();
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

        let json_string = serde_json::to_string(&json_wallet_file)?;

        let encrypted_string = self.key.clone().unwrap().encrypt_str_to_base64(json_string);
        fs::write(&self.filename, encrypted_string)?;

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
        let contacts = self
            .contacts
            .iter()
            .map(|(pub_key, wallet_name)| JsonContact {
                pub_key: pub_key.clone(),
                wallet_name: wallet_name.clone(),
            })
            .collect();
        return Ok(JsonWalletFile { wallets, contacts });
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
            self.selected_wallet = Some(priv_key.clone());
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

    pub fn create_passworded_file(
        &mut self,
        password: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json_wallet_file = self.to_json_wallet_file()?;
        let json_string = serde_json::to_string(&json_wallet_file)?;
        self.key = Some(new_magic_crypt!(password, 256));
        let encrypted_string = self.key.clone().unwrap().encrypt_str_to_base64(json_string);
        fs::write(&self.filename, encrypted_string)?;
        return Ok(());
    }
    //  Below is fishy, probably want to redo it
    pub fn rename_wallet(
        &mut self,
        selected_priv_key: &str,
        new_wallet_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = self.get_file();
        let mut encrypted_contents = String::new();
        file.read_to_string(&mut encrypted_contents)?;
        let contents = self
            .key
            .clone()
            .unwrap()
            .decrypt_base64_to_string(&encrypted_contents)
            .unwrap();
        let mut json_wallet_file: JsonWalletFile = serde_json::from_str(&contents)?;

        for wallet in &mut json_wallet_file.wallets {
            if wallet.priv_key == selected_priv_key {
                wallet.wallet_name = new_wallet_name.to_string();

                break;
            }
        }

        let json_string = serde_json::to_string(&json_wallet_file)?;

        let encrypted_string = self.key.clone().unwrap().encrypt_str_to_base64(json_string);
        fs::write(&self.filename, encrypted_string)?;

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

    pub fn validate_password(&mut self, password: &str) -> bool {
        let mut file = self.get_file();
        let mut encrypted_contents = Vec::new();
        let mc = new_magic_crypt!(password, 256);
        file.read_to_end(&mut encrypted_contents).unwrap();
        let encrypted_string = String::from_utf8(encrypted_contents).unwrap();
        let contents = mc.decrypt_base64_to_string(encrypted_string);
        // let open_result = serde_json::from_str(&contents).unwrap();
        match contents {
            Err(_) => return false,
            Ok(_) => {
                self.key = Some(mc);
                return true;
            }
        }
    }
}

pub fn encryption_test() {
    let mc = new_magic_crypt!("magickey", 256);

    let base64 = mc.encrypt_str_to_base64("http://magiclen.org");

    assert_eq!("DS/2U8royDnJDiNY2ps3f6ZoTbpZo8ZtUGYLGEjwLDQ=", base64);

    assert_eq!(
        "http://magiclen.org",
        mc.decrypt_base64_to_string(&base64).unwrap()
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn working() {
        assert!(true);
    }
}
