//TODO: <IOLITE> copyright
use std::ops::Deref;
use rlp::{self};

use types::metalogs::MetaLogs;
use meta::base_meta_executor::{BaseMetaExecutor, MetaExecute, Bytes};

pub struct SimpleMetaExecutor {
    executor: BaseMetaExecutor,
}

impl Deref for SimpleMetaExecutor {
    type Target = BaseMetaExecutor;

    fn deref(&self) -> &Self::Target{
        &self.executor
    }
}

impl SimpleMetaExecutor {
    pub fn new(metadata: Bytes) -> Self {
        SimpleMetaExecutor {
            executor: BaseMetaExecutor { metadata: metadata },
        }
    }
}

impl<'a> MetaExecute<'a> for SimpleMetaExecutor {
    fn execute(&'a mut self) -> Result<MetaLogs, String> {
        if self.metadata.len() == 0 {
            return Err("[iolite] Error! Metadata is empty.".to_string());
        }

        let meta: MetaLogs = match rlp::decode(&self.metadata) {
            Ok(meta) => meta,
            Err(err) => return Err(err.to_string()),
        };

        for log in meta.logs() {
            info!("[iolite] Decoded metadata. To: {recipient}, Value: {value};",
                  recipient = log.recipient, value = log.amount);
        }
        Ok(meta)
    }
}
