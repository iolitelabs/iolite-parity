//TODO: <IOLITE> copyright
use std::ops::Deref;
use rlp::{self};

use types::metalogs::MetaLogs;
use meta::base_meta_executor::{BaseMetaExecutor, MetaExecute, Bytes};

use ethereum_types::{Address, U256};
use types::metalogs::MetaLog;

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

fn to_hex_string(bytes: Vec<u8>) -> String {
  let mut strs: Vec<String> = bytes.iter()
                               .map(|b| format!("{:02x}", b))
                               .collect();
  strs.insert(0, "0".to_string());
  strs.insert(1, "x".to_string());
  strs.join("")
}
fn print_rlp<T: rlp::Encodable + ::std::fmt::Display>(s: &T) {
    println!("RLP of {}: {}", s, to_hex_string(rlp::encode(s).into_vec()));
}
fn test_rlp() {
        let addr: &'static str = "0xdc4014def24ee392bf36e465c65ab0a3ed52fe5b";
        let address = Address::from(addr);
        print_rlp(&address);

        let metalog = MetaLog {
            recipient: Address::from(addr),
            amount: U256::from(5),
        };
        println!("Rlp of Metalog: {}", to_hex_string(rlp::encode(&metalog).into_vec()));

        let mut metalogs = MetaLogs::new();
        metalogs.push(address, U256::from(5));
        println!("Rlp of Metalogs: {}", to_hex_string(rlp::encode(&metalogs).into_vec()));
}

impl MetaExecute for SimpleMetaExecutor {
    fn execute(&mut self) -> Result<MetaLogs, String> {
        test_rlp();
        if self.metadata.len() == 0 {
            println!("[iolite] Error! Metadata is empty.");
            return Err("[iolite] Error! Metadata is empty.".to_string());
        }

        println!("Trying to decode metalogs.");
        let meta: MetaLogs = match rlp::decode(&self.metadata) {
            Ok(meta) => meta,
            Err(err) => { println!("{}", err); return Err(err.to_string()) },
        };

        for log in meta.logs() {
            info!("[iolite] Decoded metadata. To: {recipient}, Value: {value};",
                  recipient = log.recipient, value = log.amount);
        }
        Ok(meta)
    }
}
