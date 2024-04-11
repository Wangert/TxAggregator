use ibc_proto::{
    cosmos::{
        auth::v1beta1::BaseAccount,
        tx::v1beta1::{
            mode_info::{Single, Sum},
            AuthInfo, Fee, ModeInfo, SignDoc, SignerInfo, Tx, TxBody, TxRaw,
        },
    },
    google::protobuf::Any,
};
use utils::encode::protobuf;

use crate::{account::Secp256k1Account, config::CosmosChainConfig, error::Error};

use super::types::{GasConfig, Memo};

pub fn create_and_sign_tx(
    chain_config: &CosmosChainConfig,
    account_info: &Secp256k1Account,
    account_detail: &BaseAccount,
    tx_memo: &Memo,
    messages: &[Any],
    fee: Option<Fee>,
) -> Result<(Tx, TxRaw), Error> {
    let public_key_bytes = account_info.key_pair()?.public_key_bytes()?;

    let signer_info = cosmos_signer_info(account_detail.sequence, public_key_bytes);

    let tx_body = tx_body(messages, tx_memo, vec![]);
    let tx_body_bytes = tx_body_bytes(&tx_body)?;

    let fee = if let Some(fee) = fee {
        fee
    } else {
        GasConfig::from(chain_config).max_fee
    };

    let auth_info = auth_info(signer_info, fee);
    let auth_info_bytes = auth_info_bytes(&auth_info)?;

    let sign_doc = SignDoc {
        body_bytes: tx_body_bytes.clone(),
        auth_info_bytes: auth_info_bytes.clone(),
        chain_id: chain_config.chain_id.clone(),
        account_number: account_detail.account_number,
    };

    let encoded_sign_doc = protobuf::encode_to_bytes(&sign_doc)
        .map_err(|e| Error::utils_protobuf_encode("sign doc".to_string(), e))?;
    let signature = account_info
        .key_pair()?
        .sign(&encoded_sign_doc)
        .map_err(|_| Error::tx_sign())?;

    let tx = Tx {
        body: Some(tx_body),
        auth_info: Some(auth_info),
        signatures: vec![signature.clone()],
    };

    let tx_raw = TxRaw {
        body_bytes: tx_body_bytes,
        auth_info_bytes,
        signatures: vec![signature],
    };

    Ok((tx, tx_raw))
}

pub fn tx_body(proto_msgs: &[Any], memo: &Memo, extension_options: Vec<Any>) -> TxBody {
    TxBody {
        messages: proto_msgs.to_vec(),
        memo: memo.to_string(),
        timeout_height: 0_u64,
        extension_options,
        non_critical_extension_options: Vec::<Any>::new(),
    }
}

pub fn tx_body_bytes(tx_body: &TxBody) -> Result<Vec<u8>, Error> {
    protobuf::encode_to_bytes(tx_body)
        .map_err(|e| Error::utils_protobuf_encode("tx body".to_string(), e))
}

pub fn cosmos_signer_info(account_sequence: u64, key_bytes: Vec<u8>) -> SignerInfo {
    let public_key = Any {
        type_url: "/cosmos.crypto.secp256k1.PubKey".to_string(),
        value: key_bytes,
    };

    // set signature mode
    let single = Single { mode: 1 };
    let sum_single = Some(Sum::Single(single));
    let mode = Some(ModeInfo { sum: sum_single });

    SignerInfo {
        public_key: Some(public_key),
        mode_info: mode,
        sequence: account_sequence,
    }
}

pub fn cosmos_signer_info_bytes(signer_info: &SignerInfo) -> Result<Vec<u8>, Error> {
    protobuf::encode_to_bytes(signer_info)
        .map_err(|e| Error::utils_protobuf_encode("cosmos signer info".to_string(), e))
}

pub fn auth_info(signer_info: SignerInfo, fee: Fee) -> AuthInfo {
    AuthInfo {
        signer_infos: vec![signer_info],
        fee: Some(fee),
        tip: None,
    }
}

pub fn auth_info_bytes(auth_info: &AuthInfo) -> Result<Vec<u8>, Error> {
    protobuf::encode_to_bytes(auth_info)
        .map_err(|e| Error::utils_protobuf_encode("auth info".to_string(), e))
}
