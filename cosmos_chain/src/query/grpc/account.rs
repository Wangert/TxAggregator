use http::Uri;
use ibc_proto::cosmos::auth::v1beta1::{
    query_client::QueryClient, BaseAccount, EthAccount, QueryAccountRequest, QueryAccountsRequest,
};
use log::info;
use prost::Message;
use tonic::{codegen::ok, transport::Channel};

use crate::{config::default::max_grpc_decoding_size, error::Error};
use tracing::info as tracing_info;

pub async fn query_detail_account(
    grpc_client: &mut QueryClient<Channel>,
    account_address: &str,
) -> Result<BaseAccount, Error> {
    let request = tonic::Request::new(QueryAccountRequest {
        address: account_address.to_string(),
    });

    // let request_all = tonic::Request::new(QueryAccountsRequest {
    //     pagination: None,
    // });

    // let response_all = client.accounts(request_all).await;
    // println!("{:?}", response_all);

    let reponse = grpc_client.account(request).await;

    info!("{:?}", reponse);

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

#[tracing::instrument]
pub async fn query_all_account(
    grpc_client: &mut QueryClient<Channel>,
) -> Result<Vec<BaseAccount>, Error> {
    tracing_info!("query all account access");
    let request_all = tonic::Request::new(QueryAccountsRequest { pagination: None });
    let reponse = grpc_client.accounts(request_all).await;

    //info!("{:?}", reponse);

    let accounts_resp = reponse
        .map_err(|e| Error::grpc_status(e, "query_accounts".to_owned()))?
        .into_inner()
        .accounts;

    let mut base_accounts: Vec<BaseAccount> = vec![];

    for account in accounts_resp {
        if account.type_url == "/cosmos.auth.v1beta1.BaseAccount" {
            let ba = BaseAccount::decode(account.value.as_slice())
                .map_err(|e| Error::protobuf_decode("BaseAccount".to_string(), e))?;
            base_accounts.push(ba);
        } else if account.type_url.ends_with(".EthAccount") {
            let ba = EthAccount::decode(account.value.as_slice())
                .map_err(|e| Error::protobuf_decode("EthAccount".to_string(), e))?
                .base_account
                .ok_or_else(Error::empty_base_account)?;
            base_accounts.push(ba);
        }
    }

    if base_accounts.len() > 0 {
        return Ok(base_accounts);
    }

    Err(Error::no_accounts())
}
