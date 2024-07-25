use std::{borrow::BorrowMut, collections::HashMap, path::Display, sync::Arc};

use tokio::sync::RwLock;
use types::{
    ibc_core::ics02_client::height::Height,
    ibc_events::{IbcEvent, IbcEventWithHeight},
};

use crate::channel_pool::channel_key_by_packet;

type EventType = u16;
type ChannelKey = String;
pub type EventClassType = (EventType, Height, ChannelKey);

pub const SEND_PACKET_EVENT: u16 = 1;
pub const WRITE_ACK_EVENT: u16 = 2;

pub type CTXGroup = Vec<IbcEventWithHeight>;

#[derive(Debug, Clone)]
pub struct EventPool {
    ibc_events: Vec<IbcEventWithHeight>,
    ibc_events_class: HashMap<EventClassType, Vec<IbcEventWithHeight>>,
    ctx_pending_groups: HashMap<EventClassType, Vec<CTXGroup>>,
    next_heights: HashMap<(EventType, ChannelKey), Height>,
    pub group_size: u64,
}

impl EventPool {
    pub fn new() -> Self {
        EventPool {
            ibc_events: Vec::new(),
            ibc_events_class: HashMap::new(),
            ctx_pending_groups: HashMap::new(),
            next_heights: HashMap::new(),
            group_size: 100,
        }
    }

    pub fn get_ibc_events_class(&self) -> HashMap<EventClassType, Vec<IbcEventWithHeight>> {
        self.ibc_events_class.clone()
    }

    pub fn get_ibc_events_class_mut(&mut self) -> &mut HashMap<EventClassType, Vec<IbcEventWithHeight>> {
        self.ibc_events_class.borrow_mut()
    }

    pub fn clear_ibc_events_class(&mut self) {
        self.ibc_events_class.clear();
    }

    pub fn update_ctx_pending_groups(
        &mut self,
        event_class_type: &EventClassType,
        mut groups: Vec<CTXGroup>,
    ) {
        if let Some(gs) = self.ctx_pending_groups.get_mut(event_class_type) {
            gs.append(&mut groups);
        } else {
            self.ctx_pending_groups
                .insert(event_class_type.clone(), groups);

            let k = (event_class_type.0, event_class_type.2.clone());
            let event_height = event_class_type.1;
            if let Some(h) = self.next_heights.get(&k) {
                if event_height < *h {
                    self.next_heights.insert(k, event_height);
                }
            } else {
                self.next_heights.insert(k, event_height);
            }
        }

        println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
        // println!("pending_groups:{:?}", self.ctx_pending_groups.clone());
        println!("next_heights:{:?}", self.next_heights.clone());
    }

    pub fn get_ctx_pending_groups_mut(&mut self) -> &mut HashMap<EventClassType, Vec<CTXGroup>> {
        &mut self.ctx_pending_groups
    }

    pub fn push_events(&mut self, mut ibc_events: Vec<IbcEventWithHeight>) {
        self.ibc_events.append(ibc_events.as_mut());
    }

    pub fn push_events_class(&mut self, ibc_events: Vec<IbcEventWithHeight>) {
        ibc_events.into_iter().for_each(|event| {
            let event_height = event.height.clone();

            let (event_type, events_option, channel_key) = match event.clone().event {
                IbcEvent::SendPacket(evt) => {
                    let channel_key_result = channel_key_by_packet(&evt.packet);
                    match channel_key_result {
                        Ok(k) => (
                            SEND_PACKET_EVENT,
                            self.ibc_events_class.get_mut(&(
                                SEND_PACKET_EVENT,
                                event_height,
                                k.clone(),
                            )),
                            k,
                        ),
                        Err(e) => {
                            eprintln!("{}", e);
                            (0, None, "".to_string())
                        }
                    }
                }
                IbcEvent::WriteAcknowledgement(evt) => {
                    let channel_key_result = channel_key_by_packet(&evt.packet);
                    match channel_key_result {
                        Ok(k) => (
                            WRITE_ACK_EVENT,
                            self.ibc_events_class.get_mut(&(
                                SEND_PACKET_EVENT,
                                event_height,
                                k.clone(),
                            )),
                            k,
                        ),
                        Err(e) => {
                            eprintln!("{}", e);
                            (0, None, "".to_string())
                        }
                    }
                }
                _ => (0, None, "".to_string()),
            };

            if let Some(events) = events_option {
                events.append(vec![event.clone()].as_mut());
            } else {
                self.ibc_events_class.insert(
                    (event_type, event_height, channel_key.clone()),
                    vec![event.clone()],
                );
            }

            // if let Some(h) = self.next_heights.get(&(event_type, channel_key.clone())) {
            //     if event_height < *h {
            //         self.next_heights
            //             .insert((event_type, channel_key), event_height);
            //     }
            // } else {
            //     self.next_heights
            //         .insert((event_type, channel_key.clone()), event_height);
            // }

            // println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
            // println!("ibc_events_class:{:?}", self.ibc_events_class.clone());
            // println!("next_heights:{:?}", self.next_heights.clone());
        });
    }

    pub fn clear_pool(&mut self) -> Vec<IbcEventWithHeight> {
        let ibc_events = self.ibc_events.clone();
        self.ibc_events.clear();

        ibc_events
    }

    pub fn read_latest_event(&mut self) -> Option<IbcEventWithHeight> {
        let event = self.ibc_events.last().cloned();
        self.ibc_events.pop();

        event
    }

    pub fn read_next_events(
        &mut self,
        event_type: EventType,
        num: usize,
        channel_key: String,
    ) -> Vec<IbcEventWithHeight> {
        let mut next_events = vec![];
        if let Some(h) = self.next_heights.get(&(event_type, channel_key.clone())) {
            println!("next_height_get=============");
            if let Some(events) =
                self.ibc_events_class
                    .get_mut(&(event_type, *h, channel_key.clone()))
            {
                println!("LEN:{:?}", events.len());
                if events.len() < num {
                    next_events = events.drain(..).collect();
                    println!("next_height_update=============");
                    self.next_heights
                        .insert((event_type, channel_key.clone()), *h + 1);
                } else {
                    next_events = events.drain(..num).collect();
                }
            } else {
                // println!("NOTNOT:{:?}", *h);
                self.next_heights
                    .insert((event_type, channel_key.clone()), *h + 1);
            }
        }

        next_events
    }

    pub fn next_pending_group(&mut self, event_type: EventType, channel_key: String) -> CTXGroup {
        let mut next_group = vec![];
        if let Some(h) = self.next_heights.get(&(event_type, channel_key.clone())) {
            println!("next_height_get=============");
            if let Some(groups) =
                self.ctx_pending_groups
                    .get_mut(&(event_type, *h, channel_key.clone()))
            {
                println!("LEN:{:?}", groups.len());
                let groups_num = groups.len();
                if 0 < groups_num && groups_num <= 1 {
                    next_group = groups.first().unwrap().clone();
                    self.ctx_pending_groups.remove(&(event_type, *h, channel_key.clone()));
                    self.next_heights
                        .insert((event_type, channel_key.clone()), *h + 1);
                } else if groups_num > 1 {
                    next_group = groups.first().unwrap().clone();
                    groups.remove(0);
                }
            } else {
                // println!("NOTNOT:{:?}", *h);
                self.next_heights
                    .insert((event_type, channel_key.clone()), *h + 1);
            }
        }

        next_group
    }
}

#[cfg(test)]
pub mod event_pool_tests {
    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    pub fn event_drain_works() {
        init();

        let mut test_vec = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let drain_vec = test_vec.drain(..5).collect::<Vec<i32>>();

        println!("{:?}", drain_vec);
        println!("{:?}", test_vec)
    }
}
