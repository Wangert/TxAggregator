use ibc_proto::{
    cosmos::base::query::v1beta1::PageRequest as IbcPageRequest,
    google::protobuf::Duration as ProtobufDuration,
    ibc::core::client::v1::query_client::QueryClient as IbcClientQueryClient,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tendermint::block::Height as TmBlockHeight;
use tendermint_rpc::HttpClient;
use tonic::transport::Channel;
use types::{
    ibc_core::{
        ics02_client::height::Height,
        ics24_host::identifier::{ChainId, ClientId},
    },
    light_clients::ics07_tendermint::client_state::ClientState,
};

use crate::{
    error::Error,
    query::{grpc, trpc},
};

pub fn parse_protobuf_duration(duration: ProtobufDuration) -> Duration {
    Duration::new(duration.seconds as u64, duration.nanos as u32)
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum QueryHeight {
    Latest,
    Specific(Height),
}

impl From<QueryHeight> for TmBlockHeight {
    fn from(height_query: QueryHeight) -> Self {
        match height_query {
            QueryHeight::Latest => Self::from(0_u32),
            QueryHeight::Specific(height) => Self::from(height),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PageRequest {
    /// key is a value returned in PageResponse.next_key to begin
    /// querying the next page most efficiently. Only one of offset or key
    /// should be set.
    pub key: ::prost::alloc::vec::Vec<u8>,
    /// offset is a numeric offset that can be used when key is unavailable.
    /// It is less efficient than using key. Only one of offset or key should
    /// be set.
    pub offset: u64,
    /// limit is the total number of results to be returned in the result page.
    /// If left empty it will default to a value to be set by each app.
    pub limit: u64,
    /// count_total is set to true  to indicate that the result set should include
    /// a count of the total number of items available for pagination in UIs.
    /// count_total is only respected when offset is used. It is ignored when key
    /// is set.
    pub count_total: bool,
    /// reverse is set to true if results are to be returned in the descending order.
    pub reverse: bool,
}

impl PageRequest {
    pub fn all() -> Self {
        // Note: do not use u64::MAX as the limit, as it may have unintended consequences
        // See https://github.com/informalsystems/hermes/pull/2950#issuecomment-1373733744

        PageRequest {
            limit: u32::MAX as u64,
            ..Default::default()
        }
    }
}

impl From<PageRequest> for IbcPageRequest {
    fn from(request: PageRequest) -> Self {
        IbcPageRequest {
            key: request.key,
            offset: request.offset,
            limit: request.limit,
            count_total: request.count_total,
            reverse: request.reverse,
        }
    }
}

pub fn query_latest_height(trpc: &mut HttpClient) -> Result<Height, Error> {
    let latest_block = trpc::block::latest_block(trpc)?;

    let latest_height = Height::new(
        ChainId::chain_version(latest_block.header.chain_id.as_str()),
        u64::from(latest_block.header.height),
    )
    .map_err(|e| Error::block_height("query_latest_height new height error".to_string(), e))?;

    Ok(latest_height)
}

pub fn query_trusted_height(
    dst_grpc: &mut IbcClientQueryClient<Channel>,
    client_id: ClientId,
    client_state: &ClientState,
    target_height: Height,
) -> Result<Height, Error> {
    let client_state_latest_height = client_state.latest_height;

    if client_state_latest_height < target_height {
        return Ok(client_state_latest_height);
    } else {
        let client_state_heights =
            grpc::consensus::query_all_consensus_state_heights(dst_grpc, client_id)?;

        client_state_heights
            .into_iter()
            .find(|h| h < &target_height)
            .ok_or_else(|| {
                Error::query_trusted_height(
                    "There is no height lower than target_height".to_string(),
                )
            })
    }
}
