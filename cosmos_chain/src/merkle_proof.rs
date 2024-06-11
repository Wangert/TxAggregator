use futures::future::InspectErr;
use ics23::{InnerOp, LeafOp};
use prost::Message;
use utils::{crypto, encode::base64::{encode_to_base64_string, u8_to_base64_string}};

use crate::error::Error;

const LENGTHOP_NO_PREFIX: i32 = 0;
const LENGTHOP_VAR_PROTO: i32 = 1;

const HASHOP_NO_HASH: i32 = 0;
const HASHOP_SHA256: i32 = 1;

pub fn calculate_next_step_hash(inner_op: &InnerOp, mut child: Vec<u8>) -> Result<Vec<u8>, Error> {
    if child.len() == 0 {
        return Err(Error::child_is_empty());
    }

    let mut data = inner_op.prefix.clone();
    data.append(&mut child);
    data.append(&mut inner_op.suffix.clone());

    do_hash_op(inner_op.hash, data)
}

pub fn calculate_leaf_hash(leaf_op: LeafOp, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
    if key.len() == 0 || value.len() == 0 {
        return Err(Error::leaf_key_or_value_is_empty());
    }

    let mut pkey = prepare_leaf_hash(leaf_op.prehash_key, leaf_op.length, key)?;
    let mut pvalue = prepare_leaf_hash(leaf_op.prehash_value, leaf_op.length, value)?;

    let mut data = leaf_op.prefix;
    data.append(&mut pkey);
    data.append(&mut pvalue);

    do_hash_op(leaf_op.hash, data)
}

pub fn prepare_leaf_hash(hash_op: i32, length_op: i32, data: Vec<u8>) -> Result<Vec<u8>, Error> {
    let hdata = do_hash_op(hash_op, data)?;
    let ldata = do_length_op(length_op, hdata)?;

    Ok(ldata)
} 

pub fn do_length_op(length_op: i32, mut data: Vec<u8>) -> Result<Vec<u8>, Error> {
    match length_op {
        LENGTHOP_NO_PREFIX => Ok(data),
        LENGTHOP_VAR_PROTO => {
            let mut res = encode_varint_proto(data.len());
            res.append(data.as_mut());
            Ok(res)
        },
        _ => Err(Error::length_op_not_exist())
    }
}

pub fn do_hash_op(hash_op: i32, data: Vec<u8>) -> Result<Vec<u8>, Error> {
    if hash_op == HASHOP_NO_HASH {
        return Ok(data);
    }

    let h = crypto::do_hash(hash_op, data).map_err(Error::crypto_error)?;

    Ok(h)
}

pub fn encode_varint_proto(mut l: usize) -> Vec<u8> {
    let mut res: Vec<u8> = vec![];
    while l >= 1 << 7 {
        res.push((l & 0x7f | 0x80) as u8);
        l = l >> 7;
    }

    res.push(l as u8);
    res
}

pub fn inner_op_to_base64_string(inner_op: &InnerOp) -> String {
    let mut data_vec = inner_op.hash.encode_to_vec();
    data_vec.append(&mut inner_op.prefix.clone());
    data_vec.append(&mut inner_op.suffix.clone());

    u8_to_base64_string(data_vec)
}

pub struct MerkleProofInfo {
    pub leaf_key: Vec<u8>,
    pub leaf_value: Vec<u8>,
    pub leaf_op: LeafOp,
    pub full_path: Vec<InnerOp>,
}