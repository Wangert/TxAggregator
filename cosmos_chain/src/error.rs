use flex_error::{define_error, TraceError};
use tonic::{transport::Error as TransportError, Status as GrpcStatus};
use prost::{DecodeError, EncodeError};

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
        UnknownAccountType
            { type_url: String }
            |e| { format!("failed to deserialize account of an unknown protobuf type: {0}", e.type_url) }
    }
}
