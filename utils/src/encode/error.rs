use flex_error::{define_error, TraceError};
use bech32::Error as bech32Error;
define_error! {
    EncodeError {
        Bech32Encode
            [ TraceError<bech32Error> ]
            |_| { "encode bech32 error" },
        Bech32Decode
            [ TraceError<bech32Error> ]
            |_| { "decode bech32 error" },
    }
}