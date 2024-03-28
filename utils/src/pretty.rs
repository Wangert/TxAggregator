use std::fmt::{Display, Error, Formatter};

use tendermint::block::signed_header::SignedHeader;
use tendermint::validator::Set as ValidatorSet;

pub struct PrettySlice<'a, T>(pub &'a [T]);

impl<'a, T: Display> Display for PrettySlice<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "[ ")?;
        let mut vec_iterator = self.0.iter().peekable();
        while let Some(element) = vec_iterator.next() {
            write!(f, "{element}")?;
            // If it is not the last element, add separator.
            if vec_iterator.peek().is_some() {
                write!(f, ", ")?;
            }
        }
        write!(f, " ]")
    }
}

pub struct PrettySignedHeader<'a>(pub &'a SignedHeader);

impl Display for PrettySignedHeader<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "SignedHeader {{ header: {{ chain_id: {}, height: {} }}, commit: {{ height: {} }} }}",
            self.0.header.chain_id, self.0.header.height, self.0.commit.height
        )
    }
}

pub struct PrettyValidatorSet<'a>(pub &'a ValidatorSet);

impl Display for PrettyValidatorSet<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let validator_addresses: Vec<_> = self
            .0
            .validators()
            .iter()
            .map(|validator| validator.address)
            .collect();
        if let Some(proposer) = self.0.proposer() {
            match &proposer.name {
                Some(name) => write!(f, "PrettyValidatorSet {{ validators: {}, proposer: {}, total_voting_power: {} }}", PrettySlice(&validator_addresses), name, self.0.total_voting_power()),
                None =>  write!(f, "PrettyValidatorSet {{ validators: {}, proposer: None, total_voting_power: {} }}", PrettySlice(&validator_addresses), self.0.total_voting_power()),
            }
        } else {
            write!(
                f,
                "PrettyValidatorSet {{ validators: {}, proposer: None, total_voting_power: {} }}",
                PrettySlice(&validator_addresses),
                self.0.total_voting_power()
            )
        }
    }
}