use futures::future::InspectErr;
use ics23::{InnerOp, LeafOp};
use prost::Message;
use utils::{crypto, encode::base64::{encode_to_base64_string, u8_to_base64_string}};

use crate::error::Error;

pub const LENGTHOP_NO_PREFIX: i32 = 0;
pub const LENGTHOP_VAR_PROTO: i32 = 1;

pub const HASHOP_NO_HASH: i32 = 0;
pub const HASHOP_SHA256: i32 = 1;

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

pub fn uint64_to_big_endian(v: u64) -> Vec<u8> {
    let mut b: [u8;8] = [0;8];
    b[0] = (v >> 56) as u8;
    b[1] = (v >> 48) as u8;
    b[2] = (v >> 40) as u8;
    b[3] = (v >> 32) as u8;
    b[4] = (v >> 24) as u8;
    b[5] = (v >> 16) as u8;
    b[6] = (v >> 8) as u8;
    b[7] = (v) as u8;

    b.to_vec()
}

pub struct MerkleProofInfo {
    pub leaf_key: Vec<u8>,
    pub leaf_value: Vec<u8>,
    pub leaf_op: LeafOp,
    pub full_path: Vec<InnerOp>,
}

#[cfg(test)]
pub mod merkle_proof_tests {
    use utils::crypto::do_hash;

    use crate::merkle_proof::{do_hash_op, do_length_op, HASHOP_SHA256, LENGTHOP_VAR_PROTO};

    use super::uint64_to_big_endian;

    #[test]
    pub fn do_hash_op_works() {
        let data = "wjt".as_bytes().to_vec();
        println!("data:{:?}", data);

        let hash_result = do_hash_op(HASHOP_SHA256, data);
        match hash_result {
            Ok(h) => println!("Result:{:?}", h),
            Err(e) => eprintln!("{}", e),
        }
    }

    #[test]
    pub fn do_length_op_works() {
        let data = "wjtwcx".as_bytes().to_vec();
        println!("data:{:?}", data);

        let hash_result = do_length_op(LENGTHOP_VAR_PROTO, data);
        match hash_result {
            Ok(h) => println!("Result:{:?}", h),
            Err(e) => eprintln!("{}", e),
        }
    }

    #[test]
    pub fn do_sha256_hash_works() {
        let data = "wangjitao".as_bytes().to_vec();
        println!("data:{:?}", data);

        let hash_result = do_hash(HASHOP_SHA256, data);
        match hash_result {
            Ok(h) => println!("Result:{:?}", h),
            Err(e) => eprintln!("{}", e),
        }
    }

    #[test]
    pub fn do_big_endian_works() {
        let result = uint64_to_big_endian(3456);
        println!("{:?}", result);

        let re_2 = (3456 as u64).to_be_bytes();
        println!("{:?}", re_2.to_vec());
    }

}