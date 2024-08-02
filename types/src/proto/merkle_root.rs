use super::aggregate_packet::Packet;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgSetHashValue {
    #[prost(message, tag = "1")]
    pub key: ::core::option::Option<Packet>,
    #[prost(bytes = "vec", tag = "2")]
    pub value: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "3")]
    pub signer: ::prost::alloc::string::String,
}