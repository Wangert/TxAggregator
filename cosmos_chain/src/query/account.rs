use http::Uri;
use ibc_proto::cosmos::auth::v1beta1::{
    query_client::QueryClient, BaseAccount, EthAccount, QueryAccountRequest, QueryAccountsRequest,
};
use prost::Message;

use crate::{config::default::max_grpc_decoding_size, error::Error};

pub async fn query_detail_account(
    grpc_address: &Uri,
    account_address: &str,
) -> Result<BaseAccount, Error> {
    let mut client = QueryClient::connect(grpc_address.clone())
        .await
        .map_err(Error::grpc_transport)?;

    client = client.max_decoding_message_size(max_grpc_decoding_size().get_bytes() as usize);

    let request = tonic::Request::new(QueryAccountRequest {
        address: account_address.to_string(),
    });

    let request_all = tonic::Request::new(QueryAccountsRequest {
        pagination: None,
    });

    let response_all = client.accounts(request_all).await;
    println!("{:?}", response_all);

    let reponse = client.account(request).await;

    println!("{:?}", reponse);

    let account_resp = match reponse
        .map_err(|e| Error::grpc_status(e, "query_account".to_owned()))?
        .into_inner()
        .account
    {
        Some(account) => account,
        None => return Err(Error::empty_query_account(account_address.to_string())),
    };

    if account_resp.type_url == "/cosmos.auth.v1beta1.BaseAccount" {
        Ok(BaseAccount::decode(account_resp.value.as_slice())
            .map_err(|e| Error::protobuf_decode("BaseAccount".to_string(), e))?)
    } else if account_resp.type_url.ends_with(".EthAccount") {
        Ok(EthAccount::decode(account_resp.value.as_slice())
            .map_err(|e| Error::protobuf_decode("EthAccount".to_string(), e))?
            .base_account
            .ok_or_else(Error::empty_base_account)?)
    } else {
        Err(Error::unknown_account_type(account_resp.type_url))
    }
}
