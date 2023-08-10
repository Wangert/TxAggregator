use subtle_encoding::hex;

pub fn hex_encode<B: AsRef<[u8]>>(bytes: B) -> Vec<u8> {
    hex::encode(bytes)
}