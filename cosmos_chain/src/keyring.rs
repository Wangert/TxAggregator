use std::str::FromStr;

use bip39::{Mnemonic, Language, Seed};
use bitcoin::{bip32::{ExtendedPrivKey, ChildNumber, DerivationPath}, Network, secp256k1::Secp256k1};
use hdpath::StandardHDPath;
use serde::{Serialize, Deserialize, Deserializer};
use subtle_encoding::base64;
use utils::encode::bech32;

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosKey {
    name: String,
    r#type: String,
    address: String,
    pubkey: String,
    mnemonic: String,
}

// 
#[derive(Debug, Deserialize)]
pub struct EncodedPubKey {
    #[serde(alias = "@type")]
    r#type: String,
    #[serde(deserialize_with = "deserialize_key")]
    key: Vec<u8>,
}

/// This method is the workhorse for deserializing
/// the `key` field from a public key.
fn deserialize_key<'de, D>(deser: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    // The key is a byte array that is base64-encoded
    // and then marshalled into a JSON String.
    let based64_encoded: Result<String, _> = Deserialize::deserialize(deser);
    let value = base64::decode(based64_encoded?)
        .map_err(|e| serde::de::Error::custom(format!("error in decoding: {e}")))?;

    Ok(value)
}

impl FromStr for EncodedPubKey {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try to deserialize into a JSON Value.
        let maybe_json: Result<EncodedPubKey, _> = serde_json::from_str(s);
        maybe_json.map_err(|e| Error::encoded_public_key(e))
    }
}

pub fn private_key_from_mnemonic(
    mnemonic_words: &str,
    hd_path: &StandardHDPath,
) -> Result<ExtendedPrivKey, Error> {
    let mnemonic = Mnemonic::from_phrase(mnemonic_words, Language::English)
        .map_err(Error::invalid_mnemonic)?;

    let seed = Seed::new(&mnemonic, "");

    let base_key =
        ExtendedPrivKey::new_master(Network::Bitcoin, seed.as_bytes()).map_err(|err| {
            Error::bip32_key_generation_failed("Secp256k1".to_string(), err.into())
        })?;

    let private_key = base_key
        .derive_priv(
            &Secp256k1::new(),
            &standard_path_to_derivation_path(hd_path),
        )
        .map_err(|err| {
            Error::bip32_key_generation_failed("Secp256k1".to_string(), err.into())
        })?;

    Ok(private_key)
}

fn standard_path_to_derivation_path(path: &StandardHDPath) -> DerivationPath {
    let child_numbers = vec![
        ChildNumber::from_hardened_idx(path.purpose().as_value().as_number())
            .expect("Purpose is not Hardened"),
        ChildNumber::from_hardened_idx(path.coin_type()).expect("Coin Type is not Hardened"),
        ChildNumber::from_hardened_idx(path.account()).expect("Account is not Hardened"),
        ChildNumber::from_normal_idx(path.change()).expect("Change is Hardened"),
        ChildNumber::from_normal_idx(path.index()).expect("Index is Hardened"),
    ];

    DerivationPath::from(child_numbers)
}

pub fn decode_bech32_address(address: &str) -> Result<Vec<u8>, Error> {
    bech32::decode(address).map_err(|e| Error::address_bech32_decode(address.to_string(), e))
}

pub fn encode_bech32_address(address_prefix: &str, address_bytes: &[u8]) -> Result<String, Error> {
    bech32::encode(address_prefix, address_bytes).map_err(|e| Error::address_bech32_encode(address_bytes.to_vec(), e))
}


#[cfg(test)]
pub mod keyring_test {
    use std::str::FromStr;

    use log::{info, error};
    use utils::file::toml_file;

    use crate::error::Error;

    use super::{EncodedPubKey, CosmosKey, decode_bech32_address, encode_bech32_address};

    #[test]
    pub fn pubkey_from_str_works() {
        let s = "{\"@type\":\"/cosmos.crypto.secp256k1.PubKey\",\"key\":\"AnWi6I8CrOIAS9ee4gsjvBxXwrkEYwUykjoiTrsU5ypg\"}";
        let pk = EncodedPubKey::from_str(s);

        match pk {
            Ok(k) => println!("{:?}", k),
            Err(e) => println!("{}", e)
        }
    }

    #[test]
    pub fn cosmos_key_read_works() {
        let key_path = "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/key_a.toml";
        let cosmos_key: CosmosKey = toml_file::toml_file_read(key_path).map_err(|e| Error::read_cosmos_key(e)).expect("toml file error!");

        println!("{:#?}", cosmos_key);

        let pk = EncodedPubKey::from_str(cosmos_key.pubkey.as_str());
        match pk {
            Ok(k) => println!("{:?}", k),
            Err(e) => println!("{}", e),
        }
    }

    #[test]
    pub fn address_bytes_works() {
        let key_path = "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/key_a.toml";
        let cosmos_key: CosmosKey = toml_file::toml_file_read(key_path).map_err(|e| Error::read_cosmos_key(e)).expect("toml file error!");

        println!("Orignal Address:{:?}", cosmos_key.address);

        let decode_result = decode_bech32_address(&cosmos_key.address);
        
        let address_bytes = if let Ok(address_bytes) = decode_result  {
            println!("{:?}", address_bytes);
            address_bytes
        } else {
            println!("Error!");
            return;
        };
       

        let encode_result = encode_bech32_address("cosmos", address_bytes.as_slice());
        match encode_result {
            Ok(address) => println!("{:?}", address),
            Err(e) => println!("{}", e),
        }
    }
}