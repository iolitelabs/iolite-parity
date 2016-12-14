// Copyright 2015, 2016 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

//! Dapps settings de/serialization.

use std::io;
use std::collections::HashMap;
use serde_json;
use hash;

type DappId = String;

/// Settings for specific dapp.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DappsSettings {
	/// A list of accounts this Dapp can see.
	pub accounts: Vec<hash::Address>,
}

impl DappsSettings {
	/// Read a hash map of DappId -> DappsSettings
	pub fn read_dapps_settings<R, S>(reader: R) -> Result<HashMap<DappId, S>, serde_json::Error> where
		R: io::Read,
		S: From<DappsSettings> + Clone,
	{
		serde_json::from_reader(reader).map(|ok: HashMap<DappId, DappsSettings>|
			ok.into_iter().map(|(a, m)| (a.into(), m.into())).collect()
		)
	}

	/// Write a hash map of DappId -> DappsSettings
	pub fn write_dapps_settings<W, S>(m: &HashMap<DappId, S>, writer: &mut W) -> Result<(), serde_json::Error> where
		W: io::Write,
		S: Into<DappsSettings> + Clone,
	{
		serde_json::to_writer(writer, &m.iter().map(|(a, m)| (a.clone().into(), m.clone().into())).collect::<HashMap<DappId, DappsSettings>>())
	}
}
