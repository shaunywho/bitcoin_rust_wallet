// Send testnet coin back to https://bitcoinfaucet.uo1.net/send.php

use bdk::bitcoin::bip32::ExtendedPrivKey;
use bdk::bitcoin::Transaction;

use bdk::template::Bip84;
use bdk::{self, BlockTime, KeychainKind, TransactionDetails};
use bdk::{
    bitcoin::Address,
    bitcoin::Network,
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

type TransactionTotal = i64;
type Fee = u64;
type TransactionAddress = String;
type TransactionId = String;
type ConfirmationTime = BlockTime;
pub enum TransactionDirection {
    To,
    From,
}

use std::str::FromStr;

pub fn generate_mnemonic<Ctx>() -> Result<GeneratedKey<Mnemonic, Ctx>, anyhow::Error>
where
    Ctx: ScriptContext,
{
    let mnemonic = Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();
    return Ok(mnemonic);
}

pub fn generate_mnemonic_string() -> Result<String, anyhow::Error> {
    let mnemonic = generate_mnemonic::<bdk::descriptor::Segwitv0>()?;
    return Ok(mnemonic.to_string());
}

pub fn generate_xpriv(mnemonic: &str) -> Result<ExtendedPrivKey, KeyError> {
    let mnemonic = Mnemonic::parse(mnemonic).unwrap();
    // Generate the extended key
    let xkey: ExtendedKey = mnemonic.into_extended_key()?;
    // Get xprv from the extended key
    let xprv = xkey.into_xprv(Network::Testnet).unwrap();
    return Ok(xprv);
}

pub fn generate_wallet(priv_key: &str) -> Result<Wallet<MemoryDatabase>, anyhow::Error> {
    let xpriv = ExtendedPrivKey::from_str(priv_key).unwrap();
    let wallet = Wallet::new(
        Bip84(xpriv.clone(), KeychainKind::External),
        Some(Bip84(xpriv, KeychainKind::Internal)),
        Network::Testnet,
        MemoryDatabase::new(),
    )?;
    return Ok(wallet);
}

pub fn is_valid_bitcoin_address(address: &str) -> bool {
    if let Ok(addr) = Address::from_str(address) {
        let network = Network::Testnet;

        return addr.is_valid_for_network(network);
    } else {
        return false;
    }
}

pub fn make_transaction(
    wallet: &Wallet<MemoryDatabase>,
    recipient_str: &str,
    amount: u64,
) -> Transaction {
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
    return psbt.extract_tx();
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

    let transaction = wallet.list_transactions(true).unwrap()[0]
        .transaction
        .clone()
        .unwrap();
    println!("{:?}", transaction);

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
pub fn extract_address_from_transaction(transaction: &Transaction) -> Vec<Address> {
    transaction
        .output
        .iter()
        .map(|output| Address::from_script(&output.script_pubkey, Network::Testnet).unwrap())
        .collect()
}

pub fn get_transaction_details(
    transaction_details: TransactionDetails,
) -> (
    TransactionDirection,
    TransactionAddress,
    TransactionId,
    TransactionTotal,
    Fee,
    Option<ConfirmationTime>,
) {
    let transaction_total = transaction_details.received as i64 - transaction_details.sent as i64;
    let transaction = transaction_details.transaction.unwrap();
    let transaction_id = transaction_details.txid.to_string();
    let addresses = extract_address_from_transaction(&transaction.clone());
    let address_index = if transaction_total < 0 { 0 } else { 1 };
    let transaction_address = addresses[address_index].to_string();
    let fee = transaction_details.fee.unwrap();
    let confirmation_time = transaction_details.confirmation_time;
    let transaction_direction = if transaction_total < 0 {
        TransactionDirection::To
    } else {
        TransactionDirection::From
    };
    return (
        transaction_direction,
        transaction_address,
        transaction_id,
        transaction_total,
        fee,
        confirmation_time,
    );
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bdk::bitcoin::bip32::ExtendedPrivKey;

    use crate::bitcoin_wallet::generate_xpriv;

    #[test]
    fn test_generating_wallet() {
        // from mnemonic
        let mnemonic_0 =
            "limb capital decade way negative task moral empty virus fragile copper elegant";
        let _mnemonic_1 = &String::from(mnemonic_0)[..];

        let xkey_1 = generate_xpriv(mnemonic_0).unwrap();

        let xpriv = xkey_1;
        let xpriv_str = xpriv.to_string();
        println!("{}", &xpriv_str);
        let xpriv_1 = ExtendedPrivKey::from_str(&xpriv_str[..]).unwrap();
        let xpriv_str_1 = xpriv_1.to_string();
        println!("{}", &xpriv_str_1);
    }
}
