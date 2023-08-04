use ibc_proto::cosmos::staking::v1beta1::{
    query_client::QueryClient as StakingQueryClient, Params,
    QueryParamsRequest as StakingQueryParamsRequest,
};
use tonic::transport::Channel;

use crate::error::Error;

pub fn query_staking_params(
    grpc_client: &mut StakingQueryClient<Channel>,
) -> Result<Params, Error> {
    let request = tonic::Request::new(StakingQueryParamsRequest {});

    let rt = tokio::runtime::Runtime::new().expect("runtime create error");
    let response = rt
        .block_on(grpc_client.params(request))
        .map_err(|e| Error::grpc_status(e, "query staking params".into()))?;

    let staking_params = response
        .into_inner()
        .params
        .ok_or_else(|| Error::cosmos_params("staking params empty".to_string()))?;

    Ok(staking_params)
}
