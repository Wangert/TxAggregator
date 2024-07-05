use std::str::FromStr;

use bitcoin::bip32::ExtendedPubKey;
use hdpath::StandardHDPath;
use ibc_proto::cosmos;
use log::info;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use types::signer::Signer;
use utils::file::toml_file;

use crate::{
    error::Error,
    keyring::{
        decode_bech32_address, private_key_from_mnemonic, CosmosKey, EncodedPubKey,
        Secp256k1KeyPair,
    },
};

#[derive(Debug, Clone)]
pub struct Secp256k1Account {
    key_pair: Option<Secp256k1KeyPair>,
    address_bytes: Vec<u8>,
    address: String,
}

impl Secp256k1Account {
    pub fn new(key_path: &str, hd_path: &str) -> Result<Self, Error> {
        let cosmos_key: CosmosKey = toml_file::toml_file_read(key_path)
            .map_err(|e| Error::read_cosmos_key(e))
            .expect("toml file error!");

        let address_bytes = decode_bech32_address(&cosmos_key.address)?;

        let encoded_pub_key: EncodedPubKey = cosmos_key.pubkey.parse()?;
        info!("{:?}", encoded_pub_key);

        let mut encoded_pub_key_bytes = encoded_pub_key.key;

        let s_hd_path =
            StandardHDPath::from_str(hd_path).map_err(|_e| Error::hd_path(hd_path.to_string()))?;

        let secp256k1_key_pair = Secp256k1KeyPair::from_mnemonic(&cosmos_key.mnemonic, &s_hd_path)?;
        let derived_pub_key_bytes = secp256k1_key_pair.public_key.serialize().to_vec();
        info!("derived public key bytes: {:?}", derived_pub_key_bytes);

        let encoded_pub_key_bytes = encoded_pub_key_bytes
            .split_off(encoded_pub_key_bytes.len() - derived_pub_key_bytes.len());
        if encoded_pub_key_bytes != derived_pub_key_bytes {
            return Err(Error::public_key_mismatch(
                cosmos_key.pubkey,
                cosmos_key.mnemonic,
            ));
        }

        println!("account address:{}", cosmos_key.address);
        Ok(Self {
            key_pair: Some(secp256k1_key_pair),
            address_bytes,
            address: cosmos_key.address,
        })
    }

    pub fn key_pair(&self) -> Result<Secp256k1KeyPair, Error> {
        self.key_pair.ok_or_else(Error::empty_key_pair)
    }

    pub fn address_bytes_vec(&self) -> Vec<u8> {
        self.address_bytes.clone()
    }

    pub fn address(&self) -> String {
        self.address.clone()
    }

    pub fn message_sign(&self, message: &[u8]) -> Result<Vec<u8>, Error> {
        self.key_pair()?.sign(message)
    }

    pub fn signature_verify(&self, message: &[u8], signature: &[u8]) -> Result<bool, Error> {
        let result = self.key_pair()?.verify(message, signature);
        Ok(result)
    }

    pub fn get_signer(&self) -> Result<Signer, Error> {
        self.address
            .parse()
            .map_err(|e| Error::signer("account parse to signer error".to_string(), e))
    }
}

#[cfg(test)]
pub mod account_tests {
    use crate::chain::CosmosChain;

    use super::Secp256k1Account;

    #[test]
    pub fn account_new_works() {
        let file_path =
            "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let account = Secp256k1Account::new(
            &cosmos_chain.config.chain_key_path,
            &cosmos_chain.config.hd_path,
        );

        match account {
            Ok(a) => println!("Account:{:?}", a),
            Err(e) => println!("{}", e),
        }
    }
}
