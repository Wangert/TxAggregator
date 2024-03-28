use tendermint_rpc::query::{EventType, Query};

pub fn new_event_source_query(key: &str, value: &str) -> Query {
    Query::eq(key, value)
}

pub fn all_event_sources() -> Vec<Query> {
    vec![
        new_block(),
        // new_event_source_query("message.module", "ibc_client"),
        // new_event_source_query("message.module", "ibc_client")
        //     .and_eq("message.action", "/ibc.core.client.v1.MsgCreateClient"),
        // new_event_source_query("message.action", "create_client"),
    ]
}

pub fn new_block() -> Query {
    Query::from(EventType::NewBlock)
}

pub fn ibc_client() -> Query {
    Query::eq("message.module", "ibc_client")
}

pub fn ibc_connection() -> Query {
    Query::eq("message.module", "ibc_connection")
}

pub fn ibc_channel() -> Query {
    Query::eq("message.module", "ibc_channel")
}

pub fn ibc_query() -> Query {
    Query::eq("message.module", "interchainquery")
}
