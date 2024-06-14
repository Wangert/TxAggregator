use std::{collections::HashMap, path::Display, sync::Arc};

use tokio::sync::RwLock;
use types::{
    ibc_core::ics02_client::height::Height,
    ibc_events::{IbcEvent, IbcEventWithHeight},
};

use crate::channel_pool::channel_key_by_packet;

type EventType = u16;
type ChannelKey = String;
type EventClassType = (EventType, Height, ChannelKey);

pub const SEND_PACKET_EVENT: u16 = 1;
pub const WRITE_ACK_EVENT: u16 = 2;

#[derive(Debug, Clone)]
pub struct EventPool {
    ibc_events: Vec<IbcEventWithHeight>,
    ibc_events_class: HashMap<EventClassType, Vec<IbcEventWithHeight>>,
    next_heights: HashMap<(EventType, ChannelKey), Height>,
}

impl EventPool {
    pub fn new() -> Self {
        EventPool {
            ibc_events: Vec::new(),
            ibc_events_class: HashMap::new(),
            next_heights: HashMap::new(),
        }
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

            if let Some(h) = self.next_heights.get(&(event_type, channel_key.clone())) {
                if event_height < *h {
                    self.next_heights
                        .insert((event_type, channel_key), event_height);
                }
            }
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
            if let Some(events) = self
                .ibc_events_class
                .get_mut(&(event_type, *h, channel_key))
            {
                if events.len() < num {
                    next_events = events.drain(..).collect();
                } else {
                    next_events = events.drain(..num).collect();
                }
            }
        }

        next_events
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
