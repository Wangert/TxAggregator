use flex_error::{define_error, TraceError, DisplayOnly};
use tonic::{transport::Error as TransportError, Status as GrpcStatus};
use prost::{DecodeError, EncodeError};
use types::error::TypesError;
use std::io::Error as IOError;
use utils::file::error::FileError;
use tendermint_rpc::error::Error as TrpcError;
use serde_json::Error as SerdeJsonError;
use utils::encode::error::EncodeError as UtilsEncodeError;
use crate::tx::types::MEMO_MAX_LEN;

define_error! {
    Error {
        EmptyQueryAccount
            { address: String }
            |e| { format!("Query/Account RPC returned an empty account for address: {}", e.address) },
        GrpcStatus
            { status: GrpcStatus, query: String}
            |e| { format!("gRPC call `{}` failed with status: {1}", e.query, e.status) },
        GrpcTransport
            [ TraceError<TransportError> ]
            |_| { "error in underlying transport when making gRPC call" },
        CosmosParams
            { payload_type: String }
            |e| { format!("query cosmos params error: {}", e.payload_type) },
        Trpc
            { payload_type: String }
            [ TraceError<TrpcError> ]
            |e| { format!("tendermint rpc error: {}", e.payload_type) },
        ProtobufDecode
            { payload_type: String }
            [ TraceError<DecodeError> ]
            |e| { format!("error decoding protocol buffer for {}", e.payload_type) }, 
        ProtobufEncode
            { payload_type: String }
            [ TraceError<EncodeError> ]
            |e| { format!("error encoding protocol buffer for {}", e.payload_type) },
        EmptyBaseAccount
            |_| { "empty BaseAccount within EthAccount" },
        NoAccounts
            |_| { "No accounts" },
        UnknownAccountType
            { type_url: String }
            |e| { format!("failed to deserialize account of an unknown protobuf type: {0}", e.type_url) },
        LoadCosmosChainConifg
            [ TraceError<FileError> ]
            |_| { "Load cosmos chain config error" },
        EmptyGrpcClient
            { payload_type: String }
            |e| { format!("empty cosmos grpc client: {}", e.payload_type) },
        EmptyTendermintRpcClient
            |_| { "empty cosmos tendermint rpc client" },

        AbciInfo
            [ TraceError<TrpcError> ]
            |_| { "query abci information error" },
        LatestBlock
            [ TraceError<TrpcError> ]
            |_| { "query latest block error" },
        LatestBlockResults
            [ TraceError<TrpcError> ]
            |_| { "query latest block results error" },
        BlockHeight
            { payload_type: String }
            [ TraceError<TypesError> ]
            |e| { format!("block height error: {}", e.payload_type) },
        ClientState
            { payload_type: String }
            [ TraceError<TypesError> ]
            |e| { format!("client state error: {}", e.payload_type) },
        // keyring error
        EncodedPublicKey
            [ TraceError<SerdeJsonError> ]
            |_| { "encode public key error" },
        ReadCosmosKey
            [ TraceError<FileError> ]
            |_| { "read cosmos key error" },
        AddressBech32Decode
            { address: String }
            [ TraceError<UtilsEncodeError> ]
            |e| { format!("address {} bech32 decode error", e.address) },
        AddressBech32Encode
            { address_bytes: Vec<u8> }
            [ TraceError<UtilsEncodeError> ]
            |e| { format!("address {:?} bech32 encode error", e.address_bytes) },
        InvalidMnemonic
            [ DisplayOnly<anyhow::Error> ]
            |_| { "invalid mnemonic" },
        Bip32KeyGenerationFailed
            { key_type: String }
            [ TraceError<anyhow::Error> ]
            |e| { format!("cannot generate {} private key from BIP-32 seed", e.key_type) },
        UtilsProtobufEncode
            { payload_type: String }
            [ TraceError<UtilsEncodeError> ]
            |e| { format!("error encoding protocol buffer for {}", e.payload_type) },
        
        // account
        HdPath
            { hd_path: String }
            |e| {format!("invalid derivation path: {}", e.hd_path) },
        PublicKeyMismatch
            { cosmos_key_pk: String, mnemonic: String }
            |e| { format!("mismatch between the public key {:?} in the cosmos key and the public key in the mnemonic {:?}",  e.cosmos_key_pk, e.mnemonic) },
        EmptyKeyPair
            |_| { "empty key pair" },

        // estimate
        TxProtobufEncode
            [ TraceError<UtilsEncodeError> ]
            |_| { "tx protobuf encode error" },
        SimulateTxGas
            |_| { "tx simulation no gas amount used was retured" },
        TxSign
            |_| { "tx signature error" }
    }
}

flex_error::define_error! {
    MemoError {
        TooLong
            { length: usize }
            |e| {
                format_args!("`memo` must been no longer than {} characters, found length {}",
                    MEMO_MAX_LEN, e.length)
            }
    }
}

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    SignerError {
        EmptySigner
            | _ | { "signer cannot be empty" },
    }
}
