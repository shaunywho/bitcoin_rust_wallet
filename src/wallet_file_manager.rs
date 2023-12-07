use bdk::bitcoin::Network;
use bdk::keys::ExtendedKey;

use csv::ReaderBuilder;
use secp256k1;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
const FILENAME: &str = "./wallet.txt";

pub struct WalletData {
    pub wallets: HashMap<String, String>,
    filename: String,
}

pub struct WalletElement {
    secretkey: String,
    name: String,
}
impl WalletData {
    pub fn new(filename: &str) -> WalletData {
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
                let priv_key_str = record[0].trim();
                let wallet_name = record[1].trim();

                if !priv_key_str.is_empty() && !wallet_name.is_empty() {
                    let priv_key_bytes = hex::decode(priv_key_str)?;
                    let mut priv_key_array = [0u8; 32];
                    priv_key_array.copy_from_slice(&priv_key_bytes);

                    self.wallets
                        .insert(hex::encode(priv_key_array), wallet_name.to_string());
                }
            }
            found_record = true;
        }
        Ok(found_record)
    }

    pub fn add_wallet(&mut self, xkey: ExtendedKey) -> Result<(), Box<dyn std::error::Error>> {
        let priv_key_array = xkey
            .into_xprv(Network::Testnet)
            .unwrap()
            .private_key
            .secret_bytes();
        if self.wallets.contains_key(&hex::encode(priv_key_array)) {
            panic!("Wallet already exists");
        } else {
            self.wallets
                .insert(hex::encode(priv_key_array), "New Wallet".to_string());
            self.append_to_wallet_file(&hex::encode(priv_key_array))?;
        }

        Ok(())
    }

    fn append_to_wallet_file(&mut self, priv_key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = self.get_file();
        if let Err(e) = writeln!(file, "{}, {}", priv_key, self.wallets[priv_key]) {
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
        for (priv_key, wallet_name) in &self.wallets {
            if selected_priv_key == priv_key {
                updated_records.push(format!("{}, {}", priv_key, new_wallet_name));
                found_record = true;
            } else {
                updated_records.push(format!("{}, {}", priv_key, wallet_name));
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
}
