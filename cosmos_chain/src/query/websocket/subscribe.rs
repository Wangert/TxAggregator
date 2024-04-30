use std::{
    borrow::Borrow, clone, collections::HashMap, future::IntoFuture, sync::Arc, time::Duration,
};

use futures::{
    future::ok,
    stream::{self, select_all},
    Stream, StreamExt, TryStreamExt,
};
use tendermint_rpc::{event::Event, query::Query, SubscriptionClient, WebSocketClient};
use tokio::{runtime::Runtime, sync::RwLock, task::JoinHandle, time};
use tracing::trace;
use types::{
    ibc_core::{ics02_client::height::Height, ics24_host::identifier::ChainId},
    ibc_events::IbcEventWithHeight,
};

use crate::{
    event_pool::EventPool, query::websocket::{
        collect_event::{self, collect_events},
        error::WsError,
    }
};

use tendermint_rpc::error::Error as TendermintRpcError;

use super::event_source::all_event_sources;
use std::thread;

type SubscriptionResult = core::result::Result<Event, TendermintRpcError>;
type SubscriptionsStream = dyn Stream<Item = SubscriptionResult> + Send + Sync + Unpin;

// #[derive(Clone)]
// pub struct EventPool {
//     events_by_height: HashMap<Height, Vec<IbcEventWithHeight>>,
// }
// impl EventPool {
//     // 在这里可以定义初始化事件池的方法，比如 new() 方法
//     pub fn new() -> EventPool {
//         EventPool {
//             events_by_height: HashMap::new(),
//         }
//     }

//     // 定义一个方法用于将事件放入事件池中
//     pub fn add_event(&mut self, event: IbcEventWithHeight) {
//         // 获取事件的高度
//         let height = event.height;

//         // 将事件放入对应高度的事件列表中
//         self.events_by_height
//             .entry(height)
//             .or_insert(Vec::new())
//             .push(event);
//     }

//     pub fn read_with_height(&self, height: Height) {
//         let ep = Arc::new(self.clone());
//         let read_thread = tokio::spawn(async move {
//             if let Some(events) = ep.events_by_height.get(&height) {
//                 println!("Events at height {:?}:", height);
//                 for event in events {
//                     println!("  {:?}", event);
//                 }
//             } else {
//                 println!("No events found at height {:?}", height);
//             }
//         });
//     }
// }

pub struct EventSubscriptions {
    pub client: Option<WebSocketClient>,
    pub subs: Box<SubscriptionsStream>,
    pub driver_handle: JoinHandle<()>,
    pub queries: Vec<Query>,
}

impl EventSubscriptions {
    pub fn new() -> EventSubscriptions {
        let subs = Box::new(futures::stream::empty());
        let queries = all_event_sources();

        EventSubscriptions {
            client: None,
            subs,
            driver_handle: tokio::spawn(async {}),
            queries,
        }
    }

    pub fn client(&self) -> Option<WebSocketClient> {
        self.client.clone()
    }

    // pub async fn driver_handle_stop(&self) {
    //     self.driver_handle.await.unwrap();
    // }

    pub async fn init_subscriptions(&mut self) -> Result<(), WsError> {
        let (client, driver) = WebSocketClient::new("ws://127.0.0.1:26657/websocket")
            .await
            .expect("websocket new error!");

        let driver_handle = tokio::spawn(async move { driver.run().await.unwrap() });

        self.client = Some(client);
        self.driver_handle = driver_handle;

        let mut subscriptions = vec![];
        // println!("55555555");
        for query in &self.queries {
            trace!("subscribing to query: {}", query);

            let subscription = self
                .client
                .as_ref()
                .ok_or_else(WsError::client_is_not_exist)?
                .subscribe(query.clone())
                .await
                .map_err(WsError::client_subscription_failed)?;

            // println!("{:?}", subscription);
            subscriptions.push(subscription);
        }

        self.subs = Box::new(select_all(subscriptions));

        Ok(())
    }

    pub fn listen_events(&mut self, chain_id: ChainId, event_pool: Arc<RwLock<EventPool>>) {
        let subs = core::mem::replace(&mut self.subs, Box::new(stream::empty()));
        let driver_handle = core::mem::replace(&mut self.driver_handle, tokio::spawn(async {}));
        let client = self.client();

        tokio::spawn(async move {
            // let chain_id = ChainId::default();
            let mut events = subs
                .map_ok(move |rpc_event| {
                    trace!(chain = %chain_id, "received an RPC event: {}", rpc_event.query);
                    collect_events(&chain_id, rpc_event)
                })
                .map_err(WsError::canceled_or_generic)
                .try_flatten();
            let event_pool_clone = event_pool.clone();
            let mut ev_count = 10;
            // println!("99999999999999999");
            while let Some(res) = events.next().await {
                match res {
                    Ok(event) => {
                        println!("Got event: {:?}", event);
                        let _ = event_pool_clone.write().await.push_events(vec![event]);
                    }
                    Err(e) => panic!("{}", e),
                }
                ev_count -= 1;
                if ev_count < 0 {
                    break;
                }
            }

            if let Some(c) = client {
                _ = c.close();
            }

            driver_handle.await.unwrap();
        });
    }
}

#[cfg(test)]
pub mod subscribe_tests {

    use std::{sync::Arc, time::Duration};

    use tokio::sync::RwLock;

    use types::ibc_core::ics24_host::identifier::ChainId;

    use crate::{event_pool::EventPool, query::websocket::subscribe::EventSubscriptions};

    #[tokio::test]
    pub async fn subscribe_newblock_event_works() {
        // let rt = Arc::new(tokio::runtime::Runtime::new().expect("runtime create error"));

        println!("111111");
        let mut es = EventSubscriptions::new();

        println!("88888888888888");
        let event_pool = Arc::new(RwLock::new(EventPool::new()));
        let event_pool_clone = event_pool.clone();
        let chain_id = ChainId::default();

        es.init_subscriptions().await.unwrap();

        es.listen_events(chain_id, event_pool_clone);

        tokio::time::sleep(Duration::from_secs(50)).await;
    }
}
