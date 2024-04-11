use std::mem;

use ibc_proto::{
    cosmos::{
        auth::v1beta1::BaseAccount,
        tx::v1beta1::{Fee, TxRaw},
    },
    google::protobuf::Any,
};

use super::{create::create_and_sign_tx, types::Memo};
use crate::{
    account::Secp256k1Account, config::CosmosChainConfig, error::Error, tx::{error::TxError, estimate::gas_to_fee, types::GasConfig},
};
use prost::Message;

/// Length information for an encoded transaction.
pub struct EncodedTxMetrics {
    /// Length of the encoded message, excluding the `body_bytes` field.
    pub envelope_len: usize,
    /// Length of the byte array in the `body_bytes` field of the `TxRaw` message.
    pub body_bytes_len: usize,
}

pub fn encoded_tx_metrics(
    chain_config: &CosmosChainConfig,
    account_info: &Secp256k1Account,
    account_detail: &BaseAccount,
    tx_memo: &Memo,
    messages: &[Any],
    fee: &Fee,
) -> Result<EncodedTxMetrics, Error> {
    let (_, tx_raw) = create_and_sign_tx(
        chain_config,
        account_info,
        account_detail,
        tx_memo,
        messages,
        Some(fee.clone()),
    )?;
    // let signed_tx = sign_tx(config, key_pair, account, tx_memo, messages, fee)?;

    // let tx_raw = TxRaw {
    //     body_bytes: signed_tx.body_bytes,
    //     auth_info_bytes: signed_tx.auth_info_bytes,
    //     signatures: signed_tx.signatures,
    // };

    let total_len = tx_raw.encoded_len();
    let body_bytes_len = tx_raw.body_bytes.len();
    let envelope_len = if body_bytes_len == 0 {
        total_len
    } else {
        total_len - 1 - prost::length_delimiter_len(body_bytes_len) - body_bytes_len
    };

    Ok(EncodedTxMetrics {
        envelope_len,
        body_bytes_len,
    })
}

pub fn batch_messages(
    chain_config: &CosmosChainConfig,
    key_pair: &Secp256k1Account,
    account: &BaseAccount,
    tx_memo: &Memo,
    messages: Vec<Any>,
) -> Result<Vec<Vec<Any>>, Error> {
    let max_message_count = chain_config.max_msg_num as usize;
    let max_tx_size = chain_config.max_tx_size as usize;

    let mut batches = vec![];

    let gas_config = GasConfig::from(chain_config);
    let max_fee = gas_to_fee(
        &gas_config,
        chain_config.max_gas.expect("max_gas error!"),
    );

    let tx_metrics = encoded_tx_metrics(chain_config, key_pair, account, tx_memo, &[], &max_fee)?;
    let tx_envelope_len = tx_metrics.envelope_len;
    let empty_body_len = tx_metrics.body_bytes_len;

    // Full length of the transaction can then be derived from the length of the invariable
    // envelope and the length of the body field, taking into account the varint encoding
    // of the body field's length delimiter.
    fn tx_len(envelope_len: usize, body_len: usize) -> usize {
        // The caller has at least one message field length added to the body's
        debug_assert!(body_len != 0);
        envelope_len + 1 + prost::length_delimiter_len(body_len) + body_len
    }

    let mut current_count = 0;
    let mut current_len = empty_body_len;
    let mut current_batch = vec![];

    for message in messages {
        let message_len = message.encoded_len();

        // The total length the message adds to the encoding includes the
        // field tag (small varint) and the length delimiter.
        let tagged_len = 1 + prost::length_delimiter_len(message_len) + message_len;

        if current_count >= max_message_count
            || tx_len(tx_envelope_len, current_len + tagged_len) > max_tx_size
        {
            let insert_batch = mem::take(&mut current_batch);

            if insert_batch.is_empty() {
                assert!(max_message_count != 0);
                return Err(Error::tx(TxError::message_too_big_for_tx(message_len)));
            }

            batches.push(insert_batch);
            current_count = 0;
            current_len = empty_body_len;
        }

        current_count += 1;
        current_len += tagged_len;
        current_batch.push(message);
    }

    if !current_batch.is_empty() {
        batches.push(current_batch);
    }

    Ok(batches)
}
