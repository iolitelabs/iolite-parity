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
