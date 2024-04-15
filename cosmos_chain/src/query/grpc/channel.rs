use ibc_proto::{ibc::core::channel::v1::query_client::QueryClient, Protobuf};
use tendermint_rpc::HttpClient;
use tonic::transport::Channel;
use types::ibc_core::{ics04_channel::channel::ChannelEnd, ics23_commitment::merkle_tree::MerkleProof, ics24_host::{identifier::{ChannelId, PortId}, path::{ChannelEndsPath, ConnectionsPath, IBC_QUERY_PATH}}};

use crate::{common::QueryHeight, error::Error, query::trpc};

pub async fn query_channel(
    trpc_client: &mut HttpClient,
    channel_id: &ChannelId,
    port_id: &PortId,
    height_query: QueryHeight,
    prove: bool,
) -> Result<(ChannelEnd, Option<MerkleProof>), Error> {
        let abci_query = trpc::abci::abci_query(
            trpc_client,
            IBC_QUERY_PATH.to_string(),
            ChannelEndsPath(port_id.clone(), channel_id.clone()).to_string(),
            height_query.into(),
            prove,
        )
        .await?;
        let channel_end = ChannelEnd::decode_vec(&abci_query.value)
            .map_err(|e| Error::tendermint_protobuf_decode("ChannelEnd".to_string(), e))?;

        if prove {
            Ok((
                channel_end,
                Some(
                    abci_query
                        .merkle_proof
                        .ok_or_else(Error::empty_response_proof)?,
                ),
            ))
        } else {
            Ok((
                channel_end,
                None,
            ))
        }
        
}
