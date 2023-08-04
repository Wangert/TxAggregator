use std::str::FromStr;

use bip39::{Language, Mnemonic, Seed};
use bitcoin::{
    bip32::{ChildNumber, DerivationPath, ExtendedPrivKey, ExtendedPubKey},
    secp256k1::Secp256k1,
    Network,
};
use derive_more::Display;
use digest::Digest;
use hdpath::StandardHDPath;
use secp256k1::{ecdsa::Signature, Message, PublicKey, SecretKey};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::Sha256;
use subtle_encoding::base64;
use utils::encode::{bech32, protobuf};

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosKey {
    pub name: String,
    pub r#type: String,
    pub address: String,
    pub pubkey: String,
    pub mnemonic: String,
}

//
#[derive(Debug, Deserialize)]
pub struct EncodedPubKey {
    #[serde(alias = "@type")]
    pub r#type: String,
    #[serde(deserialize_with = "deserialize_key")]
    pub key: Vec<u8>,
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

    let base_key = ExtendedPrivKey::new_master(Network::Bitcoin, seed.as_bytes())
        .map_err(|err| Error::bip32_key_generation_failed("Secp256k1".to_string(), err.into()))?;

    let private_key = base_key
        .derive_priv(
            &Secp256k1::new(),
            &standard_path_to_derivation_path(hd_path),
        )
        .map_err(|err| Error::bip32_key_generation_failed("Secp256k1".to_string(), err.into()))?;

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
    bech32::encode(address_prefix, address_bytes)
        .map_err(|e| Error::address_bech32_encode(address_bytes.to_vec(), e))
}

#[derive(Debug, Clone, Copy)]
pub struct Secp256k1KeyPair {
    pub public_key: PublicKey,
    private_key: SecretKey,
}

impl Secp256k1KeyPair {
    pub fn from_mnemonic(mnemonic: &str, hd_path: &StandardHDPath) -> Result<Self, Error> {
        let private_key = private_key_from_mnemonic(mnemonic, hd_path)?;
        let publick_key = ExtendedPubKey::from_priv(&Secp256k1::signing_only(), &private_key);

        Ok(Self {
            public_key: publick_key.public_key,
            private_key: private_key.private_key,
        })
    }

    pub fn public_key_bytes(&self) -> Result<Vec<u8>, Error> {
        protobuf::encode_to_bytes(&self.public_key.serialize().to_vec())
            .map_err(|e| Error::utils_protobuf_encode("secp256l1 public key".to_string(), e))
    }

    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Error> {
        let message_hash = Sha256::digest(message);
        let message = Message::from_slice(&message_hash).unwrap();
        let signature = Secp256k1::signing_only()
            .sign_ecdsa(&message, &self.private_key)
            .serialize_compact()
            .to_vec();

        Ok(signature)
    }

    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        let message_hash = Sha256::digest(message);
        let message = Message::from_slice(&message_hash).unwrap();

        let signature = Signature::from_compact(signature).expect("signature convert error");
        Secp256k1::verification_only()
            .verify_ecdsa(&message, &signature, &self.public_key)
            .is_ok()
    }
}

#[cfg(test)]
pub mod keyring_test {
    use std::str::FromStr;

    use log::{error, info};
    use utils::file::toml_file;

    use crate::{account::Secp256k1Account, chain::CosmosChain, error::Error};

    use super::{decode_bech32_address, encode_bech32_address, CosmosKey, EncodedPubKey};

    #[test]
    pub fn pubkey_from_str_works() {
        let s = "{\"@type\":\"/cosmos.crypto.secp256k1.PubKey\",\"key\":\"AnWi6I8CrOIAS9ee4gsjvBxXwrkEYwUykjoiTrsU5ypg\"}";
        let pk = EncodedPubKey::from_str(s);

        match pk {
            Ok(k) => println!("{:?}", k),
            Err(e) => println!("{}", e),
        }
    }

    #[test]
    pub fn cosmos_key_read_works() {
        let key_path = "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/key_a.toml";
        let cosmos_key: CosmosKey = toml_file::toml_file_read(key_path)
            .map_err(|e| Error::read_cosmos_key(e))
            .expect("toml file error!");

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
        let cosmos_key: CosmosKey = toml_file::toml_file_read(key_path)
            .map_err(|e| Error::read_cosmos_key(e))
            .expect("toml file error!");

        println!("Orignal Address:{:?}", cosmos_key.address);

        let decode_result = decode_bech32_address(&cosmos_key.address);

        let address_bytes = if let Ok(address_bytes) = decode_result {
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

    #[test]
    pub fn sign_and_verify_works() {
        let file_path =
            "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let account = Secp256k1Account::new(
            &cosmos_chain.config.chain_a_key_path,
            &cosmos_chain.config.hd_path,
        )
        .expect("account error!");

        let key_pair = account.key_pair();
        let key_pair = match key_pair {
            Ok(key_pair) => key_pair,
            Err(e) => panic!("{}", e),
        };

        let message = "wangjitao".as_bytes();

        let sig = key_pair.sign(message);
        let sig_bytes = match sig {
            Ok(sig_bytes) => sig_bytes,
            Err(e) => panic!("{}", e),
        };

        println!("Signature: {:?}", sig_bytes);

        let verify_result = key_pair.verify(message, &sig_bytes);

        println!("Verify result:{}", verify_result);
    }
}
