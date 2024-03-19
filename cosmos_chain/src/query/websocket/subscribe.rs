use std::{str::FromStr, sync::Arc};

use tendermint_rpc::{query::EventType, WebSocketClient, SubscriptionClient, Subscription, WebSocketClientUrl};
use tokio::{task::JoinHandle, runtime::Runtime};
use futures::StreamExt;

use crate::error::{Error, self};

pub struct EventSubscriptions {
    pub client: WebSocketClient,
    pub subs: Vec<Subscription>,
    pub driver_handle: JoinHandle<()>,
    pub rt: Arc<Runtime>
}

impl EventSubscriptions {
    pub fn new(rt: Arc<Runtime>) -> EventSubscriptions {    
        // let url =  WebSocketClientUrl::from_str("ws://127.0.0.1:26657/websocket").unwrap();
        // let (client, driver) = rt.block_on(WebSocketClient::builder(url).build()).expect("build error!");
        let (client, driver) = rt.block_on(WebSocketClient::new("ws://127.0.0.1:26657/websocket")).expect("build error!");
        let driver_handle = rt.spawn(async move { driver.run().await.unwrap() });
        
        let mut subs = vec![];
        
        
        EventSubscriptions { client, subs, driver_handle, rt }   
    }

    pub async fn receive_events(&mut self, event: EventType) {
        let sub_result = self.client.subscribe(event.into()).await;
        let mut sub: Subscription;
        println!("1111111");
        match sub_result {
            Ok(s) => sub = s,
            Err(e) => { println!("{}", e); panic!("error!!"); }
        }
        
        let mut ev_count = 100;
        println!("99999999999999999");
        while let Some(res) = sub.next().await {
            match res {
                Ok(event) => println!("Got event: {:?}", event),
                Err(e) => panic!("{}", e)
            }

            // let ev = res.unwrap();
            // println!("############################");
            // println!("Got event: {:?}", ev);

            ev_count -= 1;
            if ev_count < 0 {
                break;
            }
        }
    }

    // pub async fn stop(&mut self) {
    //     self.client.clone().close().unwrap();
    //     self.driver_handle.await.unwrap();
    // }


}


#[cfg(test)]
pub mod subscribe_tests {
    use std::{borrow::BorrowMut, sync::Arc};

    use futures::StreamExt;
    use tendermint_rpc::{query::EventType, SubscriptionClient};

    use crate::query::websocket::subscribe::EventSubscriptions;


    #[test]
    pub fn subscribe_newblock_event_works() {
        let rt = Arc::new(tokio::runtime::Runtime::new().expect("runtime create error"));

        println!("111111");
        let mut es = EventSubscriptions::new(rt);

        println!("88888888888888");
        
        let rrt = tokio::runtime::Runtime::new().expect("runtime create error");
        rrt.block_on(es.receive_events(EventType::NewBlock));
        es.client.close().unwrap();
        rrt.block_on(es.driver_handle).unwrap();
        
    }
}