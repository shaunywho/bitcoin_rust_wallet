use bdk::bitcoin::bip32::ExtendedPrivKey;
use bdk::bitcoin::Network;
use bdk::database::MemoryDatabase;
use bdk::keys::ExtendedKey;
use bdk::wallet::Wallet;
use csv::ReaderBuilder;

use chrono::{DateTime, Duration, Utc};
use egui::Memory;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::rc::Rc;
use std::str::FromStr;

use crate::bitcoin_wallet::generate_wallet_rc_obj;
const FILENAME: &str = "./wallet.txt";

pub struct WalletData {
    pub wallets: HashMap<String, WalletElement>,
    filename: String,
}

pub struct WalletElement {
    pub wallet_name: String,
    pub wallet_obj: Rc<Wallet<MemoryDatabase>>,
}

impl WalletElement {
    pub fn new(wallet_name: String, wallet_obj: Rc<Wallet<MemoryDatabase>>) -> Self {
        return Self {
            wallet_name: wallet_name,
            wallet_obj: wallet_obj,
        };
    }
}

pub struct WalletValue {
    amount: usize,
    data: Utc,
}
impl WalletData {
    pub fn new(filename: &str) -> Self {
        let mut wallet_data = Self {
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
                    let xpriv = ExtendedPrivKey::from_str(private_key_str)?;
                    self.wallets.insert(
                        private_key_str.to_owned(),
                        WalletElement::new(wallet_name.to_string(), generate_wallet_rc_obj(xpriv)?),
                    );
                }
            }
            found_record = true;
        }
        Ok(found_record)
    }
    pub fn get_wallet_from_xpriv_str(
        &mut self,
        xprv_str: String,
    ) -> Result<Rc<Wallet<MemoryDatabase>>, anyhow::Error> {
        let xprv_str = &xprv_str[..];
        let wallet_element = self.wallets.get_mut(xprv_str).unwrap();
        return Ok(Rc::clone(&wallet_element.wallet_obj));
    }

    pub fn add_wallet(&mut self, xprv: ExtendedPrivKey) -> Result<(), Box<dyn std::error::Error>> {
        let wallet_nameing = xprv.to_string();
        if self.wallets.contains_key(&wallet_nameing) {
            panic!("Wallet already exists");
        } else {
            self.wallets.insert(
                wallet_nameing.clone(),
                WalletElement::new("New Wallet".to_string(), generate_wallet_rc_obj(xprv)?),
            );
            self.append_to_wallet_file(&wallet_nameing)?;
        }

        Ok(())
    }

    fn append_to_wallet_file(&mut self, priv_key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = self.get_file();
        let wallet_element = &self.wallets[priv_key];
        if let Err(e) = writeln!(file, "{}, {}", priv_key, wallet_element.wallet_name) {
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
}

#[cfg(test)]
mod tests {
    #[test]
    fn working() {
        assert!(true);
    }
}
