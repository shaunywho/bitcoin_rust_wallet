use bdk::bitcoin::Network;
use bdk::keys::ExtendedKey;
use bincode::{deserialize, serialize};
use csv::ReaderBuilder;
use hex::encode;
use secp256k1;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{Read, Write};

const FILENAME: &str = "./wallet.txt";
pub struct WalletData {
    pub wallets: HashMap<[u8; 32], String>,
    file: File,
}
impl WalletData {
    pub fn new(filename: &str) -> WalletData {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename)
            .unwrap();

        Self {
            wallets: HashMap::new(),
            file: file,
        }
    }
    pub fn read_from_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(self.file);

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

                    self.wallets.insert(priv_key_array, wallet_name.to_string());
                }
            }
        }
        Ok(())
    }

    pub fn add_wallet(&mut self, xkey: ExtendedKey) -> Result<(), Box<dyn std::error::Error>> {
        let priv_key_array = xkey
            .into_xprv(Network::Testnet)
            .unwrap()
            .private_key
            .as_ref()
            .to_owned();

        if self.wallets.contains_key(&priv_key_array) {
            panic!("Wallet already exists");
        } else {
            self.wallets
                .insert(priv_key_array, "New Wallet".to_string());
            self.append_private_key_to_wallet_file(&priv_key_array)?;
        }

        Ok(())
    }

    pub fn append_private_key_to_wallet_file(
        &mut self,
        priv_key: &[u8; 32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Err(e) = writeln!(
            self.file,
            "{}, {}",
            hex::encode(priv_key),
            self.wallets[priv_key]
        ) {
            eprintln!("Couldn't write to file: {}", e);
        }
        Ok(())
    }
}
