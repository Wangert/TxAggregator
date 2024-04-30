use ibc_proto::cosmos::auth::v1beta1::{
    query_client::QueryClient as AuthQueryClient, BaseAccount, EthAccount, QueryAccountRequest,
    QueryAccountsRequest,
};
use log::info;
use prost::Message;
use tonic::transport::Channel;

use crate::error::Error;
use tracing::info as tracing_info;

pub async fn query_detail_account(
    grpc_client: &mut AuthQueryClient<Channel>,
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
    grpc_client: &mut AuthQueryClient<Channel>,
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

#[cfg(test)]
pub mod query_grpc_account_tests {
    use crate::chain::CosmosChain;

    #[test]
    pub fn query_all_acount_works() {
        let file_path =
            "C:/Users/admin/Documents/GitHub/TxAggregator/cosmos_chain/src/config/chain_b_config.toml";
        let mut cosmos_chain = CosmosChain::new(file_path);
        let rt = tokio::runtime::Runtime::new().unwrap();

        // let rt = cosmos_chain.rt.clone();
        let account = rt
            .block_on(
                cosmos_chain.query_detail_account_by_address(
                    "cosmos1gn6f8n4wlnn4dcwq0c9pzy8sewu9jj6tzfs58c",
                ),
            )
            .expect("query account error!");

        println!("Account: {:#?}", account);
    }
}
