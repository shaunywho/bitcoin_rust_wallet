use bdk::bitcoin::Network;
use bdk::keys::ExtendedKey;
use bincode::{deserialize, serialize};
use hex::encode;
use secp256k1;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{Read, Write};

pub struct WalletData {
    wallets: HashMap<[u8; 32], String>,
}
impl WalletData {
    pub fn read_from_dat_file(&mut self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = std::fs::File::open(filename)?;
        let mut encoded_data = Vec::new();
        file.read(&mut encoded_data)?;

        // self.wallets
        let wallet = bincode::deserialize(&encoded_data)?;
        Ok(())
    }

    pub fn add_wallet(
        &mut self,
        xkey: ExtendedKey,
        filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
            self.add_wallet_to_wallet_dat(&priv_key_array, filename)?;
        }

        Ok(())
    }

    pub fn add_wallet_to_wallet_dat(
        &mut self,
        priv_key: &[u8; 32],
        filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(filename)
            .unwrap();

        if let Err(e) = writeln!(file, "{}", std::str::from_utf8(priv_key)?) {
            eprintln!("Couldn't write to file: {}", e);
        }
        Ok(())
    }
}
