//TODO: <IOLITE> copyright

use ethereum_types::{U256, Address};
use rlp::{self};
use serde::ser::{Serialize, Serializer, SerializeSeq};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MetaLogs {
    pub logs: Vec<MetaLog>,
}

impl rlp::Decodable for MetaLogs {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        if rlp.is_empty() {
            return Ok(MetaLogs { logs: vec![] });
        }

        let mut metalogs = MetaLogs { logs: vec![], };
        let num_items = rlp.item_count()?;
        for i in 0..num_items {
            metalogs.logs.push(rlp.val_at(i)?);
        }

        Ok(metalogs)
    }
}

impl rlp::Encodable for MetaLogs {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(self.logs.len());
        for log in &self.logs {
            s.append(log);
        }
    }
}

impl Serialize for MetaLogs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.logs.len()))?;
        for e in self.logs() {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

impl MetaLogs {
    pub fn new() -> Self {
        MetaLogs {
            logs: vec![],
        }
    }

    pub fn logs(&self) -> &[MetaLog] {
        &self.logs
    }

    pub fn mut_logs(&mut self) -> &[MetaLog] {
        &mut self.logs
    }

    pub fn push(&mut self, recipient: Address, amount: U256) {
        self.logs.push(MetaLog { recipient: recipient, amount: amount });
    }
}

//TODO: Implement Iterator for MetaLogs

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
