#[macro_use]
extern crate log;
extern crate rlp;

type Bytes = Vec<u8>;

use types::metalogs::MetaLogs;
use v1::types::{U256};
use executive::Executive;
