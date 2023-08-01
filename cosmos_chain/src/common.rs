use std::time::Duration;

use ibc_proto::google::protobuf::Duration as ProtobufDuration;

use crate::error::Error;

pub fn parse_protobuf_duration(duration: ProtobufDuration) -> Duration {
    Duration::new(duration.seconds as u64, duration.nanos as u32)
}
