use flex_error::define_error;

define_error! {
    ChannelError {
        MissingChannelId
        | _ | { "missing channel id" },
    }
}