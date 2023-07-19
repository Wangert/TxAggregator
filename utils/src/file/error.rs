use flex_error::{define_error, TraceError};
use std::io::Error as IOError;
use toml::de::Error as TomlDeError;

define_error! {
    FileError {
        EmptyQueryAccount
            { address: String }
            |e| { format!("Query/Account RPC returned an empty account for address: {}", e.address) },
        FileReadToString
            [ TraceError<IOError> ]
            |_| { "file read to string error" },
        FileRead
            { file_path: String }
            [ TraceError<IOError> ]
            |e| { format!("file read error: {}", e.file_path) },
        ParseTomlFileFromString
            { file_path: String }
            [ TraceError<TomlDeError> ]
            |e| { format!("failed to deserialize toml file: {}", e.file_path) },
    }
}