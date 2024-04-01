use flex_error::define_error;

define_error! {
    TxError {
        MessageTooBigForTx
            { len: usize }
            |e| {
                format_args!("message with length {} is too large for a transaction", e.len)
            },
    }
}