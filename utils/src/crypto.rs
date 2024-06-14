use flex_error::define_error;
use sha2::{Digest, Sha256};

const SHA256: i32 = 1;

pub fn do_hash(hash_op: i32, data: Vec<u8>) -> Result<Vec<u8>, CryptoError> {
    match hash_op {
        SHA256 => {
            let mut hasher = Sha256::new();
            hasher.update(data);
            Ok(hasher.finalize().to_vec())
        },
        _ => Err(CryptoError::hash_op_not_exist())
    }
}



define_error! {
    CryptoError {
        HashOpNotExist
            |_| { "hash op is not exist" },
    }
}