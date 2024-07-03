use super::{aggrelite, ics07_tendermint};

#[derive(Debug)]
pub enum HeaderType {
    Tendermint(ics07_tendermint::header::Header),
    Aggrelite(aggrelite::header::Header)
}

#[derive(Debug)]
pub enum AdjustHeadersType {
    Tendermint(TendermintAdjustHeaders),
    Aggrelite(AggreliteAdjustHeaders),
}

#[derive(Debug)]
pub struct TendermintAdjustHeaders {
    pub target_header: ics07_tendermint::header::Header,
    pub supporting_headers: Vec<ics07_tendermint::header::Header>,
}

#[derive(Debug)]
pub struct AggreliteAdjustHeaders {
    pub target_header: aggrelite::header::Header,
    pub supporting_headers: Vec<aggrelite::header::Header>,
}