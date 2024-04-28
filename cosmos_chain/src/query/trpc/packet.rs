use crate::{common::QueryHeight, error::Error};
use tendermint_rpc::HttpClient;
use types::ibc_core::{
    ics04_channel::packet::Sequence,
    ics23_commitment::merkle_tree::MerkleProof,
    ics24_host::{
        identifier::{ChannelId, PortId},
        path::{AcksPath, CommitmentsPath, IBC_QUERY_PATH},
    },
};

use super::abci;

pub async fn query_packet_commitment(
    trpc_client: &mut HttpClient,
    channel_id: &ChannelId,
    port_id: &PortId,
    sequence: &Sequence,
    height_query: QueryHeight,
    prove: bool,
) -> Result<(Vec<u8>, Option<MerkleProof>), Error> {
    let abci_query = abci::abci_query(
        trpc_client,
        IBC_QUERY_PATH.to_string(),
        CommitmentsPath {
            port_id: port_id.clone(),
            channel_id: channel_id.clone(),
            sequence: sequence.clone(),
        }
        .to_string(),
        height_query.into(),
        prove,
    )
    .await?;

    if prove {
        Ok((
            abci_query.value,
            Some(
                abci_query
                    .merkle_proof
                    .ok_or_else(Error::empty_response_proof)?,
            ),
        ))
    } else {
        Ok((abci_query.value, None))
    }
}

pub async fn query_packet_acknowledgement(
    trpc_client: &mut HttpClient,
    channel_id: &ChannelId,
    port_id: &PortId,
    sequence: &Sequence,
    height_query: QueryHeight,
    prove: bool,
) -> Result<(Vec<u8>, Option<MerkleProof>), Error> {
    let abci_query = abci::abci_query(
        trpc_client,
        IBC_QUERY_PATH.to_string(),
        AcksPath {
            port_id: port_id.clone(),
            channel_id: channel_id.clone(),
            sequence: sequence.clone(),
        }
        .to_string(),
        height_query.into(),
        prove,
    )
    .await?;

    if prove {
        Ok((
            abci_query.value,
            Some(
                abci_query
                    .merkle_proof
                    .ok_or_else(Error::empty_response_proof)?,
            ),
        ))
    } else {
        Ok((abci_query.value, None))
    }
}
