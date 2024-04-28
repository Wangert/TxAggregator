use std::sync::Arc;

use futures::{
    stream::{self, select_all},
    Stream, StreamExt, TryStreamExt,
};
use tendermint_rpc::{event::Event, query::Query, SubscriptionClient, WebSocketClient};
use tokio::{runtime::Runtime, task::JoinHandle};
use tracing::trace;
use types::ibc_core::ics24_host::identifier::ChainId;

use crate::query::websocket::{
    collect_event::{self, collect_events},
    error::WsError,
};

use tendermint_rpc::error::Error as TendermintRpcError;

use super::event_source::all_event_sources;

type SubscriptionResult = core::result::Result<Event, TendermintRpcError>;
type SubscriptionsStream = dyn Stream<Item = SubscriptionResult> + Send + Sync + Unpin;

pub struct EventSubscriptions {
    pub client: WebSocketClient,
    pub subs: Box<SubscriptionsStream>,
    pub driver_handle: JoinHandle<()>,
    pub queries: Vec<Query>,
    // pub rt: Arc<Runtime>,
}

impl EventSubscriptions {
    pub fn new(rt: Arc<Runtime>) -> EventSubscriptions {
        // let rt = Runtime::new().unwrap();
        let (client, driver) = rt
            .block_on(WebSocketClient::new("ws://127.0.0.1:26657/websocket"))
            .expect("build error!");
        let driver_handle = rt.spawn(async move { driver.run().await.unwrap() });

        let subs = Box::new(futures::stream::empty());
        let queries = all_event_sources();

        EventSubscriptions {
            client,
            subs,
            driver_handle,
            queries,
            // rt,
        }
    }

    pub fn init_subscriptions(&mut self, rt: Arc<Runtime>) -> Result<(), WsError> {
        // let rt = Runtime::new().unwrap();
        let mut subscriptions = vec![];

        for query in &self.queries {
            trace!("subscribing to query: {}", query);

            let subscription = rt.block_on(self.client.subscribe(query.clone()))
                .map_err(WsError::client_subscription_failed)?;

            subscriptions.push(subscription);
        }

        self.subs = Box::new(select_all(subscriptions));

        Ok(())
    }

    pub async fn listen_events(&mut self) {
        let subs = core::mem::replace(&mut self.subs, Box::new(stream::empty()));

        let chain_id = ChainId::default();
        let mut events = subs
            .map_ok(move |rpc_event| {
                trace!(chain = %chain_id, "received an RPC event: {}", rpc_event.query);
                collect_events(&chain_id, rpc_event)
            })
            .map_err(WsError::canceled_or_generic)
            .try_flatten();

        let mut ev_count = 5;
        println!("99999999999999999");

        while let Some(res) = events.next().await {
            match res {
                Ok(event) => println!("Got event: {:?}", event),
                Err(e) => panic!("{}", e),
            }

            ev_count -= 1;
            if ev_count < 0 {
                break;
            }
        }
    }

}

#[cfg(test)]
pub mod subscribe_tests {
    use std::sync::Arc;

    use tendermint_rpc::SubscriptionClient;

    use crate::query::websocket::subscribe::EventSubscriptions;

    #[test]
    pub fn subscribe_newblock_event_works() {
        // console_error_panic_hook::set_once();
        let rt = Arc::new(tokio::runtime::Runtime::new().expect("runtime create error"));

        // let rt = tokio::runtime::Runtime::new().expect("runtime create error");

        println!("111111");
        let mut es = EventSubscriptions::new(rt.clone());

        println!("88888888888888");
    

        _ = es.init_subscriptions(rt.clone());

        let rt_2 = tokio::runtime::Runtime::new().expect("runtime create error");

        rt_2.block_on(es.listen_events());

        es.client.close().unwrap();
        rt_2.block_on(es.driver_handle).unwrap();
    }
}
