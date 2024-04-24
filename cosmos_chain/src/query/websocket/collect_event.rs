use std::collections::HashMap;

use futures::{stream::{self, StreamExt, TryStreamExt}, Stream};
use types::{ibc_core::{ics02_client::height::Height, ics24_host::identifier::ChainId}, ibc_events::{ibc_event_try_from_abci_event, IbcEvent, IbcEventWithHeight}};
use tendermint_rpc::event::{Event as TmRpcEvent, EventData as TmRpcEventData};
use types::ibc_core::ics02_client::events as ClientEvents;
use super::{error::WsError, event_source};
use types::ibc_core::ics03_connection::events as ConnectionEvents;
use types::ibc_core::ics04_channel::events as ChannelEvents;

/// Collect the IBC events from an RPC event
pub fn collect_events(
    chain_id: &ChainId,
    event: TmRpcEvent,
) -> impl Stream<Item = Result<IbcEventWithHeight, WsError>> {
    let events = extract_events(chain_id, event).unwrap_or_default();
    stream::iter(events).map(Ok)
}

fn extract_events(
    chain_id: &ChainId,
    result: TmRpcEvent,
) -> Result<Vec<IbcEventWithHeight>, String> {
    let mut events_with_height: Vec<IbcEventWithHeight> = vec![];
    let TmRpcEvent {
        data,
        events,
        query,
    } = result;
    let events = events.ok_or("missing events")?;

    match data {
        TmRpcEventData::NewBlock { block, .. } | TmRpcEventData::LegacyNewBlock { block, .. }
            if query == event_source::new_block().to_string() =>
        {
            let height = Height::new(
                ChainId::chain_version(chain_id.to_string().as_str()),
                u64::from(block.as_ref().ok_or("tx.height")?.header.height),
            )
            .map_err(|_| String::from("tx.height: invalid header height of 0"))?;

            events_with_height.push(IbcEventWithHeight::new(
                ClientEvents::NewBlock::new(height).into(),
                height,
            ));
            // events_with_height.append(&mut parse_block_events(height, &events));
        }
        TmRpcEventData::Tx { tx_result } => {
            let height = Height::new(
                ChainId::chain_version(chain_id.to_string().as_str()),
                tx_result.height as u64,
            )
            .map_err(|_| String::from("tx_result.height: invalid header height of 0"))?;

            for abci_event in &tx_result.result.events {
                if let Ok(ibc_event) = ibc_event_try_from_abci_event(abci_event) {
                    println!("EVENTEVENTVENTVENTVENTVENT");
                    println!("{:?}", query);
                    println!("{:?}", ibc_event);
                    if query == event_source::ibc_client().to_string()
                        && event_is_type_client(&ibc_event)
                    {
                        tracing::trace!("extracted ibc_client event {}", ibc_event);
                        events_with_height.push(IbcEventWithHeight::new(ibc_event, height));
                    } else if query == event_source::ibc_connection().to_string()
                        && event_is_type_connection(&ibc_event)
                    {
                        tracing::trace!("extracted ibc_connection event {}", ibc_event);
                        events_with_height.push(IbcEventWithHeight::new(ibc_event, height));
                    } else if query == event_source::ibc_channel().to_string()
                        && event_is_type_channel(&ibc_event)
                    {
                        let _span = tracing::trace_span!("ibc_channel event").entered();
                        tracing::trace!("extracted {}", ibc_event);
                        if matches!(ibc_event, IbcEvent::SendPacket(_)) {
                            // Should be the same as the hash of tx_result.tx?
                            if let Some(hash) =
                                events.get("tx.hash").and_then(|values| values.first())
                            {
                                tracing::trace!(event = "SendPacket", "tx hash: {}", hash);
                            }
                        }

                        events_with_height.push(IbcEventWithHeight::new(ibc_event, height));
                    }
                }
            }
        }
        _ => {}
    }

    Ok(events_with_height)
}


fn event_is_type_client(ev: &IbcEvent) -> bool {
    matches!(
        ev,
        IbcEvent::CreateClient(_)
            | IbcEvent::UpdateClient(_)
            | IbcEvent::UpgradeClient(_)
            // | IbcEvent::ClientMisbehaviour(_)
    )
}

fn event_is_type_connection(ev: &IbcEvent) -> bool {
    matches!(
        ev,
        IbcEvent::OpenInitConnection(_)
            | IbcEvent::OpenTryConnection(_)
            | IbcEvent::OpenAckConnection(_)
            | IbcEvent::OpenConfirmConnection(_)
    )
}

fn event_is_type_channel(ev: &IbcEvent) -> bool {
    matches!(
        ev,
        IbcEvent::OpenInitChannel(_)
            | IbcEvent::OpenTryChannel(_)
            | IbcEvent::OpenAckChannel(_)
            | IbcEvent::OpenConfirmChannel(_)
            | IbcEvent::CloseInitChannel(_)
            | IbcEvent::CloseConfirmChannel(_)
            | IbcEvent::SendPacket(_)
            | IbcEvent::ReceivePacket(_)
            | IbcEvent::WriteAcknowledgement(_)
            | IbcEvent::AcknowledgePacket(_)
            | IbcEvent::TimeoutPacket(_)
            | IbcEvent::TimeoutOnClosePacket(_)
    )
}

// fn parse_block_events(
//     height: Height,
//     block_events: &HashMap<String, Vec<String>>,
// ) -> Vec<IbcEventWithHeight> {
//     #[inline]
//     fn extract_events<'a, T: TryFrom<RawObject<'a>>>(
//         height: Height,
//         block_events: &'a HashMap<String, Vec<String>>,
//         event_type: &str,
//         event_field: &str,
//     ) -> Vec<T> {
//         block_events
//             .get(&format!("{event_type}.{event_field}"))
//             .unwrap_or(&vec![])
//             .iter()
//             .enumerate()
//             .filter_map(|(i, _)| {
//                 let raw_obj = RawObject::new(height, event_type.to_owned(), i, block_events);
//                 T::try_from(raw_obj).ok()
//             })
//             .collect()
//     }

//     #[inline]
//     fn append_events<T: Into<IbcEvent>>(
//         events: &mut Vec<IbcEventWithHeight>,
//         chan_events: Vec<T>,
//         height: Height,
//     ) {
//         events.append(
//             &mut chan_events
//                 .into_iter()
//                 .map(|ev| IbcEventWithHeight::new(ev.into(), height))
//                 .collect(),
//         );
//     }

//     let mut events: Vec<IbcEventWithHeight> = vec![];
//     append_events::<ChannelEvents::OpenInit>(
//         &mut events,
//         extract_events(height, block_events, "channel_open_init", "channel_id"),
//         height,
//     );
//     append_events::<ChannelEvents::OpenTry>(
//         &mut events,
//         extract_events(height, block_events, "channel_open_try", "channel_id"),
//         height,
//     );
//     append_events::<ChannelEvents::OpenAck>(
//         &mut events,
//         extract_events(height, block_events, "channel_open_ack", "channel_id"),
//         height,
//     );
//     append_events::<ChannelEvents::OpenConfirm>(
//         &mut events,
//         extract_events(height, block_events, "channel_open_confirm", "channel_id"),
//         height,
//     );
//     append_events::<ChannelEvents::SendPacket>(
//         &mut events,
//         extract_events(height, block_events, "send_packet", "packet_data_hex"),
//         height,
//     );
//     append_events::<ChannelEvents::CloseInit>(
//         &mut events,
//         extract_events(height, block_events, "channel_close_init", "channel_id"),
//         height,
//     );
//     append_events::<ChannelEvents::CloseConfirm>(
//         &mut events,
//         extract_events(height, block_events, "channel_close_confirm", "channel_id"),
//         height,
//     );
//     // // extract cross chain query event from block_events
//     // if let Ok(ccq) = CrossChainQueryPacket::extract_query_event(block_events) {
//     //     events.push(IbcEventWithHeight::new(ccq, height));
//     // }

//     events
// }