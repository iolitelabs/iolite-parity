use std::ops::Deref;
use std::fmt;
use ethereum_types::Address;
use rlp::{self};

pub type Bytes = Vec<u8>;

#[derive(Debug)]
pub struct BusinessMetadata {
    data: Metadata
}

impl Deref for BusinessMetadata {
    type Target = Metadata;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl rlp::Decodable for BusinessMetadata {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        Ok(BusinessMetadata {
            data: rlp.as_val()?,
        })
    }
}

impl rlp::Encodable for BusinessMetadata {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(1);
        s.append(&self.data);
    }
}

impl fmt::Display for BusinessMetadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BusinessMetadata: {}", &self.data)
    }
}

impl BusinessMetadata {
    pub fn is_valid(metadata: &[u8]) -> Result<bool, String> {
        // Empty metadata is always valid
        if metadata.is_empty() {
            return Ok(true);
        }

        let _: BusinessMetadata = rlp::decode(metadata).map_err(|err| err.to_string())?;
        Ok(true)
    }

    /// Unpacks metadata and calculates required gas for `Input` field only
    pub fn gas_required_for(metadata: &[u8]) -> Result<u64, rlp::DecoderError> {
        if metadata.is_empty() {
            return Ok(0u64);
        }

        let business_metadata: BusinessMetadata = rlp::decode(metadata)?;
        let gas_required = BusinessMetadata::gas_required_for_raw(&business_metadata.input);
        Ok(gas_required)
    }

    /// Calculates required gas for given amount of any raw data.
    fn gas_required_for_raw(data: &[u8]) -> u64 {
        //TODO: <IOLITE> don't use hardcoded values as
        // tx_data_non_zero_gas and tx_data_zero_gas is in ethcore/vm/src/schedule.rs
        //let tx_gas = 21000u64;
        let tx_data_zero_gas = 4u64;
        let tx_data_non_zero_gas = 68u64;

        data.iter().fold(
            0, //tx_gas,
            |g, b| g + (match *b { 0 => tx_data_zero_gas, _ => tx_data_non_zero_gas }) as u64
        )
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]//, Serialize)]
pub struct Metadata {
    //#[serde(rename="to")]
    pub business: Address,
    //#[serde(rename="input")]
    pub input: Bytes,
}

impl rlp::Decodable for Metadata {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        if rlp.is_empty() {
            return Err(rlp::DecoderError::Custom("Can't decode business metadata from given rlp."));
        }

        let metadata = Metadata {
            business: rlp.val_at(0)?,
            input: rlp.val_at(1)?,
        };

        Ok(metadata)
    }
}

impl rlp::Encodable for Metadata {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(2);
        s.append(&self.business);
        s.append(&self.input);
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Business: {}; Input(len): {}; Input: {:x?}",
               self.business, self.input.len(), self.input)
    }
}
