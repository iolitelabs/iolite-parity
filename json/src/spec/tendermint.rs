// Copyright 2015, 2016 Ethcore (UK) Ltd.
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

//! Tendermint params deserialization.

use uint::Uint;
use hash::Address;

/// Tendermint params deserialization.
#[derive(Debug, PartialEq, Deserialize)]
pub struct TendermintParams {
	/// Gas limit divisor.
	#[serde(rename="gasLimitBoundDivisor")]
	pub gas_limit_bound_divisor: Uint,
	/// Valid authorities
	pub authorities: Vec<Address>,
	/// Propose step timeout in milliseconds.
	#[serde(rename="timeoutPropose")]
	pub timeout_propose: Option<Uint>,
	/// Prevote step timeout in milliseconds.
	#[serde(rename="timeoutPrevote")]
	pub timeout_prevote: Option<Uint>,
	/// Precommit step timeout in milliseconds.
	#[serde(rename="timeoutPrecommit")]
	pub timeout_precommit: Option<Uint>,
	/// Commit step timeout in milliseconds.
	#[serde(rename="timeoutCommit")]
	pub timeout_commit: Option<Uint>,
}

/// Tendermint engine deserialization.
#[derive(Debug, PartialEq, Deserialize)]
pub struct Tendermint {
	/// Ethash params.
	pub params: TendermintParams,
}

#[cfg(test)]
mod tests {
	use serde_json;
	use spec::tendermint::Tendermint;

	#[test]
	fn basic_authority_deserialization() {
		let s = r#"{
			"params": {
				"gasLimitBoundDivisor": "0x0400",
				"authorities" : ["0xc6d9d2cd449a754c494264e1809c50e34d64562b"]
			}
		}"#;

		let _deserialized: Tendermint = serde_json::from_str(s).unwrap();
	}
}
