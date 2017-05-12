// Copyright 2015-2017 Parity Technologies (UK) Ltd.
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

use std::io;
use std::time;
use std::sync::{Arc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::collections::btree_map::Entry;
use std::net::{SocketAddr, IpAddr};
use futures::{finished, failed, Future, Stream, BoxFuture};
use futures_cpupool::CpuPool;
use parking_lot::{RwLock, Mutex};
use tokio_io::IoFuture;
use tokio_core::reactor::{Handle, Remote, Interval};
use tokio_core::net::{TcpListener, TcpStream};
use ethkey::{Public, Secret, KeyPair, Signature, Random, Generator};
use key_server_cluster::{Error, NodeId, SessionId, AclStorage, KeyStorage, DocumentEncryptedKeyShadow};
use key_server_cluster::message::{self, Message, ClusterMessage, EncryptionMessage, DecryptionMessage};
use key_server_cluster::decryption_session::{SessionImpl as DecryptionSessionImpl, SessionState as DecryptionSessionState,
	SessionParams as DecryptionSessionParams, Session as DecryptionSession, DecryptionSessionId};
use key_server_cluster::encryption_session::{SessionImpl as EncryptionSessionImpl, SessionState as EncryptionSessionState,
	SessionParams as EncryptionSessionParams, Session as EncryptionSession};
use key_server_cluster::io::{DeadlineStatus, ReadMessage, SharedTcpStream, read_encrypted_message, WriteMessage, write_encrypted_message};
use key_server_cluster::net::{accept_connection as net_accept_connection, connect as net_connect, Connection as NetConnection};

/// Maintain interval (seconds). Every MAINTAIN_INTERVAL seconds node:
/// 1) checks if connected nodes are responding to KeepAlive messages
/// 2) tries to connect to disconnected nodes
/// 3) checks if enc/dec sessions are time-outed
const MAINTAIN_INTERVAL: u64 = 10;

/// When no messages have been received from node within KEEP_ALIVE_SEND_INTERVAL seconds,
/// we must send KeepAlive message to the node to check if it still responds to messages.
const KEEP_ALIVE_SEND_INTERVAL: u64 = 30;
/// When no messages have been received from node within KEEP_ALIVE_DISCONNECT_INTERVAL seconds,
/// we must treat this node as non-responding && disconnect from it.
const KEEP_ALIVE_DISCONNECT_INTERVAL: u64 = 60;

/// When there are no encryption session-related messages for ENCRYPTION_SESSION_TIMEOUT_INTERVAL seconds,
/// we must treat this session as stalled && finish it with an error.
/// This timeout is for cases when node is responding to KeepAlive messages, but intentionally ignores
/// session messages.
const ENCRYPTION_SESSION_TIMEOUT_INTERVAL: u64 = 60;

/// When there are no decryption session-related messages for DECRYPTION_SESSION_TIMEOUT_INTERVAL seconds,
/// we must treat this session as stalled && finish it with an error.
/// This timeout is for cases when node is responding to KeepAlive messages, but intentionally ignores
/// session messages.
const DECRYPTION_SESSION_TIMEOUT_INTERVAL: u64 = 60;

/// Encryption sesion timeout interval. It works
/// Empty future.
type BoxedEmptyFuture = BoxFuture<(), ()>;

/// Cluster interface for external clients.
pub trait ClusterClient: Send + Sync {
	/// Get cluster state.
	fn cluster_state(&self) -> ClusterState;
	/// Start new encryption session.
	fn new_encryption_session(&self, session_id: SessionId, threshold: usize) -> Result<Arc<EncryptionSession>, Error>;
	/// Start new decryption session.
	fn new_decryption_session(&self, session_id: SessionId, requestor_signature: Signature, is_shadow_decryption: bool) -> Result<Arc<DecryptionSession>, Error>;

	#[cfg(test)]
	/// Ask node to make 'faulty' encryption sessions.
	fn make_faulty_encryption_sessions(&self);
	#[cfg(test)]
	/// Get active encryption session with given id.
	fn encryption_session(&self, session_id: &SessionId) -> Option<Arc<EncryptionSessionImpl>>;
	#[cfg(test)]
	/// Try connect to disconnected nodes.
	fn connect(&self);
}

/// Cluster access for single encryption/decryption participant.
pub trait Cluster: Send + Sync {
	/// Broadcast message to all other nodes.
	fn broadcast(&self, message: Message) -> Result<(), Error>;
	/// Send message to given node.
	fn send(&self, to: &NodeId, message: Message) -> Result<(), Error>;
}

#[derive(Clone)]
/// Cluster initialization parameters.
pub struct ClusterConfiguration {
	/// Number of threads reserved by cluster.
	pub threads: usize,
	/// Allow connecting to 'higher' nodes.
	pub allow_connecting_to_higher_nodes: bool,
	/// KeyPair this node holds.
	pub self_key_pair: KeyPair,
	/// Interface to listen to.
	pub listen_address: (String, u16),
	/// Cluster nodes.
	pub nodes: BTreeMap<NodeId, (String, u16)>,
	/// Reference to key storage
	pub key_storage: Arc<KeyStorage>,
	/// Reference to ACL storage
	pub acl_storage: Arc<AclStorage>,
}

/// Cluster state.
pub struct ClusterState {
	/// Nodes, to which connections are established.
	pub connected: BTreeSet<NodeId>,
}

/// Network cluster implementation.
pub struct ClusterCore {
	/// Handle to the event loop.
	handle: Handle,
	/// Listen address.
	listen_address: SocketAddr,
	/// Cluster data.
	data: Arc<ClusterData>,
}

/// Network cluster client interface implementation.
pub struct ClusterClientImpl {
	/// Cluster data.
	data: Arc<ClusterData>,
}

/// Network cluster view. It is a communication channel, required in single session.
pub struct ClusterView {
	core: Arc<Mutex<ClusterViewCore>>,
}

/// Cross-thread shareable cluster data.
pub struct ClusterData {
	/// Cluster configuration.
	config: ClusterConfiguration,
	/// Handle to the event loop.
	handle: Remote,
	/// Handle to the cpu thread pool.
	pool: CpuPool,
	/// KeyPair this node holds.
	self_key_pair: KeyPair,
	/// Connections data.
	connections: ClusterConnections,
	/// Active sessions data.
	sessions: ClusterSessions,
}

/// Connections that are forming the cluster.
pub struct ClusterConnections {
	/// Self node id.
	pub self_node_id: NodeId,
	/// All known other key servers.
	pub nodes: BTreeMap<NodeId, SocketAddr>,
	/// Active connections to key servers.
	pub connections: RwLock<BTreeMap<NodeId, Arc<Connection>>>,
}

/// Active sessions on this cluster.
pub struct ClusterSessions {
	/// Self node id.
	pub self_node_id: NodeId,
	/// All nodes ids.
	pub nodes: BTreeSet<NodeId>,
	/// Reference to key storage
	pub key_storage: Arc<KeyStorage>,
	/// Reference to ACL storage
	pub acl_storage: Arc<AclStorage>,
	/// Active encryption sessions.
	pub encryption_sessions: RwLock<BTreeMap<SessionId, QueuedEncryptionSession>>,
	/// Active decryption sessions.
	pub decryption_sessions: RwLock<BTreeMap<DecryptionSessionId, QueuedDecryptionSession>>,
	/// Make faulty encryption sessions.
	pub make_faulty_encryption_sessions: AtomicBool,
}

/// Encryption session and its message queue.
pub struct QueuedEncryptionSession {
	/// Session master.
	pub master: NodeId,
	/// Cluster view.
	pub cluster_view: Arc<ClusterView>,
	/// Last received message time.
	pub last_message_time: time::Instant,
	/// Encryption session.
	pub session: Arc<EncryptionSessionImpl>,
	/// Messages queue.
	pub queue: VecDeque<(NodeId, EncryptionMessage)>,
}

/// Decryption session and its message queue.
pub struct QueuedDecryptionSession {
	/// Session master.
	pub master: NodeId,
	/// Cluster view.
	pub cluster_view: Arc<ClusterView>,
	/// Last received message time.
	pub last_message_time: time::Instant,
	/// Decryption session.
	pub session: Arc<DecryptionSessionImpl>,
	/// Messages queue.
	pub queue: VecDeque<(NodeId, DecryptionMessage)>,
}

/// Cluster view core.
struct ClusterViewCore {
	/// Cluster reference.
	cluster: Arc<ClusterData>,
	/// Subset of nodes, required for this session.
	nodes: BTreeSet<NodeId>,
}

/// Connection to single node.
pub struct Connection {
	/// Node id.
	node_id: NodeId,
	/// Node address.
	node_address: SocketAddr,
	/// Is inbound connection?
	is_inbound: bool,
	/// Tcp stream.
	stream: SharedTcpStream,
	/// Connection key.
	key: KeyPair,
	/// Last message time.
	last_message_time: Mutex<time::Instant>,
}

/// Encryption session implementation, which removes session from cluster on drop.
struct EncryptionSessionWrapper {
	/// Wrapped session.
	session: Arc<EncryptionSession>,
	/// Session Id.
	session_id: SessionId,
	/// Cluster data reference.
	cluster: Weak<ClusterData>,
}

/// Decryption session implementation, which removes session from cluster on drop.
struct DecryptionSessionWrapper {
	/// Wrapped session.
	session: Arc<DecryptionSession>,
	/// Session Id.
	session_id: SessionId,
	/// Session sub id.
	access_key: Secret,
	/// Cluster data reference.
	cluster: Weak<ClusterData>,
}

impl ClusterCore {
	pub fn new(handle: Handle, config: ClusterConfiguration) -> Result<Arc<Self>, Error> {
		let listen_address = make_socket_address(&config.listen_address.0, config.listen_address.1)?;
		let connections = ClusterConnections::new(&config)?;
		let sessions = ClusterSessions::new(&config);
		let data = ClusterData::new(&handle, config, connections, sessions);

		Ok(Arc::new(ClusterCore {
			handle: handle,
			listen_address: listen_address,
			data: data,
		}))
	}

	/// Create new client interface.
	pub fn client(&self) -> Arc<ClusterClient> {
		Arc::new(ClusterClientImpl::new(self.data.clone()))
	}

	#[cfg(test)]
	/// Get cluster configuration.
	pub fn config(&self) -> &ClusterConfiguration {
		&self.data.config
	}

	#[cfg(test)]
	/// Get connection to given node.
	pub fn connection(&self, node: &NodeId) -> Option<Arc<Connection>> {
		self.data.connection(node)
	}

	/// Run cluster.
	pub fn run(&self) -> Result<(), Error> {
		self.run_listener()
			.and_then(|_| self.run_connections())?;

		// schedule maintain procedures
		ClusterCore::schedule_maintain(&self.handle, self.data.clone());

		Ok(())
	}

	/// Start listening for incoming connections.
	pub fn run_listener(&self) -> Result<(), Error> {
		// start listeining for incoming connections
		self.handle.spawn(ClusterCore::listen(&self.handle, self.data.clone(), self.listen_address.clone())?);
		Ok(())
	}

	/// Start connecting to other nodes.
	pub fn run_connections(&self) -> Result<(), Error> {
		// try to connect to every other peer
		ClusterCore::connect_disconnected_nodes(self.data.clone());
		Ok(())
	}

	/// Connect to peer.
	fn connect(data: Arc<ClusterData>, node_address: SocketAddr) {
		data.handle.clone().spawn(move |handle| {
			data.pool.clone().spawn(ClusterCore::connect_future(handle, data, node_address))
		})
	}

	/// Connect to socket using given context and handle.
	fn connect_future(handle: &Handle, data: Arc<ClusterData>, node_address: SocketAddr) -> BoxedEmptyFuture {
		let disconnected_nodes = data.connections.disconnected_nodes().keys().cloned().collect();
		net_connect(&node_address, handle, data.self_key_pair.clone(), disconnected_nodes)
			.then(move |result| ClusterCore::process_connection_result(data, false, result))
			.then(|_| finished(()))
			.boxed()
	}

	/// Start listening for incoming connections.
	fn listen(handle: &Handle, data: Arc<ClusterData>, listen_address: SocketAddr) -> Result<BoxedEmptyFuture, Error> {
		Ok(TcpListener::bind(&listen_address, &handle)?
			.incoming()
			.and_then(move |(stream, node_address)| {
				ClusterCore::accept_connection(data.clone(), stream, node_address);
				Ok(())
			})
			.for_each(|_| Ok(()))
			.then(|_| finished(()))
			.boxed())
	}

	/// Accept connection.
	fn accept_connection(data: Arc<ClusterData>, stream: TcpStream, node_address: SocketAddr) {
		data.handle.clone().spawn(move |handle| {
			data.pool.clone().spawn(ClusterCore::accept_connection_future(handle, data, stream, node_address))
		})
	}

	/// Accept connection future.
	fn accept_connection_future(handle: &Handle, data: Arc<ClusterData>, stream: TcpStream, node_address: SocketAddr) -> BoxedEmptyFuture {
		let disconnected_nodes = data.connections.disconnected_nodes().keys().cloned().collect();
		net_accept_connection(node_address, stream, handle, data.self_key_pair.clone(), disconnected_nodes)
			.then(move |result| ClusterCore::process_connection_result(data, true, result))
			.then(|_| finished(()))
			.boxed()
	}

	/// Schedule mainatain procedures.
	fn schedule_maintain(handle: &Handle, data: Arc<ClusterData>) {
		let d = data.clone();
		let interval: BoxedEmptyFuture = Interval::new(time::Duration::new(MAINTAIN_INTERVAL, 0), handle)
			.expect("failed to create interval")
			.and_then(move |_| Ok(ClusterCore::maintain(data.clone())))
			.for_each(|_| Ok(()))
			.then(|_| finished(()))
			.boxed();

		d.spawn(interval);
	}

	/// Execute maintain procedures.
	fn maintain(data: Arc<ClusterData>) {
		trace!(target: "secretstore_net", "{}: executing maintain procedures", data.self_key_pair.public());

		ClusterCore::keep_alive(data.clone());
		ClusterCore::connect_disconnected_nodes(data.clone());
		data.sessions.stop_stalled_sessions();
	}

	/// Called for every incomming mesage.
	fn process_connection_messages(data: Arc<ClusterData>, connection: Arc<Connection>) -> IoFuture<Result<(), Error>> {
		connection
			.read_message()
			.then(move |result|
				match result {
					Ok((_, Ok(message))) => {
						ClusterCore::process_connection_message(data.clone(), connection.clone(), message);
						// continue serving connection
						data.spawn(ClusterCore::process_connection_messages(data.clone(), connection));
						finished(Ok(())).boxed()
					},
					Ok((_, Err(err))) => {
						warn!(target: "secretstore_net", "{}: protocol error {} when reading message from node {}", data.self_key_pair.public(), err, connection.node_id());
						// continue serving connection
						data.spawn(ClusterCore::process_connection_messages(data.clone(), connection));
						finished(Err(err)).boxed()
					},
					Err(err) => {
						warn!(target: "secretstore_net", "{}: network error {} when reading message from node {}", data.self_key_pair.public(), err, connection.node_id());
						// close connection
						data.connections.remove(connection.node_id(), connection.is_inbound());
						failed(err).boxed()
					},
				}
			).boxed()
	}

	/// Send keepalive messages to every othe node.
	fn keep_alive(data: Arc<ClusterData>) {
		for connection in data.connections.active_connections() {
			let last_message_diff = time::Instant::now() - connection.last_message_time();
			if last_message_diff > time::Duration::from_secs(KEEP_ALIVE_DISCONNECT_INTERVAL) {
				data.connections.remove(connection.node_id(), connection.is_inbound());
				data.sessions.on_connection_timeout(connection.node_id());
			}
			else if last_message_diff > time::Duration::from_secs(KEEP_ALIVE_SEND_INTERVAL) {
				data.spawn(connection.send_message(Message::Cluster(ClusterMessage::KeepAlive(message::KeepAlive {}))));
			}
		}
	}

	/// Try to connect to every disconnected node.
	fn connect_disconnected_nodes(data: Arc<ClusterData>) {
		for (node_id, node_address) in data.connections.disconnected_nodes() {
			if data.config.allow_connecting_to_higher_nodes || data.self_key_pair.public() < &node_id {
				ClusterCore::connect(data.clone(), node_address);
			}
		}
	}

	/// Process connection future result.
	fn process_connection_result(data: Arc<ClusterData>, is_inbound: bool, result: Result<DeadlineStatus<Result<NetConnection, Error>>, io::Error>) -> IoFuture<Result<(), Error>> {
		match result {
			Ok(DeadlineStatus::Meet(Ok(connection))) => {
				let connection = Connection::new(is_inbound, connection);
				if data.connections.insert(connection.clone()) {
					ClusterCore::process_connection_messages(data.clone(), connection)
				} else {
					finished(Ok(())).boxed()
				}
			},
			Ok(DeadlineStatus::Meet(Err(_))) => {
				finished(Ok(())).boxed()
			},
			Ok(DeadlineStatus::Timeout) => {
				finished(Ok(())).boxed()
			},
			Err(_) => {
				// network error
				finished(Ok(())).boxed()
			},
		}
	}

	/// Process single message from the connection.
	fn process_connection_message(data: Arc<ClusterData>, connection: Arc<Connection>, message: Message) {
		connection.set_last_message_time(time::Instant::now());
		trace!(target: "secretstore_net", "{}: received message {} from {}", data.self_key_pair.public(), message, connection.node_id());
		match message {
			Message::Encryption(message) => ClusterCore::process_encryption_message(data, connection, message),
			Message::Decryption(message) => ClusterCore::process_decryption_message(data, connection, message),
			Message::Cluster(message) => ClusterCore::process_cluster_message(data, connection, message),
		}
	}

	/// Process single encryption message from the connection.
	fn process_encryption_message(data: Arc<ClusterData>, connection: Arc<Connection>, mut message: EncryptionMessage) {
		let session_id = message.session_id().clone();
		let mut sender = connection.node_id().clone();
		let session = match message {
			EncryptionMessage::InitializeSession(_) => {
				let mut connected_nodes = data.connections.connected_nodes();
				connected_nodes.insert(data.self_key_pair.public().clone());

				let cluster = Arc::new(ClusterView::new(data.clone(), connected_nodes));
				data.sessions.new_encryption_session(sender.clone(), session_id.clone(), cluster)
			},
			_ => {
				data.sessions.encryption_session(&session_id)
					.ok_or(Error::InvalidSessionId)
			},
		};

		let mut is_queued_message = false;
		loop {
			match session.clone().and_then(|session| match message {
				EncryptionMessage::InitializeSession(ref message) =>
					session.on_initialize_session(sender.clone(), message),
				EncryptionMessage::ConfirmInitialization(ref message) =>
					session.on_confirm_initialization(sender.clone(), message),
				EncryptionMessage::CompleteInitialization(ref message) =>
					session.on_complete_initialization(sender.clone(), message),
				EncryptionMessage::KeysDissemination(ref message) =>
					session.on_keys_dissemination(sender.clone(), message),
				EncryptionMessage::PublicKeyShare(ref message) =>
					session.on_public_key_share(sender.clone(), message),
				EncryptionMessage::SessionError(ref message) =>
					session.on_session_error(sender.clone(), message),
				EncryptionMessage::SessionCompleted(ref message) => 
					session.on_session_completed(sender.clone(), message),
			}) {
				Ok(_) => {
					// if session is completed => stop
					let session = session.clone().expect("session.method() call finished with success; session exists; qed");
					let session_state = session.state();
					if session_state == EncryptionSessionState::Finished {
						info!(target: "secretstore_net", "{}: encryption session completed", data.self_key_pair.public());
					}
					if session_state == EncryptionSessionState::Finished || session_state == EncryptionSessionState::Failed {
						data.sessions.remove_encryption_session(&session_id);
						break;
					}

					// try to dequeue message
					match data.sessions.dequeue_encryption_message(&session_id) {
						Some((msg_sender, msg)) => {
							is_queued_message = true;
							sender = msg_sender;
							message = msg;
						},
						None => break,
					}
				},
				Err(Error::TooEarlyForRequest) => {
					data.sessions.enqueue_encryption_message(&session_id, sender, message, is_queued_message);
					break;
				},
				Err(err) => {
					warn!(target: "secretstore_net", "{}: encryption session error {} when processing message {} from node {}", data.self_key_pair.public(), err, message, sender);
					data.sessions.respond_with_encryption_error(&session_id, message::SessionError {
						session: session_id.clone().into(),
						error: format!("{:?}", err),
					});
					if err != Error::InvalidSessionId {
						data.sessions.remove_encryption_session(&session_id);
					}
					break;
				},
			}
		}
	}

	/// Process single decryption message from the connection.
	fn process_decryption_message(data: Arc<ClusterData>, connection: Arc<Connection>, mut message: DecryptionMessage) {
		let session_id = message.session_id().clone();
		let sub_session_id = message.sub_session_id().clone();
		let mut sender = connection.node_id().clone();
		let session = match message {
			DecryptionMessage::InitializeDecryptionSession(_) => {
				let mut connected_nodes = data.connections.connected_nodes();
				connected_nodes.insert(data.self_key_pair.public().clone());

				let cluster = Arc::new(ClusterView::new(data.clone(), connected_nodes));
				data.sessions.new_decryption_session(sender.clone(), session_id.clone(), sub_session_id.clone(), cluster)
			},
			_ => {
				data.sessions.decryption_session(&session_id, &sub_session_id)
					.ok_or(Error::InvalidSessionId)
			},
		};

		let mut is_queued_message = false;
		loop {
			match session.clone().and_then(|session| match message {
				DecryptionMessage::InitializeDecryptionSession(ref message) =>
					session.on_initialize_session(sender.clone(), message),
				DecryptionMessage::ConfirmDecryptionInitialization(ref message) =>
					session.on_confirm_initialization(sender.clone(), message),
				DecryptionMessage::RequestPartialDecryption(ref message) => 
					session.on_partial_decryption_requested(sender.clone(), message),
				DecryptionMessage::PartialDecryption(ref message) => 
					session.on_partial_decryption(sender.clone(), message),
				DecryptionMessage::DecryptionSessionError(ref message) => 
					session.on_session_error(sender.clone(), message),
				DecryptionMessage::DecryptionSessionCompleted(ref message) => 
					session.on_session_completed(sender.clone(), message),
			}) {
				Ok(_) => {
					// if session is completed => stop
					let session = session.clone().expect("session.method() call finished with success; session exists; qed");
					let session_state = session.state();
					if session_state == DecryptionSessionState::Finished {
						info!(target: "secretstore_net", "{}: decryption session completed", data.self_key_pair.public());
					}
					if session_state == DecryptionSessionState::Finished || session_state == DecryptionSessionState::Failed {
						data.sessions.remove_decryption_session(&session_id, &sub_session_id);
						break;
					}

					// try to dequeue message
					match data.sessions.dequeue_decryption_message(&session_id, &sub_session_id) {
						Some((msg_sender, msg)) => {
							is_queued_message = true;
							sender = msg_sender;
							message = msg;
						},
						None => break,
					}
				},
				Err(Error::TooEarlyForRequest) => {
					data.sessions.enqueue_decryption_message(&session_id, &sub_session_id, sender, message, is_queued_message);
					break;
				},
				Err(err) => {
					warn!(target: "secretstore_net", "{}: decryption session error {} when processing message {} from node {}", data.self_key_pair.public(), err, message, sender);
					data.sessions.respond_with_decryption_error(&session_id, &sub_session_id, &sender, message::DecryptionSessionError {
						session: session_id.clone().into(),
						sub_session: sub_session_id.clone().into(),
						error: format!("{:?}", err),
					});
					if err != Error::InvalidSessionId {
						data.sessions.remove_decryption_session(&session_id, &sub_session_id);
					}
					break;
				},
			}
		}
	}

	/// Process single cluster message from the connection.
	fn process_cluster_message(data: Arc<ClusterData>, connection: Arc<Connection>, message: ClusterMessage) {
		match message {
			ClusterMessage::KeepAlive(_) => data.spawn(connection.send_message(Message::Cluster(ClusterMessage::KeepAliveResponse(message::KeepAliveResponse {})))),
			ClusterMessage::KeepAliveResponse(_) => (),
			_ => warn!(target: "secretstore_net", "{}: received unexpected message {} from node {} at {}", data.self_key_pair.public(), message, connection.node_id(), connection.node_address()),
		}
	}
}

impl ClusterConnections {
	pub fn new(config: &ClusterConfiguration) -> Result<Self, Error> {
		let mut connections = ClusterConnections {
			self_node_id: config.self_key_pair.public().clone(),
			nodes: BTreeMap::new(),
			connections: RwLock::new(BTreeMap::new()),
		};

		for (node_id, &(ref node_addr, node_port)) in config.nodes.iter().filter(|&(node_id, _)| node_id != config.self_key_pair.public()) {
			let socket_address = make_socket_address(&node_addr, node_port)?;
			connections.nodes.insert(node_id.clone(), socket_address);
		}

		Ok(connections)
	}

	pub fn cluster_state(&self) -> ClusterState {
		ClusterState {
			connected: self.connections.read().keys().cloned().collect(),
		}
	}

	pub fn get(&self, node: &NodeId) -> Option<Arc<Connection>> {
		self.connections.read().get(node).cloned()
	}

	pub fn insert(&self, connection: Arc<Connection>) -> bool {
		let mut connections = self.connections.write();
		if connections.contains_key(connection.node_id()) {
			// we have already connected to the same node
			// the agreement is that node with lower id must establish connection to node with higher id
			if (&self.self_node_id < connection.node_id() && connection.is_inbound())
				|| (&self.self_node_id > connection.node_id() && !connection.is_inbound()) {
				return false;
			}
		}

		trace!(target: "secretstore_net", "{}: inserting connection to {} at {}", self.self_node_id, connection.node_id(), connection.node_address());
		connections.insert(connection.node_id().clone(), connection);
		true
	}

	pub fn remove(&self, node: &NodeId, is_inbound: bool) {
		let mut connections = self.connections.write();
		if let Entry::Occupied(entry) = connections.entry(node.clone()) {
			if entry.get().is_inbound() != is_inbound {
				return;
			}

			trace!(target: "secretstore_net", "{}: removing connection to {} at {}", self.self_node_id, entry.get().node_id(), entry.get().node_address());
			entry.remove_entry();
		}
	}

	pub fn connected_nodes(&self) -> BTreeSet<NodeId> {
		self.connections.read().keys().cloned().collect()
	}

	pub fn active_connections(&self)-> Vec<Arc<Connection>> {
		self.connections.read().values().cloned().collect()
	}

	pub fn disconnected_nodes(&self) -> BTreeMap<NodeId, SocketAddr> {
		let connections = self.connections.read();
		self.nodes.iter()
			.filter(|&(node_id, _)| !connections.contains_key(node_id))
			.map(|(node_id, node_address)| (node_id.clone(), node_address.clone()))
			.collect()
	}
}

impl ClusterSessions {
	pub fn new(config: &ClusterConfiguration) -> Self {
		ClusterSessions {
			self_node_id: config.self_key_pair.public().clone(),
			nodes: config.nodes.keys().cloned().collect(),
			acl_storage: config.acl_storage.clone(),
			key_storage: config.key_storage.clone(),
			encryption_sessions: RwLock::new(BTreeMap::new()),
			decryption_sessions: RwLock::new(BTreeMap::new()),
			make_faulty_encryption_sessions: AtomicBool::new(false),
		}
	}

	pub fn new_encryption_session(&self, master: NodeId, session_id: SessionId, cluster: Arc<ClusterView>) -> Result<Arc<EncryptionSessionImpl>, Error> {
		let mut encryption_sessions = self.encryption_sessions.write();
		// check that there's no active encryption session with the same id
		if encryption_sessions.contains_key(&session_id) {
			return Err(Error::DuplicateSessionId);
		}
		// check that there's no finished encryption session with the same id
		if self.key_storage.contains(&session_id) {
			return Err(Error::DuplicateSessionId);
		}

		// communicating to all other nodes is crucial for encryption session
		// => check that we have connections to all cluster nodes
		if self.nodes.iter().any(|n| !cluster.is_connected(n)) {
			return Err(Error::NodeDisconnected);
		}

		let session = Arc::new(EncryptionSessionImpl::new(EncryptionSessionParams {
			id: session_id.clone(),
			self_node_id: self.self_node_id.clone(),
			key_storage: self.key_storage.clone(),
			cluster: cluster.clone(),
		}));
		let encryption_session = QueuedEncryptionSession {
			master: master,
			cluster_view: cluster,
			last_message_time: time::Instant::now(),
			session: session.clone(),
			queue: VecDeque::new()
		};
		if self.make_faulty_encryption_sessions.load(Ordering::Relaxed) {
			encryption_session.session.simulate_faulty_behaviour();
		}
		encryption_sessions.insert(session_id, encryption_session);
		Ok(session)
	}

	pub fn remove_encryption_session(&self, session_id: &SessionId) {
		self.encryption_sessions.write().remove(session_id);
	}

	pub fn encryption_session(&self, session_id: &SessionId) -> Option<Arc<EncryptionSessionImpl>> {
		self.encryption_sessions.read().get(session_id).map(|s| s.session.clone())
	}

	pub fn enqueue_encryption_message(&self, session_id: &SessionId, sender: NodeId, message: EncryptionMessage, is_queued_message: bool) {
		self.encryption_sessions.write().get_mut(session_id)
			.map(|session| if is_queued_message { session.queue.push_front((sender, message)) }
				else { session.queue.push_back((sender, message)) });
	}

	pub fn dequeue_encryption_message(&self, session_id: &SessionId) -> Option<(NodeId, EncryptionMessage)> {
		self.encryption_sessions.write().get_mut(session_id)
			.and_then(|session| session.queue.pop_front())
	}

	pub fn respond_with_encryption_error(&self, session_id: &SessionId, error: message::SessionError) {
		self.encryption_sessions.read().get(session_id)
			.map(|s| {
				// error in encryption session is considered fatal
				// => broadcast error

				// do not bother processing send error, as we already processing error
				let _ = s.cluster_view.broadcast(Message::Encryption(EncryptionMessage::SessionError(error)));
			});
	}

	#[cfg(test)]
	pub fn make_faulty_encryption_sessions(&self) {
		self.make_faulty_encryption_sessions.store(true, Ordering::Relaxed);
	}

	pub fn new_decryption_session(&self, master: NodeId, session_id: SessionId, sub_session_id: Secret, cluster: Arc<ClusterView>) -> Result<Arc<DecryptionSessionImpl>, Error> {
		let mut decryption_sessions = self.decryption_sessions.write();
		let session_id = DecryptionSessionId::new(session_id, sub_session_id);
		if decryption_sessions.contains_key(&session_id) {
			return Err(Error::DuplicateSessionId);
		}

		// some of nodes, which were encrypting secret may be down
		// => do not use these in decryption session
		let mut encrypted_data = self.key_storage.get(&session_id.id).map_err(|e| Error::KeyStorage(e.into()))?;
		let disconnected_nodes: BTreeSet<_> = encrypted_data.id_numbers.keys().cloned().collect();
		let disconnected_nodes: BTreeSet<_> = disconnected_nodes.difference(&cluster.nodes()).cloned().collect();
		for disconnected_node in disconnected_nodes {
			encrypted_data.id_numbers.remove(&disconnected_node);
		}

		let session = Arc::new(DecryptionSessionImpl::new(DecryptionSessionParams {
			id: session_id.id.clone(),
			access_key: session_id.access_key.clone(),
			self_node_id: self.self_node_id.clone(),
			encrypted_data: encrypted_data,
			acl_storage: self.acl_storage.clone(),
			cluster: cluster.clone(),
		})?);
		let decryption_session = QueuedDecryptionSession {
			master: master,
			cluster_view: cluster,
			last_message_time: time::Instant::now(),
			session: session.clone(),
			queue: VecDeque::new()
		};
		decryption_sessions.insert(session_id, decryption_session);
		Ok(session)
	}

	pub fn remove_decryption_session(&self, session_id: &SessionId, sub_session_id: &Secret) {
		let session_id = DecryptionSessionId::new(session_id.clone(), sub_session_id.clone());
		self.decryption_sessions.write().remove(&session_id);
	}

	pub fn decryption_session(&self, session_id: &SessionId, sub_session_id: &Secret) -> Option<Arc<DecryptionSessionImpl>> {
		let session_id = DecryptionSessionId::new(session_id.clone(), sub_session_id.clone());
		self.decryption_sessions.read().get(&session_id).map(|s| s.session.clone())
	}

	pub fn enqueue_decryption_message(&self, session_id: &SessionId, sub_session_id: &Secret, sender: NodeId, message: DecryptionMessage, is_queued_message: bool) {
		let session_id = DecryptionSessionId::new(session_id.clone(), sub_session_id.clone());
		self.decryption_sessions.write().get_mut(&session_id)
			.map(|session| if is_queued_message { session.queue.push_front((sender, message)) }
				else { session.queue.push_back((sender, message)) });
	}

	pub fn dequeue_decryption_message(&self, session_id: &SessionId, sub_session_id: &Secret) -> Option<(NodeId, DecryptionMessage)> {
		let session_id = DecryptionSessionId::new(session_id.clone(), sub_session_id.clone());
		self.decryption_sessions.write().get_mut(&session_id)
			.and_then(|session| session.queue.pop_front())
	}

	pub fn respond_with_decryption_error(&self, session_id: &SessionId, sub_session_id: &Secret, to: &NodeId, error: message::DecryptionSessionError) {
		let session_id = DecryptionSessionId::new(session_id.clone(), sub_session_id.clone());
		self.decryption_sessions.read().get(&session_id)
			.map(|s| {
				// error in decryption session is non-fatal, if occurs on slave node
				// => either respond with error
				// => or broadcast error

				// do not bother processing send error, as we already processing error
				if &s.master == s.session.node() {
					let _ = s.cluster_view.broadcast(Message::Decryption(DecryptionMessage::DecryptionSessionError(error)));
				} else {
					let _ = s.cluster_view.send(to, Message::Decryption(DecryptionMessage::DecryptionSessionError(error)));
				}
			});
	}

	fn stop_stalled_sessions(&self) {
		{
			let sessions = self.encryption_sessions.write();
			for sid in sessions.keys().collect::<Vec<_>>() {
				let session = sessions.get(&sid).expect("enumerating only existing sessions; qed");
				if time::Instant::now() - session.last_message_time > time::Duration::from_secs(ENCRYPTION_SESSION_TIMEOUT_INTERVAL) {
					session.session.on_session_timeout();
					if session.session.state() == EncryptionSessionState::Finished
						|| session.session.state() == EncryptionSessionState::Failed {
						self.remove_encryption_session(&sid);
					}
				}
			}
		}
		{
			let sessions = self.decryption_sessions.write();
			for sid in sessions.keys().collect::<Vec<_>>() {
				let session = sessions.get(&sid).expect("enumerating only existing sessions; qed");
				if time::Instant::now() - session.last_message_time > time::Duration::from_secs(DECRYPTION_SESSION_TIMEOUT_INTERVAL) {
					session.session.on_session_timeout();
					if session.session.state() == DecryptionSessionState::Finished
						|| session.session.state() == DecryptionSessionState::Failed {
						self.remove_decryption_session(&sid.id, &sid.access_key);
					}
				}
			}
		}
	}

	pub fn on_connection_timeout(&self, node_id: &NodeId) {
		for (sid, session) in self.encryption_sessions.read().iter() {
			session.session.on_node_timeout(node_id);
			if session.session.state() == EncryptionSessionState::Finished
				|| session.session.state() == EncryptionSessionState::Failed {
				self.remove_encryption_session(sid);
			}
		}
		for (sid, session) in self.decryption_sessions.read().iter() {
			session.session.on_node_timeout(node_id);
			if session.session.state() == DecryptionSessionState::Finished
				|| session.session.state() == DecryptionSessionState::Failed {
				self.remove_decryption_session(&sid.id, &sid.access_key);
			}
		}
	}
}

impl ClusterData {
	pub fn new(handle: &Handle, config: ClusterConfiguration, connections: ClusterConnections, sessions: ClusterSessions) -> Arc<Self> {
		Arc::new(ClusterData {
			handle: handle.remote().clone(),
			pool: CpuPool::new(config.threads),
			self_key_pair: config.self_key_pair.clone(),
			connections: connections,
			sessions: sessions,
			config: config,
		})
	}

	/// Get connection to given node.
	pub fn connection(&self, node: &NodeId) -> Option<Arc<Connection>> {
		self.connections.get(node)
	}

	/// Spawns a future using thread pool and schedules execution of it with event loop handle.
	pub fn spawn<F>(&self, f: F) where F: Future + Send + 'static, F::Item: Send + 'static, F::Error: Send + 'static {
		let pool_work = self.pool.spawn(f);
		self.handle.spawn(move |_handle| {
			pool_work.then(|_| finished(()))
		})
	}
}

impl Connection {
	pub fn new(is_inbound: bool, connection: NetConnection) -> Arc<Connection> {
		Arc::new(Connection {
			node_id: connection.node_id,
			node_address: connection.address,
			is_inbound: is_inbound,
			stream: connection.stream,
			key: connection.key,
			last_message_time: Mutex::new(time::Instant::now()),
		})
	}

	pub fn is_inbound(&self) -> bool {
		self.is_inbound
	}

	pub fn node_id(&self) -> &NodeId {
		&self.node_id
	}

	pub fn last_message_time(&self) -> time::Instant {
		*self.last_message_time.lock()
	}

	pub fn set_last_message_time(&self, last_message_time: time::Instant) {
		*self.last_message_time.lock() = last_message_time;
	}

	pub fn node_address(&self) -> &SocketAddr {
		&self.node_address
	}

	pub fn send_message(&self, message: Message) -> WriteMessage<SharedTcpStream> {
		write_encrypted_message(self.stream.clone(), &self.key, message)
	}

	pub fn read_message(&self) -> ReadMessage<SharedTcpStream> {
		read_encrypted_message(self.stream.clone(), self.key.clone())
	}
}

impl ClusterView {
	pub fn new(cluster: Arc<ClusterData>, nodes: BTreeSet<NodeId>) -> Self {
		ClusterView {
			core: Arc::new(Mutex::new(ClusterViewCore {
				cluster: cluster,
				nodes: nodes,
			})),
		}
	}

	pub fn is_connected(&self, node: &NodeId) -> bool {
		self.core.lock().nodes.contains(node)
	}

	pub fn nodes(&self) -> BTreeSet<NodeId> {
		self.core.lock().nodes.clone()
	}
}

impl Cluster for ClusterView {
	fn broadcast(&self, message: Message) -> Result<(), Error> {
		let core = self.core.lock();
		for node in core.nodes.iter().filter(|n| *n != core.cluster.self_key_pair.public()) {
			trace!(target: "secretstore_net", "{}: sent message {} to {}", core.cluster.self_key_pair.public(), message, node);
			let connection = core.cluster.connection(node).ok_or(Error::NodeDisconnected)?;
			core.cluster.spawn(connection.send_message(message.clone()))
		}
		Ok(())
	}

	fn send(&self, to: &NodeId, message: Message) -> Result<(), Error> {
		let core = self.core.lock();
		trace!(target: "secretstore_net", "{}: sent message {} to {}", core.cluster.self_key_pair.public(), message, to);
		let connection = core.cluster.connection(to).ok_or(Error::NodeDisconnected)?;
		core.cluster.spawn(connection.send_message(message));
		Ok(())
	}
}

impl ClusterClientImpl {
	pub fn new(data: Arc<ClusterData>) -> Self {
		ClusterClientImpl {
			data: data,
		}
	}
}

impl ClusterClient for ClusterClientImpl {
	fn cluster_state(&self) -> ClusterState {
		self.data.connections.cluster_state()
	}

	fn new_encryption_session(&self, session_id: SessionId, threshold: usize) -> Result<Arc<EncryptionSession>, Error> {
		let mut connected_nodes = self.data.connections.connected_nodes();
		connected_nodes.insert(self.data.self_key_pair.public().clone());

		let cluster = Arc::new(ClusterView::new(self.data.clone(), connected_nodes.clone()));
		let session = self.data.sessions.new_encryption_session(self.data.self_key_pair.public().clone(), session_id.clone(), cluster)?;
		session.initialize(threshold, connected_nodes)?;
		Ok(EncryptionSessionWrapper::new(Arc::downgrade(&self.data), session_id, session))
	}

	fn new_decryption_session(&self, session_id: SessionId, requestor_signature: Signature, is_shadow_decryption: bool) -> Result<Arc<DecryptionSession>, Error> {
		let mut connected_nodes = self.data.connections.connected_nodes();
		connected_nodes.insert(self.data.self_key_pair.public().clone());

		let access_key = Random.generate()?.secret().clone();
		let cluster = Arc::new(ClusterView::new(self.data.clone(), connected_nodes.clone()));
		let session = self.data.sessions.new_decryption_session(self.data.self_key_pair.public().clone(), session_id, access_key.clone(), cluster)?;
		session.initialize(requestor_signature, is_shadow_decryption)?;
		Ok(DecryptionSessionWrapper::new(Arc::downgrade(&self.data), session_id, access_key, session))
	}

	#[cfg(test)]
	fn connect(&self) {
		ClusterCore::connect_disconnected_nodes(self.data.clone());
	}

	#[cfg(test)]
	fn make_faulty_encryption_sessions(&self) {
		self.data.sessions.make_faulty_encryption_sessions();
	}

	#[cfg(test)]
	fn encryption_session(&self, session_id: &SessionId) -> Option<Arc<EncryptionSessionImpl>> {
		self.data.sessions.encryption_session(session_id)
	}
}

impl EncryptionSessionWrapper {
	pub fn new(cluster: Weak<ClusterData>, session_id: SessionId, session: Arc<EncryptionSession>) -> Arc<Self> {
		Arc::new(EncryptionSessionWrapper {
			session: session,
			session_id: session_id,
			cluster: cluster,
		})
	}
}

impl EncryptionSession for EncryptionSessionWrapper {
	fn state(&self) -> EncryptionSessionState {
		self.session.state()
	}

	fn wait(&self, timeout: Option<time::Duration>) -> Result<Public, Error> {
		self.session.wait(timeout)
	}

	#[cfg(test)]
	fn joint_public_key(&self) -> Option<Result<Public, Error>> {
		self.session.joint_public_key()
	}
}

impl Drop for EncryptionSessionWrapper {
	fn drop(&mut self) {
		if let Some(cluster) = self.cluster.upgrade() {
			cluster.sessions.remove_encryption_session(&self.session_id);
		}
	}
}

impl DecryptionSessionWrapper {
	pub fn new(cluster: Weak<ClusterData>, session_id: SessionId, access_key: Secret, session: Arc<DecryptionSession>) -> Arc<Self> {
		Arc::new(DecryptionSessionWrapper {
			session: session,
			session_id: session_id,
			access_key: access_key,
			cluster: cluster,
		})
	}
}

impl DecryptionSession for DecryptionSessionWrapper {
	fn wait(&self) -> Result<DocumentEncryptedKeyShadow, Error> {
		self.session.wait()
	}
}

impl Drop for DecryptionSessionWrapper {
	fn drop(&mut self) {
		if let Some(cluster) = self.cluster.upgrade() {
			cluster.sessions.remove_decryption_session(&self.session_id, &self.access_key);
		}
	}
}

fn make_socket_address(address: &str, port: u16) -> Result<SocketAddr, Error> {
	let ip_address: IpAddr = address.parse().map_err(|_| Error::InvalidNodeAddress)?;
	Ok(SocketAddr::new(ip_address, port))
}

#[cfg(test)]
pub mod tests {
	use std::sync::Arc;
	use std::time;
	use std::collections::VecDeque;
	use parking_lot::Mutex;
	use tokio_core::reactor::Core;
	use ethkey::{Random, Generator};
	use key_server_cluster::{NodeId, SessionId, Error, DummyAclStorage, DummyKeyStorage};
	use key_server_cluster::message::Message;
	use key_server_cluster::cluster::{Cluster, ClusterCore, ClusterConfiguration};
	use key_server_cluster::encryption_session::{Session as EncryptionSession, SessionState as EncryptionSessionState};

	#[derive(Debug)]
	pub struct DummyCluster {
		id: NodeId,
		data: Mutex<DummyClusterData>,
	}

	#[derive(Debug, Default)]
	struct DummyClusterData {
		nodes: Vec<NodeId>,
		messages: VecDeque<(NodeId, Message)>,
	}

	impl DummyCluster {
		pub fn new(id: NodeId) -> Self {
			DummyCluster {
				id: id,
				data: Mutex::new(DummyClusterData::default())
			}
		}

		pub fn node(&self) -> NodeId {
			self.id.clone()
		}

		pub fn add_node(&self, node: NodeId) {
			self.data.lock().nodes.push(node);
		}

		pub fn take_message(&self) -> Option<(NodeId, Message)> {
			self.data.lock().messages.pop_front()
		}
	}

	impl Cluster for DummyCluster {
		fn broadcast(&self, message: Message) -> Result<(), Error> {
			let mut data = self.data.lock();
			let all_nodes: Vec<_> = data.nodes.iter().cloned().filter(|n| n != &self.id).collect();
			for node in all_nodes {
				data.messages.push_back((node, message.clone()));
			}
			Ok(())
		}

		fn send(&self, to: &NodeId, message: Message) -> Result<(), Error> {
			debug_assert!(&self.id != to);
			self.data.lock().messages.push_back((to.clone(), message));
			Ok(())
		}
	}

	pub fn loop_until<F>(core: &mut Core, timeout: time::Duration, predicate: F) where F: Fn() -> bool {
		let start = time::Instant::now();
		loop {
			core.turn(Some(time::Duration::from_millis(1)));
			if predicate() {
				break;
			}

			if time::Instant::now() - start > timeout {
				panic!("no result in {:?}", timeout);
			}
		}
	}

	pub fn all_connections_established(cluster: &Arc<ClusterCore>) -> bool {
		cluster.config().nodes.keys()
			.filter(|p| *p != cluster.config().self_key_pair.public())
			.all(|p| cluster.connection(p).is_some())
	}

	pub fn make_clusters(core: &Core, ports_begin: u16, num_nodes: usize) -> Vec<Arc<ClusterCore>> {
		let key_pairs: Vec<_> = (0..num_nodes).map(|_| Random.generate().unwrap()).collect();
		let cluster_params: Vec<_> = (0..num_nodes).map(|i| ClusterConfiguration {
			threads: 1,
			self_key_pair: key_pairs[i].clone(),
			listen_address: ("127.0.0.1".to_owned(), ports_begin + i as u16),
			nodes: key_pairs.iter().enumerate()
				.map(|(j, kp)| (kp.public().clone(), ("127.0.0.1".into(), ports_begin + j as u16)))
				.collect(),
			allow_connecting_to_higher_nodes: false,
			key_storage: Arc::new(DummyKeyStorage::default()),
			acl_storage: Arc::new(DummyAclStorage::default()),
		}).collect();
		let clusters: Vec<_> = cluster_params.into_iter().enumerate()
			.map(|(_, params)| ClusterCore::new(core.handle(), params).unwrap())
			.collect();

		clusters
	}

	pub fn run_clusters(clusters: &[Arc<ClusterCore>]) {
		for cluster in clusters {
			cluster.run_listener().unwrap();
		}
		for cluster in clusters {
			cluster.run_connections().unwrap();
		}
	}

	#[test]
	fn cluster_connects_to_other_nodes() {
		let mut core = Core::new().unwrap();
		let clusters = make_clusters(&core, 6010, 3);
		run_clusters(&clusters);
		loop_until(&mut core, time::Duration::from_millis(300), || clusters.iter().all(all_connections_established));
	}

	#[test]
	fn cluster_wont_start_encryption_session_if_not_fully_connected() {
		let core = Core::new().unwrap();
		let clusters = make_clusters(&core, 6013, 3);
		clusters[0].run().unwrap();
		match clusters[0].client().new_encryption_session(SessionId::default(), 1) {
			Err(Error::NodeDisconnected) => (),
			Err(e) => panic!("unexpected error {:?}", e),
			_ => panic!("unexpected success"),
		}
	}

	#[test]
	fn error_in_encryption_session_broadcasted_to_all_other_nodes() {
		let mut core = Core::new().unwrap();
		let clusters = make_clusters(&core, 6016, 3);
		run_clusters(&clusters);
		loop_until(&mut core, time::Duration::from_millis(300), || clusters.iter().all(all_connections_established));

		// ask one of nodes to produce faulty encryption sessions
		clusters[1].client().make_faulty_encryption_sessions();

		// start && wait for encryption session to fail
		let session = clusters[0].client().new_encryption_session(SessionId::default(), 1).unwrap();
		loop_until(&mut core, time::Duration::from_millis(300), || session.joint_public_key().is_some());
		assert!(session.joint_public_key().unwrap().is_err());

		// check that faulty session is either removed from all nodes, or nonexistent (already removed)
		assert!(clusters[0].client().encryption_session(&SessionId::default()).is_none());
		for i in 1..3 {
			if let Some(session) = clusters[i].client().encryption_session(&SessionId::default()) {
				loop_until(&mut core, time::Duration::from_millis(300), || session.joint_public_key().is_some());
				assert!(session.joint_public_key().unwrap().is_err());
				assert!(clusters[i].client().encryption_session(&SessionId::default()).is_none());
			}
		}
	}

	#[test]
	fn encryption_session_is_removed_when_succeeded() {
		let mut core = Core::new().unwrap();
		let clusters = make_clusters(&core, 6019, 3);
		run_clusters(&clusters);
		loop_until(&mut core, time::Duration::from_millis(300), || clusters.iter().all(all_connections_established));

		// start && wait for encryption session to complete
		let session = clusters[0].client().new_encryption_session(SessionId::default(), 1).unwrap();
		loop_until(&mut core, time::Duration::from_millis(300), || session.state() == EncryptionSessionState::Finished);
		assert!(session.joint_public_key().unwrap().is_ok());

		// check that session is either removed from all nodes, or nonexistent (already removed)
		assert!(clusters[0].client().encryption_session(&SessionId::default()).is_none());
		for i in 1..3 {
			if let Some(session) = clusters[i].client().encryption_session(&SessionId::default()) {
				loop_until(&mut core, time::Duration::from_millis(300), || session.state() == EncryptionSessionState::Finished);
				assert!(session.joint_public_key().unwrap().is_err());
				assert!(clusters[i].client().encryption_session(&SessionId::default()).is_none());
			}
		}
	}
}
