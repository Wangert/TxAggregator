use flex_error::{define_error, TraceError};
use bech32::Error as bech32Error;
use base64::{DecodeError as Base64DecodeError};
use prost::EncodeError as prostEncodeError;
use std::string::FromUtf8Error;
use serde_json::Error as SjError;

define_error! {
    EncodeError {
        Bech32Encode
            [ TraceError<bech32Error> ]
            |_| { "encode bech32 error" },
        Bech32Decode
            [ TraceError<bech32Error> ]
            |_| { "decode bech32 error" },
        ProtobufEncode
            [ TraceError<prostEncodeError> ]
            |_| { "protobuf encode error" },
        Base64Decode
            [ TraceError<Base64DecodeError> ]
            |_| { "decode base64 error" },
        // Base64Encode
        //     [ TraceError<Base64DecodeError> ]
        //     |_| { "decode base64 error" },
        BytesToString
            [ TraceError<FromUtf8Error> ]
            |_| { "bytes to string error" },
        SerdeJsonError
            [ TraceError<SjError> ]
            |_| { "serde json error" },
    }
}