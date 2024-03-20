use flex_error::{define_error, TraceError};
use tendermint_rpc::error::Error as TendermintRpcError;

define_error! {
    #[derive(Debug, Clone)]
    WsError {
        ClientSubscriptionFailed
            [ TraceError<TendermintRpcError> ]
            |_| { "failed to run previous WebSocket driver to completion" },
    }
}