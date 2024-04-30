use flex_error::{define_error, TraceError};
use tendermint_rpc::error::Error as TendermintRpcError;

define_error! {
    #[derive(Debug, Clone)]
    WsError {
        ClientSubscriptionFailed
            [ TraceError<TendermintRpcError> ]
            |_| { "failed to run previous WebSocket driver to completion" },
        CollectEventsError
            |_| { "collect events error" },
        SubscriptionCancelled
            [ TraceError<TendermintRpcError> ]
            |_| { "subscription cancelled" },
        ClientIsNotExist
            |_| { "client is not exist" },
        DriverHandlerIsNotExist
            |_| { "driver handler is not exist" },
    }
}

impl WsError {
    pub fn canceled_or_generic(e: TendermintRpcError) -> Self {
        use tendermint_rpc::error::ErrorDetail;

        match e.detail() {
            ErrorDetail::Server(detail) if detail.reason.contains("subscription was cancelled") => {
                Self::subscription_cancelled(e)
            }
            _ => Self::collect_events_error(),
        }
    }
}