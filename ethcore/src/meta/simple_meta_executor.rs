use base_meta_executor::{BaseMetaExecutor, MetaExecute};

struct SimpleMetaExecutor {
    executor: BaseMetaExecutor,
}

impl Deref for SimpleMetaExecutor {
    type Target = BaseMetaExecutor;

    fn deref(&self) -> &Self::Type {
        &self.executor
    }
}

impl SimpleMetaExecutor {
    fn new(metadata: Bytes) -> Self {
        SimpleMetaExecutor {
            executor: BaseMetaExecutor { metadata: metadata },
        }
    }
}

impl MetaExecute for SimpleMetaExecutor {
    fn execute(&self) -> Result<MetaLogs, ()> {
        if self.metadata.len() == 0 {
            return Err("[iolite] Error! Metadata is empty.");
        }

        match rlp::decode(&self.metadata) {
            Ok(meta) => {
                for log in meta.logs() {
                    info!("[iolite] Decoded metadata. To: {recipient}, Value: {value};",
                          recipient = log.recipient, value = log.amount);
                }
                meta
            },
            Err(err) => err,
        }
    }
}
