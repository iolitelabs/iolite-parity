//TODO: <IOLITE> copyright

//use serde::{Serialize};
//use serde::ser::SerializeStruct;
use ethereum_types::{U256, Address};
use rlp::{self, RlpStream, DecoderError, Encodable};

#[derive(Debug)]
pub struct MetaLogs {
    //TODO: <IOLITE> should we own this value?
    //logs: &[MetaLog],
    pub logs: Vec<MetaLog>,
}

impl rlp::Decodable for MetaLogs {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        if rlp.is_empty() {
            Ok(MetaLogs { logs: vec![] })
        } else {
            Ok(rlp.as_val()?)
        }
    }
}

impl rlp::Encodable for MetaLogs {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(1);
        s.append(&vec![]);
        // Need to implement rlp::Encodable for std::vec::Vec<metalogs::MetaLog>
        //s.append(&self.logs);
    }
}

impl MetaLogs {
    pub fn new() -> Self {
        MetaLogs {
            logs: vec![],
        }
    }

    // Will fail on compile
    pub fn logs(&self) -> &[MetaLog] {
        &self.logs
    }

    pub fn push(&mut self, recipient: Address, amount: U256) {
        self.logs.push(MetaLog { recipient: recipient, amount: amount });
    }
}

//TODO: Implement Iterator for MetaLogs

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
//TODO: <IOLITE> need to implement rlp::Encodable for this field
pub struct MetaLog {
    #[serde(rename="to")]
    pub recipient: Address,
    #[serde(rename="value")]
    pub amount: U256,
}

impl rlp::Decodable for MetaLog {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        Ok(MetaLog {
            recipient: rlp.val_at(0)?,
            amount: rlp.val_at(1)?,
        })
    }
}

impl rlp::Encodable for MetaLog {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(2);
        s.append(&self.recipient);
        s.append(&self.amount);
    }
}
