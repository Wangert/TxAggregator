use std::cmp::min;

use http::Uri;
use ibc_proto::{
    cosmos::{
        auth::v1beta1::query_client::QueryClient,
        base::v1beta1::Coin,
        tx::v1beta1::{service_client::ServiceClient, Fee, SimulateRequest, SimulateResponse, Tx},
    },
    google::protobuf::Any,
};
use log::{error, info};
use tonic::transport::Channel;
use utils::{
    encode::protobuf,
    operation::{mul_ceil, mul_floor},
};

use crate::{
    account::Secp256k1Account,
    config::CosmosChainConfig,
    error::Error,
    query::grpc::account::query_detail_account,
    tx::types::{GasConfig, GasPrice},
};

use super::{create::create_and_sign_tx, types::Memo};

pub async fn estimate_tx(
    chain_config: &CosmosChainConfig,
    grpc_query_client: &mut QueryClient<Channel>,
    grpc_service_client: &mut ServiceClient<Channel>,
    account_info: &Secp256k1Account,
    tx_memo: &Memo,
    messages: &[Any],
) -> Result<Fee, Error> {
    let account_detail =
        query_detail_account(grpc_query_client, account_info.address().as_str()).await?;
    let (tx, _) = create_and_sign_tx(
        chain_config,
        account_info,
        &account_detail,
        tx_memo,
        messages,
        None,
    )?;
    estimate_tx_fee(chain_config, grpc_service_client, tx).await
}

pub async fn estimate_tx_fee(
    chain_config: &CosmosChainConfig,
    grpc_service_client: &mut ServiceClient<Channel>,
    tx: Tx,
) -> Result<Fee, Error> {
    let gas_info_result = simulate_tx(grpc_service_client, tx)
        .await
        .map(|sr| sr.gas_info);
    let gas = match gas_info_result {
        Ok(Some(gas_info)) => {
            info!(
                "tx simulation successful, gas amount: {:?}",
                gas_info.gas_used
            );

            gas_info.gas_used
        }
        Ok(None) => {
            error!("tx simulation successful but no gas amount used was returned.");
            return Err(Error::simulate_tx_gas());
        }
        Err(e) => {
            error!("failed to simulate tx.");
            return Err(e);
        }
    };

    let gas_config = GasConfig::from(chain_config);

    let fee = gas_to_fee(&gas_config, gas);

    Ok(fee)
}

pub async fn estimate_tx_fee_with_grpc_address(
    chain_config: &CosmosChainConfig,
    grpc_address: &Uri,
    tx: Tx,
) -> Result<Fee, Error> {
    let gas_info_result = simulate_tx_with_grpc_address(grpc_address, tx)
        .await
        .map(|sr| sr.gas_info);
    let gas = match gas_info_result {
        Ok(Some(gas_info)) => {
            info!(
                "tx simulation successful, gas amount: {:?}",
                gas_info.gas_used
            );

            gas_info.gas_used
        }
        Ok(None) => {
            error!("tx simulation successful but no gas amount used was returned.");
            return Err(Error::simulate_tx_gas());
        }
        Err(e) => {
            error!("failed to simulate tx.");
            return Err(e);
        }
    };

    let gas_config = GasConfig::from(chain_config);

    let fee = gas_to_fee(&gas_config, gas);

    Ok(fee)
}

pub fn gas_to_fee(gas_config: &GasConfig, gas_amount: u64) -> Fee {
    let adjusted_gas_limit =
        adjust_estimated_gas(gas_config.gas_multiplier, gas_amount, gas_config.max_gas);

    // The fee in coins based on gas amount
    let fee_amount = mul_ceil(adjusted_gas_limit, gas_config.gas_price.price);

    let coin_fee = Coin {
        denom: gas_config.gas_price.denom.to_string(),
        amount: fee_amount.to_string(),
    };

    Fee {
        amount: vec![coin_fee],
        gas_limit: adjusted_gas_limit,
        payer: "".to_string(),
        granter: gas_config.fee_granter.clone(),
    }
}

/// Adjusts the fee based on the configured `gas_multiplier` to prevent out of gas errors.
/// The actual gas cost, when a transaction is executed, may be slightly higher than the
/// one returned by the simulation.
fn adjust_estimated_gas(gas_multiplier: f64, gas_amount: u64, max_gas: u64) -> u64 {
    // No need to compute anything if the gas amount is zero
    if gas_amount == 0 {
        return 0;
    };

    // If the multiplier is 1, no need to perform the multiplication
    if gas_multiplier == 1.0 {
        return min(gas_amount, max_gas);
    }

    // Multiply the gas estimate by the gas_multiplier option
    let (_sign, digits) = mul_floor(gas_amount, gas_multiplier).to_u64_digits();

    let gas = match digits.as_slice() {
        // If there are no digits it means that the resulting amount is zero.
        [] => 0,

        // If there is a single "digit", it means that the result fits in a u64, so we can use that.
        [gas] => *gas,

        // Otherwise, the multiplication overflow and we use u64::MAX instead.
        _ => u64::MAX,
    };

    // Bound the gas estimate by the max_gas option
    min(gas, max_gas)
}

pub fn calculate_fee(adjusted_gas_amount: u64, gas_price: &GasPrice) -> Coin {
    let fee_amount = mul_ceil(adjusted_gas_amount, gas_price.price);

    Coin {
        denom: gas_price.denom.to_string(),
        amount: fee_amount.to_string(),
    }
}

// The transaction is simulated by the given grpc address
pub async fn simulate_tx_with_grpc_address(
    grpc_address: &Uri,
    tx: Tx,
) -> Result<SimulateResponse, Error> {
    let tx_bytes = protobuf::encode_to_bytes(&tx).map_err(|e| Error::tx_protobuf_encode(e))?;

    let sim_request = SimulateRequest {
        tx_bytes,
        ..Default::default()
    };

    let mut grpc_service_client = ServiceClient::connect(grpc_address.clone())
        .await
        .map_err(Error::grpc_transport)?;

    let request = tonic::Request::new(sim_request);
    let response = grpc_service_client
        .simulate(request)
        .await
        .map_err(|e| Error::grpc_status(e, "simulate_tx".to_owned()))?
        .into_inner();

    Ok(response)
}

// The transaction is simulated by the given service client
pub async fn simulate_tx(
    grpc_service_client: &mut ServiceClient<Channel>,
    tx: Tx,
) -> Result<SimulateResponse, Error> {
    let tx_bytes = protobuf::encode_to_bytes(&tx).map_err(|e| Error::tx_protobuf_encode(e))?;

    let sim_request = SimulateRequest {
        tx_bytes,
        ..Default::default()
    };

    let request = tonic::Request::new(sim_request);
    let response = grpc_service_client
        .simulate(request)
        .await
        .map_err(|e| Error::grpc_status(e, "simulate_tx".to_owned()))?
        .into_inner();

    Ok(response)
}

#[cfg(test)]
pub mod estimate_tests {
    use std::time::Duration;

    use ibc_proto::google::protobuf::Any;
    use ibc_proto::ibc::core::client::v1::MsgCreateClient as IbcMsgCreateClient;

    use utils::encode::protobuf;

    use crate::{
        account::Secp256k1Account,
        chain::CosmosChain,
        client::{build_create_client_request, CreateClientOptions},
        query::{
            grpc::connect::{grpc_auth_client, grpc_staking_client, grpc_tx_service_client},
            trpc::connect::tendermint_rpc_client,
        },
        tx::types::Memo,
    };

    use super::estimate_tx;

    #[test]
    pub fn estimate_tx_works() {
        let rt = tokio::runtime::Runtime::new().expect("runtime create error");

        let file_path =
            "/Users/joten/rust_projects/TxAggregator/cosmos_chain/src/config/chain_config.toml";
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

        match fee {
            Ok(fee) => println!("Fee: {:?}", fee),
            Err(e) => panic!("panic {}", e),
        }
    }
}
