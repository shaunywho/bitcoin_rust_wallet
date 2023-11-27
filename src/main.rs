// Copyright (c) 2020-2021 Bitcoin Dev Kit Developers
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.

use anyhow::anyhow;
use bdk::bitcoin::bip32::DerivationPath;
use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::Network;
use bdk::descriptor;
use bdk::descriptor::IntoWalletDescriptor;
use bdk::keys::bip39::{Language, Mnemonic, WordCount};
use bdk::keys::{GeneratableKey, GeneratedKey};
use bdk::miniscript::Tap;
use std::str::FromStr;

/// This example demonstrates how to generate a mnemonic phrase
/// using BDK and use that to generate a descriptor string.
fn descriptor_generation() -> Result<(), anyhow::Error> {
    let secp = Secp256k1::new();

    // In this example we are generating a 12 words mnemonic phrase
    // but it is also possible generate 15, 18, 21 and 24 words
    // using their respective `WordCount` variant.
    let mnemonic: GeneratedKey<_, Tap> =
        Mnemonic::generate((WordCount::Words12, Language::English))
            .map_err(|_| anyhow!("Mnemonic generation error"))?;

    println!("Mnemonic phrase: {}", *mnemonic);
    let mnemonic_with_passphrase = (mnemonic, None);

    // define external and internal derivation key path
    let external_path = DerivationPath::from_str("m/86h/0h/0h/0").unwrap();
    let internal_path = DerivationPath::from_str("m/86h/0h/0h/1").unwrap();

    // generate external and internal descriptor from mnemonic
    let (external_descriptor, ext_keymap) =
        descriptor!(tr((mnemonic_with_passphrase.clone(), external_path)))?
            .into_wallet_descriptor(&secp, Network::Testnet)?;
    let (internal_descriptor, int_keymap) =
        descriptor!(tr((mnemonic_with_passphrase, internal_path)))?
            .into_wallet_descriptor(&secp, Network::Testnet)?;

    println!("tpub external descriptor: {}", external_descriptor);
    println!("tpub internal descriptor: {}", internal_descriptor);
    println!(
        "tprv external descriptor: {}",
        external_descriptor.to_string_with_secret(&ext_keymap)
    );
    println!(
        "tprv internal descriptor: {}",
        internal_descriptor.to_string_with_secret(&int_keymap)
    );

    Ok(())
}

// fn main() -> Result<(), anyhow::Error> {
//     descriptor_generation()
// }

use bdk::bitcoin;
use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client;
use bdk::{SyncOptions, Wallet};

fn sync_wallet() -> Result<(), bdk::Error> {
    let client = Client::new("ssl://electrum.blockstream.info:60002")?;
    let blockchain = ElectrumBlockchain::from(client);
    let wallet = Wallet::new(
        "wpkh([c258d2e4/84h/1h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/0/*)",
        Some("wpkh([c258d2e4/84h/1h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/1/*)"),
        bitcoin::Network::Testnet,
        MemoryDatabase::default(),
    )?;

    wallet.sync(&blockchain, SyncOptions::default())?;

    println!("Descriptor balance: {} SAT", wallet.get_balance()?);

    Ok(())
}

// use bdk::database::MemoryDatabase;
use bdk::wallet::AddressIndex::New;
// use bdk::Wallet;

fn generate_addresses() -> Result<(), bdk::Error> {
    let wallet = Wallet::new(
        "wpkh([c258d2e4/84h/1h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/0/*)",
        Some("wpkh([c258d2e4/84h/1h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/1/*)"),
        bitcoin::Network::Testnet,
        MemoryDatabase::default(),
    )?;

    println!("Address #0: {}", wallet.get_address(New)?);
    println!("Address #1: {}", wallet.get_address(New)?);
    println!("Address #2: {}", wallet.get_address(New)?);

    Ok(())
}

// use bdk::blockchain::ElectrumBlockchain;
// use bdk::database::MemoryDatabase;
// use bdk::electrum_client::Client;
// use bdk::{FeeRate, SyncOptions, Wallet};
use bdk::FeeRate;

// use bdk::wallet::AddressIndex::New;
use bitcoin::consensus::serialize;

fn create_transaction() -> Result<(), bdk::Error> {
    let client = Client::new("ssl://electrum.blockstream.info:60002")?;
    let wallet = Wallet::new(
        "wpkh([c258d2e4/84h/1h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/0/*)",
        Some("wpkh([c258d2e4/84h/1h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/1/*)"),
        bitcoin::Network::Testnet,
        MemoryDatabase::default(),
    )?;
    let blockchain = ElectrumBlockchain::from(client);

    wallet.sync(&blockchain, SyncOptions::default())?;

    let send_to = wallet.get_address(New)?;
    let (psbt, details) = {
        let mut builder = wallet.build_tx();
        builder
            .add_recipient(send_to.script_pubkey(), 50_000)
            .enable_rbf()
            .do_not_spend_change()
            .fee_rate(FeeRate::from_sat_per_vb(5.0));
        builder.finish()?
    };

    println!("Transaction details: {:#?}", details);
    println!("Unsigned PSBT: {}", &psbt);

    Ok(())
}

// use std::str::FromStr;

use bitcoin::psbt::PartiallySignedTransaction as Psbt;

// use bdk::database::MemoryDatabase;
// use bdk::{SignOptions, Wallet};
use bdk::SignOptions;

fn sign_transaction() -> Result<(), bdk::Error> {
    let wallet = Wallet::new(
        "wpkh([c258d2e4/84h/1h/0h]tprv8griRPhA7342zfRyB6CqeKF8CJDXYu5pgnj1cjL1u2ngKcJha5jjTRimG82ABzJQ4MQe71CV54xfn25BbhCNfEGGJZnxvCDQCd6JkbvxW6h/0/*)",
        Some("wpkh([c258d2e4/84h/1h/0h]tprv8griRPhA7342zfRyB6CqeKF8CJDXYu5pgnj1cjL1u2ngKcJha5jjTRimG82ABzJQ4MQe71CV54xfn25BbhCNfEGGJZnxvCDQCd6JkbvxW6h/1/*)"),
        bitcoin::Network::Testnet,
        MemoryDatabase::default(),
    )?;

    let psbt = "...";
    let mut psbt = Psbt::from_str(psbt)?;

    let finalized = wallet.sign(&mut psbt, SignOptions::default())?;

    Ok(())
}

fn main() -> Result<(), bdk::Error> {
    sign_transaction()
}
