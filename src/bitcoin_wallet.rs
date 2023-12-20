// Copyright (c) 2020-2021 Bitcoin Dev Kit Developers
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.
// Send testnet coin back to https://bitcoinfaucet.uo1.net/send.php

use bdk::bitcoin::bip32::ExtendedPrivKey;

use bdk::template::Bip84;
use bdk::{self, KeychainKind};
use bdk::{
    bitcoin::Address,
    bitcoin::Network,
    blockchain::Blockchain,
    blockchain::ElectrumBlockchain,
    database::MemoryDatabase,
    electrum_client::Client,
    keys::{
        bip39::{Language, Mnemonic, WordCount},
        DerivableKey, ExtendedKey, GeneratableKey, GeneratedKey, KeyError,
    },
    miniscript::ScriptContext,
    wallet::{AddressIndex, Wallet},
    SignOptions, SyncOptions,
};

use std::rc::Rc;
use std::str::FromStr;

pub fn generate_mnemonic<Ctx>() -> Result<GeneratedKey<Mnemonic, Ctx>, anyhow::Error>
where
    Ctx: ScriptContext,
{
    let mnemonic = Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();
    Ok(mnemonic)
}

pub fn generate_mnemonic_string() -> Result<String, anyhow::Error> {
    let mnemonic = generate_mnemonic::<bdk::descriptor::Segwitv0>()?;
    Ok(mnemonic.to_string())
}

pub fn generate_xpriv(mnemonic: &str) -> Result<ExtendedPrivKey, KeyError> {
    let mnemonic = Mnemonic::parse(mnemonic).unwrap();
    // Generate the extended key
    let xkey: ExtendedKey = mnemonic.into_extended_key()?;
    // Get xprv from the extended key
    let xprv = xkey.into_xprv(Network::Testnet).unwrap();
    Ok(xprv)
}

pub fn get_transactions(wallet: &Wallet<MemoryDatabase>) {
    let client = Client::new("ssl://electrum.blockstream.info:60002").unwrap();
    let blockchain = ElectrumBlockchain::from(client);
    wallet.sync(&blockchain, SyncOptions::default());
    println!("{:?}", wallet.list_transactions(true).unwrap());
}

pub fn get_balance(wallet: &Wallet<MemoryDatabase>) -> u64 {
    let client = Client::new("ssl://electrum.blockstream.info:60002").unwrap();
    let blockchain = ElectrumBlockchain::from(client);
    wallet.sync(&blockchain, SyncOptions::default());
    let balance = wallet.get_balance().unwrap();

    return balance.confirmed;
}

pub fn generate_wallet(xprv: ExtendedPrivKey) -> Result<Wallet<MemoryDatabase>, anyhow::Error> {
    let wallet = Wallet::new(
        Bip84(xprv.clone(), KeychainKind::External),
        Some(Bip84(xprv, KeychainKind::Internal)),
        Network::Testnet,
        MemoryDatabase::new(),
    )?;
    return Ok(wallet);
}

pub fn is_valid_bitcoin_address(address: &str) -> bool {
    if let Ok(addr) = Address::from_str(address) {
        // You can also specify the Bitcoin network (mainnet, testnet, etc.)
        // For example, let network = Network::Bitcoin; or Network::Testnet
        let network = Network::Testnet;

        // Check if the address is valid for the specified network
        addr.is_valid_for_network(network)
    } else {
        false // Parsing failed, so the address is not valid
    }
}

pub fn generate_wallet_rc_obj(
    xprv: ExtendedPrivKey,
) -> Result<Rc<Wallet<MemoryDatabase>>, anyhow::Error> {
    let wallet = generate_wallet(xprv);
    return Ok(Rc::new(wallet?));
}

pub fn make_transaction(wallet: &Wallet<MemoryDatabase>, recipient_str: &str, amount: u64) {
    let balance = wallet.get_balance().unwrap();
    println!("Available balance: {}", balance);
    let recipient_address = Address::from_str(recipient_str)
        .unwrap()
        .require_network(Network::Testnet)
        .unwrap();
    let mut tx_builder = wallet.build_tx();
    tx_builder
        .add_recipient(
            recipient_address.script_pubkey(),
            // (balance.trusted_pending + balance.confirmed) / 2,
            amount,
        )
        .enable_rbf();
    println!("{:?}", tx_builder);
    let (mut psbt, _tx_details) = tx_builder.finish().unwrap();

    let _finalized = wallet.sign(&mut psbt, SignOptions::default()).unwrap();
}

pub fn bitcoin_test() -> Result<(), Box<dyn std::error::Error>> {
    let external_descriptor = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/0'/0'/0/*)";
    let internal_descriptor = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/0'/0'/1/*)";
    let client = Client::new("ssl://electrum.blockstream.info:60002")?;
    let blockchain = ElectrumBlockchain::from(client);
    let wallet: Wallet<MemoryDatabase> = Wallet::new(
        external_descriptor,
        Some(internal_descriptor),
        Network::Testnet,
        MemoryDatabase::new(),
    )?;

    let address = wallet.get_address(AddressIndex::New)?;

    wallet.sync(&blockchain, SyncOptions::default())?;
    println!("\n\n\n\n\n");
    println!("{:#?}", wallet.list_transactions(true).unwrap());
    println!("\n\n\n\n\n");

    println!("Generated Address: {}", address);
    let balance = wallet.get_balance()?;
    println!("Wallet balance in SAT: {}", balance);

    let faucet_address = Address::from_str("tb1qw2c3lxufxqe2x9s4rdzh65tpf4d7fssjgh8nv6")
        .unwrap()
        .require_network(Network::Testnet)?;

    let mut tx_builder = wallet.build_tx();
    tx_builder
        .add_recipient(
            faucet_address.script_pubkey(),
            // (balance.trusted_pending + balance.confirmed) / 2,
            7000,
        )
        .enable_rbf();
    let (mut psbt, tx_details) = tx_builder.finish().unwrap();

    let finalized = wallet.sign(&mut psbt, SignOptions::default())?;
    assert!(finalized, "Tx has not been finalized");
    println!("Transaction Signed: {}", finalized);
    println!("balance.trusted_pending: {}", balance.trusted_pending);
    println!("balance.confirmed: {}", balance.confirmed);
    println!("Transaction details: {:#?}", tx_details);

    let finalized = wallet.sign(&mut psbt, SignOptions::default())?;
    assert!(finalized, "Tx has not been finalized");
    println!("Transaction Signed: {}", finalized);

    let raw_transaction = psbt.extract_tx();
    let txid = raw_transaction.txid();
    // blockchain.broadcast(&raw_transaction)?;
    println!(
        "Transaction sent! TXID: {txid}.\nExplorer URL: https://blockstream.info/testnet/tx/{txid}",
        txid = txid
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bdk::bitcoin::bip32::ExtendedPrivKey;

    use crate::bitcoin_wallet::{generate_wallet_rc_obj, generate_xpriv};

    #[test]
    fn test_generating_wallet() {
        // from mnemonic
        let mnemonic_0 =
            "limb capital decade way negative task moral empty virus fragile copper elegant";
        let _mnemonic_1 = &String::from(mnemonic_0)[..];
        let xkey_0 = generate_xpriv(mnemonic_0).unwrap();
        let xkey_1 = generate_xpriv(mnemonic_0.clone()).unwrap();
        let _wallet_0 = generate_wallet_rc_obj(xkey_0).unwrap();

        let xpriv = xkey_1;
        let xpriv_str = xpriv.to_string();
        println!("{}", &xpriv_str);
        let xpriv_1 = ExtendedPrivKey::from_str(&xpriv_str[..]).unwrap();
        let xpriv_str_1 = xpriv_1.to_string();
        println!("{}", &xpriv_str_1);
    }
}
