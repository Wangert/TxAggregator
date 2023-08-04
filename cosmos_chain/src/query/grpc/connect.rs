use http::Uri;
use ibc_proto::cosmos::{
    auth::v1beta1::{
        query_client::QueryClient as AuthQueryClient, BaseAccount, EthAccount, QueryAccountRequest,
    },
    staking::v1beta1::{query_client::QueryClient as StakingQueryClient, Params as StakingParams}, tx::v1beta1::service_client::ServiceClient,
};
use log::{info, trace};
use tonic::transport::Channel;

use crate::config::default::max_grpc_decoding_size;

pub async fn grpc_auth_client(grpc_addr: &str) -> AuthQueryClient<Channel> {
    trace!("grpc auth client connect");

    let grpc_addr = grpc_addr.parse::<Uri>().expect("grpc address parse error!");
    let auth_client = match AuthQueryClient::connect(grpc_addr.clone()).await {
        Ok(client) => client,
        Err(e) => panic!("grpc auth client connect error: {:?}", e),
    };

    info!("grpc auth client connect success");

    auth_client.max_decoding_message_size(max_grpc_decoding_size().get_bytes() as usize)
}

pub async fn grpc_staking_client(grpc_addr: &str) -> StakingQueryClient<Channel> {
    trace!("grpc staking client connect");

    let grpc_addr = grpc_addr.parse::<Uri>().expect("grpc address parse error!");
    let staking_client = match StakingQueryClient::connect(grpc_addr.clone()).await {
        Ok(client) => client,
        Err(e) => panic!("grpc staking client connect error: {:?}", e),
    };

    info!("grpc staking client connect success");

    staking_client.max_decoding_message_size(max_grpc_decoding_size().get_bytes() as usize)
}

pub async fn grpc_tx_service_client(grpc_addr: &str) -> ServiceClient<Channel> {
    trace!("grpc tx service client connect");

    let grpc_addr = grpc_addr.parse::<Uri>().expect("grpc address parse error!");
    let tx_service_client = match ServiceClient::connect(grpc_addr.clone()).await {
        Ok(client) => client,
        Err(e) => panic!("grpc tx service client connect error: {:?}", e),
    };

    info!("grpc tx service client connect success");

    tx_service_client.max_decoding_message_size(max_grpc_decoding_size().get_bytes() as usize)
}
