use std::{thread, time::Duration};

use ibc_proto::{
    cosmos::{auth::v1beta1::query_client::QueryClient, tx::v1beta1::Fee},
    google::protobuf::Any,
};
use tendermint_rpc::{
    endpoint::broadcast::tx_async::Response as TxAsyncResponse,
    endpoint::broadcast::tx_sync::Response as TxSyncResponse, Client, HttpClient,
};
use tonic::transport::Channel;
use types::ibc_events::{ibc_event_try_from_abci_event, IbcEvent};
use utils::encode::protobuf;

use crate::{
    account::Secp256k1Account,
    config::CosmosChainConfig,
    error::Error,
    query::{grpc::account::query_detail_account, trpc::tx::tx},
};

use super::{
    create::create_and_sign_tx,
    types::{Memo, TxStatus, TxSyncResult},
};

const WAIT_BACKOFF: Duration = Duration::from_millis(300);

pub async fn send_tx_with_fee(
    trpc_client: &HttpClient,
    grpc_query_client: &mut QueryClient<Channel>,
    chain_config: &CosmosChainConfig,
    account_info: &Secp256k1Account,
    tx_memo: &Memo,
    messages: &[Any],
    fee: Fee,
) -> Result<TxSyncResponse, Error> {
    let account_detail =
        query_detail_account(grpc_query_client, account_info.address().as_str()).await?;

    let (_, tx_raw) = create_and_sign_tx(
        chain_config,
        account_info,
        &account_detail,
        tx_memo,
        messages,
        Some(fee),
    )?;
    let tx_bytes = protobuf::encode_to_bytes(&tx_raw).map_err(|e| Error::tx_protobuf_encode(e))?;

    broadcast_tx_sync(trpc_client, tx_bytes).await
}

pub async fn broadcast_tx_sync(
    trpc_client: &HttpClient,
    tx_bytes: Vec<u8>,
) -> Result<TxSyncResponse, Error> {
    let response = trpc_client
        .broadcast_tx_sync(tx_bytes)
        .await
        .map_err(|e| Error::trpc("broadcast tx sync".to_string(), e))?;
    Ok(response)
}

pub async fn broadcast_tx_async(
    trpc_client: &HttpClient,
    tx_bytes: Vec<u8>,
) -> Result<TxAsyncResponse, Error> {
    let response = trpc_client
        .broadcast_tx_async(tx_bytes)
        .await
        .map_err(|e| Error::trpc("broadcast tx async".to_string(), e))?;
    Ok(response)
}

pub fn wait_for_tx_block_commit(
    trpc_client: &mut HttpClient,
    tx_sync_response: &TxSyncResponse,
) -> Result<TxSyncResult, Error> {
    if tx_sync_response.code.is_err() {
        Err(Error::tx_commit("tx sync response code is err".to_string()))
    } else {
        let rt = tokio::runtime::Runtime::new().unwrap();
        loop {
            let tx_response_result = rt.block_on(tx(trpc_client, tx_sync_response.hash, false));

            // println!("[wait_for_tx_block_commit]: tx_response_result=={:?}", tx_response_result);

            if tx_response_result.is_ok() {
                let tx_response = tx_response_result.unwrap();
                let mut events: Vec<IbcEvent> = vec![];

                if tx_response.tx_result.code.is_err() {
                    events = vec![
                        IbcEvent::CosmosChainError(format!(
                            "deliver_tx for {} reports error: code={:?}, log={:?}",
                            tx_response.hash, tx_response.tx_result.code, tx_response.tx_result.log
                        ));
                        1
                    ];
                } else {
                    events = tx_response
                        .tx_result
                        .events
                        .iter()
                        .flat_map(|event| {
                            ibc_event_try_from_abci_event(event).map_err(|e| {
                                Error::ibc_event("ibc_event_try_from_abci_event".to_string(), e)
                            })
                        })
                        .collect::<Vec<_>>();
                }

                return Ok(TxSyncResult {
                    response: tx_sync_response.clone(),
                    events,
                    status: TxStatus::ReceivedResponse,
                });
            }

            thread::sleep(WAIT_BACKOFF);
        }
    }
}

#[cfg(test)]
pub mod tx_send_tests {
    use std::time::Duration;

    use crate::{
        account::Secp256k1Account,
        chain::CosmosChain,
        client::{build_create_client_request, CreateClientOptions},
        query::{
            grpc::connect::{grpc_auth_client, grpc_staking_client, grpc_tx_service_client},
            trpc::connect::tendermint_rpc_client,
        },
        tx::{
            estimate::estimate_tx,
            send::{send_tx_with_fee, wait_for_tx_block_commit},
            types::Memo,
        },
    };
    use ibc_proto::{
        google::protobuf::Any, ibc::core::client::v1::MsgCreateClient as IbcMsgCreateClient,
    };
    use utils::encode::protobuf;

    #[test]
    pub fn send_tx_with_fee_wokrs() {
        let rt = tokio::runtime::Runtime::new().expect("runtime create error");

        let file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
        let cosmos_chain = CosmosChain::new(file_path);

        let account = Secp256k1Account::new(
            &cosmos_chain.config.chain_a_key_path,
            &cosmos_chain.config.hd_path,
        )
        .expect("account error!");

        let mut trpc_client = tendermint_rpc_client(&cosmos_chain.config.tendermint_rpc_addr);
        let mut grpc_staking_client =
            rt.block_on(grpc_staking_client(&cosmos_chain.config.grpc_addr));
        // let mut trpc_client = cosmos_chain.tendermint_rpc_client().unwrap();
        // let mut grpc_staking_client = cosmos_chain.grpc_staking_client().unwrap();

        let create_client_options = CreateClientOptions {
            max_clock_drift: Some(Duration::from_secs(cosmos_chain.config.max_block_time)),
            trusting_period: Some(Duration::from_secs(
                cosmos_chain.config.trusting_period * 86400,
            )),
            trust_level: None,
        };

        let src_chain_config = cosmos_chain.config.clone();
        let dst_chain_config = cosmos_chain.config.clone();

        println!("access build create client request");
        let msg_create_client = build_create_client_request(
            &mut trpc_client,
            &mut grpc_staking_client,
            &create_client_options,
            &src_chain_config,
            &dst_chain_config,
        )
        .expect("msg_create_client error!");

        let ibc_msg_create_client = IbcMsgCreateClient::from(msg_create_client);
        let protobuf_value =
            protobuf::encode_to_bytes(&ibc_msg_create_client).expect("protobuf encode error!");
        let msg = Any {
            type_url: "/ibc.core.client.v1.MsgCreateClient".to_string(),
            value: protobuf_value,
        };

        let mut grpc_tx_service_client =
            rt.block_on(grpc_tx_service_client(&cosmos_chain.config.grpc_addr));
        let mut grpc_auth_client = rt.block_on(grpc_auth_client(&cosmos_chain.config.grpc_addr));
        let tx_memo = Memo::default();
        let messages = vec![msg];

        println!("execute estimate_tx!!!");
        let fee = rt.block_on(estimate_tx(
            &src_chain_config,
            &mut grpc_auth_client,
            &mut grpc_tx_service_client,
            &account,
            &tx_memo,
            &messages,
        ));

        let fee = match fee {
            Ok(fee) => fee,
            Err(e) => panic!("panic {}", e),
        };

        println!("execute tx!!!");
        let tx_sync_resp = rt.block_on(send_tx_with_fee(
            &trpc_client,
            &mut grpc_auth_client,
            &src_chain_config,
            &account,
            &tx_memo,
            &messages,
            fee,
        ));

        let r = match tx_sync_resp {
            Ok(r) => r,
            Err(e) => panic!("panic {}", e),
        };
        println!("Tx_Sync_Response: {:?}", r);

        let tx_sync_result = wait_for_tx_block_commit(&mut trpc_client, &r)
            .expect("wait for tx block commit error!");

        println!("tx_sync_result: {:?}", tx_sync_result);
    }
}
