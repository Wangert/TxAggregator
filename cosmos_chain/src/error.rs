use crate::tx::error::TxError;
use crate::tx::types::MEMO_MAX_LEN;
use flex_error::{define_error, DisplayOnly, TraceError};
use prost::{DecodeError, EncodeError};
use serde_json::Error as SerdeJsonError;
use tendermint_light_client::components::io::IoError as LightClientIoError;
use tendermint_light_client::errors::Error as LightClientError;
use tendermint_proto::Error as TendermintProtoError;
use tendermint_rpc::endpoint::abci_query::AbciQuery;
use tendermint_rpc::error::Error as TrpcError;
use tonic::metadata::errors::InvalidMetadataValue;
use tonic::{transport::Error as TransportError, Status as GrpcStatus};
use types::error::TypesError;
use types::ibc_core::ics03_connection::error::ConnectionError;
use types::ibc_core::ics04_channel::error::ChannelError;
use types::ibc_core::ics23_commitment::error::CommitmentError;
use types::ibc_core::ics24_host::error::IdentifierError;
use types::ibc_core::ics24_host::identifier::ConnectionId;
use types::ibc_events::IbcEvent;
use types::proofs::ProofError;
use types::signer::SignerError;
use utils::encode::error::EncodeError as UtilsEncodeError;
use utils::file::error::FileError;
use utils::crypto::CryptoError;
// use tendermint_proto::Error as ProtobufError;

define_error! {
    Error {
        MissingSmallerTrustedHeight
            |e| {"missing trusted state smaller than target height"},
        TxResponse
            { event: String }
            |e| {
                format!("tx response event consists of an error: {}",
                    e.event)
            },
        InvalidEvent
            { event: IbcEvent }
            |e| {
                format!("a connection object cannot be built from {}",
                    e.event)
            },
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
        TendermintProtobufDecode
            { payload_type: String }
            [ TraceError<TendermintProtoError> ]
            |e| { format!("Tendermint protobuf decode error: {}", e.payload_type) },
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
        AbciQuery
            { query: AbciQuery, payload: String}
            |e| { format!("ABCI query returned an error: {:?} => details: {:?}", e.query, e.payload) },
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
        QueryTrustedHeight
            { payload_type: String }
            |e| { format!("query trusted height error: {}", e.payload_type) },
        ClientState
            { payload_type: String }
            [ TraceError<TypesError> ]
            |e| { format!("client state error: {}", e.payload_type) },
        InvalidClientState
            { payload_type: String }
            |e| { format!("Invalid client state: {}", e.payload_type) },
        ExpiredClientState
            |_| { "client state has expire" },
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
        UtilsEncodeError
            { payload_type: String }
            [ TraceError<UtilsEncodeError> ]
            |e| { format!("error encoding for {}", e.payload_type) },
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
            |_| { "tx signature error" },
        FetchLightBlock
            [ TraceError<LightClientIoError> ]
            |_| { "light client fetch light block error" },
        LightClientVerifyBlock
            [ TraceError<LightClientError> ]
            |_| { "light client verify a block with height error" },

        Signer
            { payload: String }
            [ TraceError<SignerError> ]
            |e| { format!("Signer error: {}", e.payload) },
        TxHash
            |_| { "tx hash convert error" },
        TxCommit
            { payload: String }
            |e| { format!("tx commit error: {}", e.payload) },

        IbcEvent
            { payload: String }
            [ TraceError<TypesError> ]
            |e| { format!("ibc event error: {}", e.payload) },

        // connection
        MissingConnectionInitEvent
            |_| { "missing connection openinit event" },
        ConnectionError
            [ TraceError<ConnectionError> ]
            |e| { format!("connection error: {}", e) },
        ConnectionNotFound
            { connection_id: ConnectionId }
            |e| { format!("connection not found: {0}", e.connection_id) },
        EmptyConnectionId
            |_| { "empty connection id" },
        HandshakeContinue
            |_| { "continue handshake" },
        ConnectionCompleted
            |_| { "connection completed" },
        ConnectionStateError
            |_| { "connectuon state error" },
        BadConnectionState
            |_| { "bad connection state" },
        ConnectionHandshkeAbnormal
            |_| { "connection handshake abnormal" },

        // channel
        ChannelCompleted
            |_| { "channel completed" },
        ChannelHandshkeAbnormal
            |_| { "channel handshake abnormal" },
        ChannelError
            [ TraceError<ChannelError> ]
            |e| { format!("channel error: {}", e) },

        // tx
        Tx
            [ TraceError<TxError> ]
            |e| { format!("tx error: {}", e) },

        // memo
        Memo
            [ TraceError<MemoError> ]
            |e| { format!("memo error: {}", e) },

        // commitment
        CommitmentError
            [ TraceError<CommitmentError> ]
            |e| { format!("commitment error: {}", e) },

        InvalidMetadata
            [ TraceError<InvalidMetadataValue> ]
            |_| { "invalid metadata" },
        EmptyResponseProof
            |_| { "empty response proof" },

        // proof error
        ProofError
            [ TraceError<ProofError> ]
            |e| { format!("proof error: {}", e) },

        // type error
        TypeError
            [ TraceError<TypesError> ]
            |e| { format!("type error: {}", e) },
        IdentifierError
            [ TraceError<IdentifierError> ]
            |e| { format!("identifier error: {}", e) },

        EmptyChainId
            |_| { "empty chain id" },
        EmptyClientId
            |_| { "empty client id" },
        EmptyPortId
            |_| { "empty port id" },
        EmptyChannelVersion
            |_| { "empty channel version" },
        EmptyChannel
            |_| { "empty channel" },
        EmptyClientType
            |_| { "empty client type" },
        ClientTypeNotExist
            |_| { "client type not exist" },
        ModeNotExist
            |_| { "mode not exist" },    
        GroupingTypeNotExist
            |_| { "grouping type not exist" },    

        CryptoError
            [ TraceError<CryptoError> ]
            |e| { format!("crypto error: {}", e) },
        LengthOpNotExist
            |_| { "length op is not exist" },
        LeafKeyOrValueIsEmpty
            |_| { "leaf key or value is empty" },
        ChildIsEmpty
            |_| { "child is empty" },
        CreateClient
            |_| { "create client error" },    }
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
