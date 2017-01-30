// Copyright 2015, 2016, 2017 Parity Technologies (UK) Ltd.
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

use std::io::{Read, Write};
use serde::{Deserialize, Deserializer, Error};
use serde::de::{Visitor, MapVisitor};
use serde_json;
use super::{Uuid, Version, Crypto, H160};

/// Key file as stored in vaults
#[derive(Debug, PartialEq, Serialize)]
pub struct VaultKeyFile {
	/// Key id
	pub id: Uuid,
	/// Key version
	pub version: Version,
	/// Encrypted secret
	pub crypto: Crypto,
	/// Encrypted serialized `VaultKeyMeta`
	pub metacrypto: Crypto,
}

/// Data, stored in `VaultKeyFile::metacrypto`
#[derive(Debug, PartialEq, Serialize)]
pub struct VaultKeyMeta {
	/// Key address
	pub address: H160,
	/// Key name
	pub name: Option<String>,
	/// Key metadata
	pub meta: Option<String>,
}

enum VaultKeyFileField {
	Id,
	Version,
	Crypto,
	MetaCrypto,
}

enum VaultKeyMetaField {
	Address,
	Name,
	Meta,
}

impl Deserialize for VaultKeyFileField {
	fn deserialize<D>(deserializer: &mut D) -> Result<VaultKeyFileField, D::Error>
		where D: Deserializer
	{
		deserializer.deserialize(VaultKeyFileFieldVisitor)
	}
}

struct VaultKeyFileFieldVisitor;

impl Visitor for VaultKeyFileFieldVisitor {
	type Value = VaultKeyFileField;

	fn visit_str<E>(&mut self, value: &str) -> Result<Self::Value, E>
		where E: Error
	{
		match value {
			"id" => Ok(VaultKeyFileField::Id),
			"version" => Ok(VaultKeyFileField::Version),
			"crypto" => Ok(VaultKeyFileField::Crypto),
			"metacrypto" => Ok(VaultKeyFileField::MetaCrypto),
			_ => Err(Error::custom(format!("Unknown field: '{}'", value))),
		}
	}
}

impl Deserialize for VaultKeyFile {
	fn deserialize<D>(deserializer: &mut D) -> Result<VaultKeyFile, D::Error>
		where D: Deserializer
	{
		static FIELDS: &'static [&'static str] = &["id", "version", "crypto", "metacrypto"];
		deserializer.deserialize_struct("VaultKeyFile", FIELDS, VaultKeyFileVisitor)
	}
}

struct VaultKeyFileVisitor;

impl Visitor for VaultKeyFileVisitor {
	type Value = VaultKeyFile;

	fn visit_map<V>(&mut self, mut visitor: V) -> Result<Self::Value, V::Error>
		where V: MapVisitor
	{
		let mut id = None;
		let mut version = None;
		let mut crypto = None;
		let mut metacrypto = None;

		loop {
			match visitor.visit_key()? {
				Some(VaultKeyFileField::Id) => { id = Some(visitor.visit_value()?); }
				Some(VaultKeyFileField::Version) => { version = Some(visitor.visit_value()?); }
				Some(VaultKeyFileField::Crypto) => { crypto = Some(visitor.visit_value()?); }
				Some(VaultKeyFileField::MetaCrypto) => { metacrypto = Some(visitor.visit_value()?); }
				None => { break; }
			}
		}

		let id = match id {
			Some(id) => id,
			None => visitor.missing_field("id")?,
		};

		let version = match version {
			Some(version) => version,
			None => visitor.missing_field("version")?,
		};

		let crypto = match crypto {
			Some(crypto) => crypto,
			None => visitor.missing_field("crypto")?,
		};

		let metacrypto = match metacrypto {
			Some(metacrypto) => metacrypto,
			None => visitor.missing_field("metacrypto")?,
		};

		visitor.end()?;

		let result = VaultKeyFile {
			id: id,
			version: version,
			crypto: crypto,
			metacrypto: metacrypto,
		};

		Ok(result)
	}
}

impl Deserialize for VaultKeyMetaField {
	fn deserialize<D>(deserializer: &mut D) -> Result<VaultKeyMetaField, D::Error>
		where D: Deserializer
	{
		deserializer.deserialize(VaultKeyMetaFieldVisitor)
	}
}

struct VaultKeyMetaFieldVisitor;

impl Visitor for VaultKeyMetaFieldVisitor {
	type Value = VaultKeyMetaField;

	fn visit_str<E>(&mut self, value: &str) -> Result<Self::Value, E>
		where E: Error
	{
		match value {
			"address" => Ok(VaultKeyMetaField::Address),
			"name" => Ok(VaultKeyMetaField::Name),
			"meta" => Ok(VaultKeyMetaField::Meta),
			_ => Err(Error::custom(format!("Unknown field: '{}'", value))),
		}
	}
}

impl Deserialize for VaultKeyMeta {
	fn deserialize<D>(deserializer: &mut D) -> Result<VaultKeyMeta, D::Error>
		where D: Deserializer
	{
		static FIELDS: &'static [&'static str] = &["address", "name", "meta"];
		deserializer.deserialize_struct("VaultKeyMeta", FIELDS, VaultKeyMetaVisitor)
	}
}

struct VaultKeyMetaVisitor;

impl Visitor for VaultKeyMetaVisitor {
	type Value = VaultKeyMeta;

	fn visit_map<V>(&mut self, mut visitor: V) -> Result<Self::Value, V::Error>
		where V: MapVisitor
	{
		let mut address = None;
		let mut name = None;
		let mut meta = None;

		loop {
			match visitor.visit_key()? {
				Some(VaultKeyMetaField::Address) => { address = Some(visitor.visit_value()?); }
				Some(VaultKeyMetaField::Name) => { name = Some(visitor.visit_value()?); }
				Some(VaultKeyMetaField::Meta) => { meta = Some(visitor.visit_value()?); }
				None => { break; }
			}
		}

		let address = match address {
			Some(address) => address,
			None => visitor.missing_field("address")?,
		};

		visitor.end()?;

		let result = VaultKeyMeta {
			address: address,
			name: name,
			meta: meta,
		};

		Ok(result)
	}
}

impl VaultKeyFile {
	pub fn load<R>(reader: R) -> Result<Self, serde_json::Error> where R: Read {
		serde_json::from_reader(reader)
	}

	pub fn write<W>(&self, writer: &mut W) -> Result<(), serde_json::Error> where W: Write {
		serde_json::to_writer(writer, self)
	}
}

impl VaultKeyMeta {
	pub fn load(bytes: &[u8]) -> Result<Self, serde_json::Error> {
		serde_json::from_slice(&bytes)
	}

	pub fn write(&self) -> Result<Vec<u8>, serde_json::Error> {
		let s = serde_json::to_string(self)?;
		Ok(s.as_bytes().into())
	}
}

#[cfg(test)]
mod test {
	use serde_json;
	use json::{VaultKeyFile, Version, Crypto, Cipher, Aes128Ctr, Kdf, Pbkdf2, Prf};

	#[test]
	fn to_and_from_json() {
		let file = VaultKeyFile {
			id: "08d82c39-88e3-7a71-6abb-89c8f36c3ceb".into(),
			version: Version::V3,
			crypto: Crypto {
				cipher: Cipher::Aes128Ctr(Aes128Ctr {
					iv: "fecb968bbc8c7e608a89ebcfe53a41d0".into(),
				}),
				ciphertext: "4befe0a66d9a4b6fec8e39eb5c90ac5dafdeaab005fff1af665fd1f9af925c91".into(),
				kdf: Kdf::Pbkdf2(Pbkdf2 {
					c: 10240,
					dklen: 32,
					prf: Prf::HmacSha256,
					salt: "f17731e84ecac390546692dbd4ccf6a3a2720dc9652984978381e61c28a471b2".into(),
				}),
				mac: "7c7c3daafb24cf11eb3079dfb9064a11e92f309a0ee1dd676486bab119e686b7".into(),
			},
			metacrypto: Crypto {
				cipher: Cipher::Aes128Ctr(Aes128Ctr {
					iv: "9c353fb3f894fc05946843616c26bb3f".into(),
				}),
				ciphertext: "fef0d113d7576c1702daf380ad6f4c5408389e57991cae2a174facd74bd549338e1014850bddbab7eb486ff5f5c9c5532800c6a6d4db2be2212cd5cd3769244ab230e1f369e8382a9e6d7c0a".into(),
				kdf: Kdf::Pbkdf2(Pbkdf2 {
					c: 10240,
					dklen: 32,
					prf: Prf::HmacSha256,
					salt: "aca82865174a82249a198814b263f43a631f272cbf7ed329d0f0839d259c652a".into(),
				}),
				mac: "b7413946bfe459d2801268dc331c04b3a84d92be11ef4dd9a507f895e8d9b5bd".into(),
			}
		};

		let serialized = serde_json::to_string(&file).unwrap();
		let deserialized = serde_json::from_str(&serialized).unwrap();

		assert_eq!(file, deserialized);
	}
}
