use bdk::bitcoin::bip32::ExtendedPrivKey;

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
use std::sync::mpsc;
use std::thread;

use crate::bitcoin_wallet::generate_wallet;

const FILENAME: &str = "./wallet.txt";

pub struct SyncData {
    pub balance: Balance,
    pub transactions: Vec<TransactionDetails>,
}

pub struct WalletData {
    pub wallets: HashMap<String, WalletElement>,
    filename: String,
}
#[derive(Clone)]
pub struct WalletElement {
    priv_key: String,
    pub wallet_name: String,
    pub address: String,
    pub balance: bdk::Balance,
    pub transactions: Vec<TransactionDetails>,
    // pub wallet_obj: Wallet<MemoryDatabase>,
}

impl WalletElement {
    fn sync(wallet: &Wallet<MemoryDatabase>) {
        let client = Client::new("ssl://electrum.blockstream.info:60002").unwrap();
        let blockchain = ElectrumBlockchain::from(client);
        wallet.sync(&blockchain, SyncOptions::default());
    }
    pub fn new(priv_key: &str, wallet_name: &str) -> Self {
        let wallet = generate_wallet(ExtendedPrivKey::from_str(priv_key).unwrap()).unwrap();
        WalletElement::sync(&wallet);
        let wallet_name = wallet_name.to_string();

        let balance = wallet.get_balance().unwrap();
        let address = wallet
            .get_address(AddressIndex::Peek(0))
            .unwrap()
            .to_string();
        let transactions = wallet.list_transactions(true).unwrap();

        return Self {
            priv_key: priv_key.to_string(),
            wallet_name: wallet_name,
            balance: balance,
            address: address,
            transactions: transactions,
        };
    }

    pub fn start_wallet_syncing_worker(&mut self, sync_data: mpsc::SyncSender<(String, SyncData)>) {
        let priv_key = self.priv_key.clone(); // Clone the necessary data

        thread::spawn(move || {
            let xpriv = ExtendedPrivKey::from_str(&priv_key).unwrap();
            let wallet: Wallet<MemoryDatabase> = generate_wallet(xpriv).unwrap();
            WalletElement::sync(&wallet);
            let balance = wallet.get_balance().unwrap().clone();
            let transactions = wallet.list_transactions(true).unwrap().clone();
            let _ = sync_data.send((
                priv_key,
                SyncData {
                    balance,
                    transactions,
                },
            ));
        });
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

    pub fn new(filename: &str) -> Self {
        let wallet_data = Self {
            wallets: HashMap::new(),
            filename: filename.to_string(),
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
    pub fn get_wallet_element_from_xpriv_str(&mut self, xprv_str: String) -> WalletElement {
        let xprv_str = &xprv_str[..];
        let wallet_element = self.wallets[xprv_str].clone();
        return wallet_element;
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

    pub fn get_wallet_element(&self, xpriv_str: &str) -> WalletElement {
        return self.wallets[xpriv_str].clone();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn working() {
        assert!(true);
    }
}
