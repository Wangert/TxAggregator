use std::{collections::HashMap, sync::Arc};

use futures::{
    stream::{self, select_all},
    Stream, StreamExt, TryStreamExt,
};
use tendermint_rpc::{event::Event, query::Query, SubscriptionClient, WebSocketClient};
use tokio::{runtime::Runtime, sync::RwLock, task::JoinHandle};
use tracing::trace;
use types::{
    ibc_core::{ics02_client::height::Height, ics24_host::identifier::ChainId},
    ibc_events::IbcEventWithHeight,
};

use crate::{
    event_pool,
    query::websocket::{
        collect_event::{self, collect_events},
        error::WsError,
    },
};

use tendermint_rpc::error::Error as TendermintRpcError;

use super::event_source::all_event_sources;
use std::thread;

type SubscriptionResult = core::result::Result<Event, TendermintRpcError>;
type SubscriptionsStream = dyn Stream<Item = SubscriptionResult> + Send + Sync + Unpin;

pub struct EventPool {
    events_by_height: HashMap<Height, Vec<IbcEventWithHeight>>,
}
impl EventPool {
    // 在这里可以定义初始化事件池的方法，比如 new() 方法
    pub fn new() -> EventPool {
        EventPool {
            events_by_height: HashMap::new(),
        }
    }

    // 定义一个方法用于将事件放入事件池中
    pub fn add_event(&mut self, event: IbcEventWithHeight) {
        // 获取事件的高度
        let height = event.height;

        // 将事件放入对应高度的事件列表中
        self.events_by_height
            .entry(height)
            .or_insert(Vec::new())
            .push(event);
    }

    pub fn read_with_height(&self, height: Height) {
        if let Some(events) = self.events_by_height.get(&height) {
            println!("Events at height {:?}:", height);
            for event in events {
                println!("  {:?}", event);
            }
        } else {
            println!("No events found at height {:?}", height);
        }
    }
}

pub struct EventSubscriptions {
    pub client: WebSocketClient,
    pub subs: Box<SubscriptionsStream>,
    pub driver_handle: JoinHandle<()>,
    pub queries: Vec<Query>,
    pub rt: Arc<Runtime>,
    // pub event_pool:Arc<RwLock<EventPool>>,
}

impl EventSubscriptions {
    pub fn new(rt: Arc<Runtime>) -> EventSubscriptions {
        let (client, driver) = rt
            .block_on(WebSocketClient::new("ws://10.176.35.58:26659/websocket"))
            .expect("build error!");
        let driver_handle = rt.spawn(async move { driver.run().await.unwrap() });

        let subs = Box::new(futures::stream::empty());
        let queries = all_event_sources();

        // let event_pool = Arc::new(RwLock::new(EventPool::new()));
        EventSubscriptions {
            client,
            subs,
            driver_handle,
            queries,
            rt,
            // event_pool,
        }
    }

    pub fn init_subscriptions(&mut self) -> Result<(), WsError> {
        let mut subscriptions = vec![];

        for query in &self.queries {
            trace!("subscribing to query: {}", query);

            let subscription = self
                .rt
                .block_on(self.client.subscribe(query.clone()))
                .map_err(WsError::client_subscription_failed)?;
            println!("{:?}", subscription);
            subscriptions.push(subscription);
        }

        self.subs = Box::new(select_all(subscriptions));

        Ok(())
    }

    pub async fn listen_events(&mut self, event_pool: Arc<RwLock<EventPool>>) {
        let subs = core::mem::replace(&mut self.subs, Box::new(stream::empty()));

        let chain_id = ChainId::default();
        let mut events = subs
            .map_ok(move |rpc_event| {
                trace!(chain = %chain_id, "received an RPC event: {}", rpc_event.query);
                collect_events(&chain_id, rpc_event)
            })
            .map_err(WsError::canceled_or_generic)
            .try_flatten();
        // let event_pool = Arc::new(RwLock::new(EventPool::new()));
        let mut ev_count = 10;
        println!("99999999999999999");
        while let Some(res) = events.next().await {
            match res {
                Ok(event) => {
                    println!("Got event: {}", event);
                    // let mut event_pool = self.event_pool.write().await;
                    let _ = event_pool.write().await.add_event(event);
                    // event_pool_write.add_event(event)
                }
                Err(e) => panic!("{}", e),
            }

            ev_count -= 1;
            if ev_count < 0 {
                break;
            }
        }
    }

    // pub async fn read_event(&mut self){
    //     let height = Height::new(0, 11111).unwrap();
    //     let _ = self.event_pool.read().await.read_with_height(height);

    // }
}

#[cfg(test)]
pub mod subscribe_tests {
    use std::{sync::Arc, time::Duration};
    use std::thread;
    use tendermint_rpc::SubscriptionClient;
    use tokio::sync::RwLock;
    use tokio::{time,try_join};
    use types::ibc_core::ics02_client::height::Height;

    use crate::{
        event_pool,
        query::websocket::subscribe::{EventPool, EventSubscriptions},
    };

    #[test]
    pub fn subscribe_newblock_event_works() {
        let rt = Arc::new(tokio::runtime::Runtime::new().expect("runtime create error"));

        println!("111111");
        let mut es = EventSubscriptions::new(rt.clone());

        println!("88888888888888");
        let event_pool = Arc::new(RwLock::new(EventPool::new()));
        let event_pool_clone = event_pool.clone();

        _ = es.init_subscriptions();
        let write_thread = rt.spawn(async move {
            // rt.block_on(es.listen_events(event_pool));
            es.listen_events(event_pool_clone).await;
            es.client.close().unwrap();
            // rt.block_on(es.driver_handle).unwrap();
            es.driver_handle.await;
            // time::sleep(Duration::from_secs(2)).await;
            // es.client.clone().unwrap();
        });
        let read_thread = rt.spawn(async move {
            let height = Height::new(0, 139780).unwrap();
            // let result = rt.block_on(es.event_pool.read());
            loop {
                let _ = event_pool.read().await.read_with_height(height);
                time::sleep(Duration::from_secs(2)).await;
            }
            // time::sleep(Duration::from_secs(2)).await;
        });
        thread::sleep(Duration::from_secs(20));
        // rt_clone.time::sleep(Duration::from_secs(2));
        
    }
}
