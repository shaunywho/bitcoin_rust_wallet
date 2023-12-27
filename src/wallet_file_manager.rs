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
pub struct JsonWalletData {
    pub wallets: Vec<JsonWallet>,
    pub contacts: Vec<JsonWallet>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonWallet {
    pub pub_key: String,
    pub priv_key: Option<String>,
    pub wallet_name: String,
    pub balance: Option<Balance>,
    pub sorted_transactions: Option<Vec<TransactionDetails>>,
}
pub enum EntryType {
    Wallet,
    Contact,
}

const FILENAME: &str = "./wallet.txt";

pub struct SyncData {
    pub priv_key: String,
    pub balance: Balance,
    pub transactions: Vec<TransactionDetails>,
}

pub struct WalletModel {
    pub json_wallet_data: JsonWalletData,
    pub wallet_objs: HashMap<String, Arc<Mutex<Wallet<MemoryDatabase>>>>,
    filename: String,
    blockchain: Arc<ElectrumBlockchain>,
    pub selected_wallet: Option<String>,
    pub key: Option<MagicCrypt256>,
}

// pub struct WalletElement {
//     pub wallet_name: String,
//     pub address: String,
//     pub wallet_obj: Arc<Mutex<Wallet<MemoryDatabase>>>,
//     pub balance: Option<Balance>,
//     pub sorted_transactions: Option<Vec<TransactionDetails>>,
// }

// impl WalletElement {
//     pub fn new(priv_key: &str, wallet_name: &str) -> Self {
//         let wallet_obj = generate_wallet(ExtendedPrivKey::from_str(priv_key).unwrap()).unwrap();

//         let wallet_name = wallet_name.to_string();

//         let address = wallet_obj
//             .get_address(AddressIndex::Peek(0))
//             .unwrap()
//             .to_string();

//         return Self {
//             wallet_name: wallet_name,
//             address: address,
//             wallet_obj: Arc::new(Mutex::new(wallet_obj)),
//             balance: None,
//             sorted_transactions: None,
//         };
//     }

//     pub fn get_total(&self) -> u64 {
//         match &self.balance {
//             None => 0,
//             Some(balance) => balance.clone().get_total(),
//         }
//     }

//     pub fn make_transaction(&self, recipient_address: &str, amount: u64) -> Transaction {
//         let wallet_locked = self.wallet_obj.lock().unwrap();
//         let transaction = make_transaction(&wallet_locked, recipient_address, amount);
//         return transaction;
//     }
// }

impl JsonWallet {
    pub fn get_total(&self) -> u64 {
        match &self.balance {
            None => 0,
            Some(balance) => balance.clone().get_total(),
        }
    }
}

impl WalletModel {
    pub fn start_wallet_syncing_worker(
        &self,
        wallet: Arc<Mutex<Wallet<MemoryDatabase>>>,
        sync_sender: Sender<SyncData>,
    ) -> JoinHandle<()> {
        let blockchain = Arc::clone(&self.blockchain);
        let priv_key = self.get_selected_wallet_string();
        let handle = thread::spawn(move || {
            let wallet_locked = wallet.lock().unwrap();
            wallet_locked
                .sync(&blockchain, SyncOptions::default())
                .unwrap();

            let balance = wallet_locked.get_balance().unwrap();
            let transactions = wallet_locked.list_transactions(true).unwrap();
            let sync_data = SyncData {
                priv_key,
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
    pub fn does_file_exist(&self) -> bool {
        let result = fs::metadata(FILENAME);
        if let Ok(_metadata) = result {
            return true;
        } else {
            return false;
        }
    }

    pub fn sync_current_wallet(&mut self, sync_sender: Sender<SyncData>) -> JoinHandle<()> {
        let wallet = self.get_selected_wallet();
        let handle = self.start_wallet_syncing_worker(wallet, sync_sender);
        return handle;
    }

    pub fn new(filename: &str) -> Self {
        let client = Client::new("ssl://electrum.blockstream.info:60002").unwrap();
        let blockchain = ElectrumBlockchain::from(client);
        let wallet_data = Self {
            json_wallet_data: JsonWalletData {
                wallets: Vec::new(),
                contacts: Vec::new(),
            },
            wallet_objs: HashMap::new(),
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
        self.json_wallet_data = serde_json::from_str(&contents)?;

        // for wallet in json_wallet_file.wallets {
        //     self.wallets.insert(
        //         wallet.address.clone(),
        //         WalletElement::new(&wallet.address, &wallet.wallet_name),
        //     );
        // }
        for wallet in self.json_wallet_data.wallets.iter() {
            let priv_key = wallet.priv_key.clone().unwrap();
            self.wallet_objs.insert(
                priv_key.clone(),
                Arc::new(Mutex::new(generate_wallet(&priv_key).unwrap())),
            );
        }
        if self.json_wallet_data.wallets.len() > 0 {
            self.selected_wallet = Some(self.get_first_wallet_xpriv_str());
        }
        // self.test_addresses();
        Ok(())
    }

    fn append_to_file(
        &mut self,
        priv_key: Option<String>,
        pub_key: &str,
        wallet_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json_wallet = JsonWallet {
            priv_key: priv_key.clone(),
            pub_key: pub_key.to_string(),
            wallet_name: wallet_name.to_string(),
            balance: None,
            sorted_transactions: None,
        };

        // let mut json_wallet_file = self.to_json_wallet_file()?;

        match priv_key {
            Some(_) => self.json_wallet_data.wallets.push(json_wallet),
            None => self.json_wallet_data.contacts.push(json_wallet),
        }

        let json_string = serde_json::to_string(&self.json_wallet_data)?;
        let encrypted_string = self.key.clone().unwrap().encrypt_str_to_base64(json_string);
        fs::write(&self.filename, encrypted_string)?;

        Ok(())
    }

    // fn to_json_wallet_file(&self) -> Result<JsonWalletData, Box<dyn std::error::Error>> {
    //     let wallets = self
    //         .wallets
    //         .iter()
    //         .map(|(address, wallet_element)| JsonWallet {
    //             address: address.clone(),
    //             wallet_name: wallet_element.wallet_name.clone(),
    //             balance: None,
    //             sorted_transactions: None,
    //         })
    //         .collect();
    //     let contacts = self
    //         .contacts
    //         .iter()
    //         .map(|(address, wallet_name)| JsonWallet {
    //             address: address.clone(),
    //             wallet_name: wallet_name.clone(),
    //             balance: None,
    //             sorted_transactions: None,
    //         })
    //         .collect();
    //     return Ok(JsonWalletData { wallets, contacts });
    // }

    pub fn add_wallet(&mut self, priv_key: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self
            .json_wallet_data
            .wallets
            .iter()
            .any(|wallet| wallet.priv_key == Some(priv_key.to_owned()))
        {
            panic!("Wallet already exists");
        } else {
            let wallet_name = "New Wallet Name";
            let wallet = generate_wallet(priv_key).unwrap();
            self.append_to_file(
                Some(priv_key.to_owned()),
                &wallet
                    .get_address(AddressIndex::Peek(0))
                    .unwrap()
                    .to_string(),
                wallet_name,
            )?;
            self.wallet_objs
                .insert(priv_key.to_string(), Arc::new(Mutex::new(wallet)));
            self.selected_wallet = Some(priv_key.to_string());
        }

        return Ok(());
    }

    // pub fn add_contact(
    //     &mut self,
    //     pub_key: &str,
    //     wallet_name: &str,
    // ) -> Result<(), Box<dyn std::error::Error>> {
    //     if self
    //         .json_wallet_data
    //         .contacts
    //         .iter()
    //         .any(|wallet| wallet.pub_key == pub_key)
    //     {
    //         panic!("Wallet already exists");
    //     }
    //     self.append_to_file(None, pub_key, wallet_name);

    //     return Ok(());
    // }

    pub fn add_wallet_from_mnemonic(
        &mut self,
        mnemonic: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let xpriv = generate_xpriv(mnemonic).unwrap().to_string();

        return self.add_wallet(&xpriv);
    }

    pub fn create_passworded_file(
        &mut self,
        password: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json_string = serde_json::to_string(&self.json_wallet_data)?;
        self.key = Some(new_magic_crypt!(password, 256));
        let encrypted_string = self.key.clone().unwrap().encrypt_str_to_base64(json_string);
        fs::write(&self.filename, encrypted_string)?;
        return Ok(());
    }
    //  Below is fishy, probably want to redo it
    pub fn rename_wallet(
        &mut self,
        entry_type: EntryType,
        address: &str,
        wallet_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_wallet_data(
            entry_type,
            address,
            Some(wallet_name.to_string()),
            None,
            None,
        )?;
        return Ok(());
    }

    pub fn sync_wallet(
        &mut self,
        address: &str,
        balance: Option<Balance>,
        transactions: Option<Vec<TransactionDetails>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_wallet_data(EntryType::Wallet, address, None, balance, transactions)?;
        return Ok(());
    }

    pub fn get_first_wallet_xpriv_str(&mut self) -> String {
        let first_wallet = self.json_wallet_data.wallets[0]
            .priv_key
            .clone()
            .unwrap()
            .to_string();
        return first_wallet;
    }

    // pub fn get_wallet_element(&mut self, xpriv_str: &str) -> &mut WalletElement {
    //     // Using entry() to ensure the key exists and get a mutable reference
    //     self.wallets
    //         .entry(xpriv_str.to_string())
    //         .or_insert_with(|| WalletElement::new("", "")) // Create a new WalletElement if key doesn't exist
    // }

    pub fn get_selected_wallet_string(&self) -> String {
        return self.selected_wallet.clone().unwrap();
    }

    pub fn get_selected_wallet(&self) -> Arc<Mutex<Wallet<MemoryDatabase>>> {
        let wallet_string = self.get_selected_wallet_string();
        let wallet = Arc::clone(&self.wallet_objs[&wallet_string]);
        return wallet;
    }
    pub fn get_selected_wallet_data(&self) -> JsonWallet {
        let wallet_string = self.get_selected_wallet_string();
        let wallet_data = self
            .json_wallet_data
            .wallets
            .iter()
            .find(|wallet| wallet.priv_key == Some(wallet_string.clone()))
            .unwrap();
        return wallet_data.clone();
    }

    fn set_wallet_data(
        &mut self,
        entry_type: EntryType,
        address: &str,
        wallet_name: Option<String>,
        balance: Option<Balance>,
        transactions: Option<Vec<TransactionDetails>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut found_index = None;
        match entry_type {
            EntryType::Wallet => {
                found_index = self
                    .json_wallet_data
                    .wallets
                    .iter()
                    .position(|wallet| wallet.priv_key == Some(address.to_string()))
            }
            EntryType::Contact => {
                found_index = self
                    .json_wallet_data
                    .contacts
                    .iter()
                    .position(|wallet| wallet.pub_key == address)
            }
        }
        if let Some(index) = found_index {
            let wallet = &mut self.json_wallet_data.wallets[index];
            if let Some(wallet_name) = wallet_name {
                wallet.wallet_name = wallet_name;
            }

            if let Some(balance) = balance {
                wallet.balance = Some(balance);
            }

            if let Some(transactions) = transactions {
                wallet.sorted_transactions = Some(transactions);
            }
        }

        let json_string = serde_json::to_string(&self.json_wallet_data)?;

        let encrypted_string = self.key.clone().unwrap().encrypt_str_to_base64(json_string);
        fs::write(&self.filename, encrypted_string)?;

        return Ok(());
    }

    pub fn send_transaction(&mut self, recipient_address: &str, amount: u64) {
        let wallet = self.get_selected_wallet();
        let transaction = make_transaction(&wallet.lock().unwrap(), recipient_address, amount);
        self.blockchain.broadcast(&transaction).unwrap();
    }

    pub fn validate_password(&mut self, password: &str) -> bool {
        let mut file = self.get_file();
        let mut encrypted_contents = Vec::new();
        let mc = new_magic_crypt!(password, 256);
        file.read_to_end(&mut encrypted_contents).unwrap();
        let encrypted_string = String::from_utf8(encrypted_contents).unwrap();
        let contents = mc.decrypt_base64_to_string(encrypted_string);
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
