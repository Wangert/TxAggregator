use flex_error::define_error;

define_error! {
    IdentifierError {
        // ics24_host identifier
        IdContainSeparator
            { id : String }
            | e | { format_args!("identifier {0} cannot contain separator '/'", e.id) },
        IdInvalidLength
            {
                id: String,
                length: usize,
                min: usize,
                max: usize,
            }
            |e| { format_args!("identifier {0} has invalid length {1} must be between {2}-{3} characters", e.id, e.length, e.min, e.max) },
        IdInvalidCharacter
            { id: String }
            |e| { format_args!("identifier {0} must only contain alphanumeric characters or `.`, `_`, `+`, `-`, `#`, - `[`, `]`, `<`, `>`", e.id) },
        IdEmpty
            |_| { "identifier cannot be empty" },
        HeaderEmpty
            |_| { "empty block header" },
        ChainIdInvalidFormat
            { id: String }
            |e| { format_args!("chain identifiers are expected to be in epoch format {0}", e.id) },
        ClientIdInvalidFormat
            { id: String }
            |e| { format_args!("client identifiers are expected to be in epoch format {0}", e.id) },
        UnknownClientType
            { client_type: String }
            |e| { format_args!("unknown client type: {0}", e.client_type) },
        InvalidCounterpartyChannelId
            |_| { "Invalid channel id in counterparty" },
    }
}