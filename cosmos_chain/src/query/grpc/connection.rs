use ibc_proto::{
    ibc::core::connection::v1::{
        query_client::QueryClient as ConnectionQueryClient, QueryConnectionRequest,
    },
    Protobuf,
};
use tendermint_rpc::HttpClient;
use tonic::{metadata::AsciiMetadataValue, transport::Channel, IntoRequest};
use types::ibc_core::{
    ics03_connection::connection::ConnectionEnd, ics23_commitment::merkle_tree::MerkleProof, ics24_host::{
        identifier::ConnectionId,
        path::{ConnectionsPath, IBC_QUERY_PATH},
    }
};

use crate::{common::QueryHeight, error::Error, query::trpc};

pub async fn query_connection(
    grpc_client: &mut ConnectionQueryClient<Channel>,
    trpc_client: &mut HttpClient,
    connection_id: &ConnectionId,
    height_query: QueryHeight,
    prove: bool,
) -> Result<(ConnectionEnd, Option<MerkleProof>), Error> {
    if prove {
        let abci_query = trpc::abci::abci_query(
            trpc_client,
            IBC_QUERY_PATH.to_string(),
            ConnectionsPath(connection_id.clone()).to_string(),
            height_query.into(),
            prove,
        )
        .await?;
        let connection_end = ConnectionEnd::decode_vec(&abci_query.value)
            .map_err(|e| Error::tendermint_protobuf_decode("ConnectionEnd".to_string(), e))?;

        Ok((
            connection_end,
            Some(
                abci_query
                    .merkle_proof
                    .ok_or_else(Error::empty_response_proof)?,
            ),
        ))
    } else {
        let mut request = QueryConnectionRequest {
            connection_id: connection_id.to_string(),
        }
        .into_request();

        let height_param = AsciiMetadataValue::try_from(height_query)?;
        request
            .metadata_mut()
            .insert("x-cosmos-block-height", height_param);

        let response = grpc_client.connection(request).await.map_err(|e| {
            if e.code() == tonic::Code::NotFound {
                Error::connection_not_found(connection_id.clone())
            } else {
                Error::grpc_status(e, "query_connection".to_owned())
            }
        })?;

        match response.into_inner().connection {
            Some(raw_connection) => {
                let connection_end =
                    ConnectionEnd::try_from(raw_connection).map_err(Error::connection_error)?;
                Ok((connection_end, None))
            }
            None => Err(Error::connection_not_found(connection_id.clone())),
        }
    }
}
