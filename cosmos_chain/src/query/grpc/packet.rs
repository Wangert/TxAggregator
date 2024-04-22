use ibc_proto::ibc::core::channel::v1::{query_client::QueryClient, QueryUnreceivedPacketsRequest};
use tonic::transport::Channel;
use types::ibc_core::{
    ics04_channel::packet::Sequence,
    ics24_host::identifier::{ChannelId, PortId},
};

use crate::error::Error;

pub async fn query_unreceived_packets(
    grpc_client: &mut QueryClient<Channel>,
    port_id: PortId,
    channel_id: ChannelId,
    sequences: Vec<Sequence>,
) -> Result<Vec<Sequence>, Error> {
    let request = QueryUnreceivedPacketsRequest {
        port_id: port_id.to_string(),
        channel_id: channel_id.to_string(),
        packet_commitment_sequences: sequences.into_iter().map(|s| s.into()).collect(),
    };

    let mut response = grpc_client
        .unreceived_packets(request)
        .await
        .map_err(|e| Error::grpc_status(e, "query unreceived packets".to_string()))?.into_inner();

    response.sequences.sort_unstable();

    Ok(response.sequences.into_iter().map(|s| s.into()).collect())

}
