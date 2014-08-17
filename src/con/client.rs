use std::sync::{Arc, RWLock};

use util::{HostMask};

use msg::{RawMessage};
use cmd;

/// Struct to hold the user info synchronized across all threads.
pub type SharedInfo = Arc<RWLock<UserInfo>>;

/// Struct holding the user info like nick name, host name etc.
pub struct UserInfo {
    id: super::PeerId,
    nick: String,
    server_name: String,
    hostmask: HostMask
}

impl UserInfo {
    /// Creates the user info struct.
    pub fn new(id: super::PeerId, server_name: String, hostmask: HostMask) -> UserInfo {
        UserInfo {
            id: id,
            nick: "*".to_string(),
            server_name: server_name,
            hostmask: hostmask
        }
    }
    
    /// Getter for the peer id
    pub fn id(&self) -> super::PeerId {
        self.id.clone()
    }
    /// Getter for the peer id
    pub fn nick(&self) -> &String {
        &self.nick
    }
    /// Getter for the peer id
    pub fn server_name(&self) -> &String {
        &self.server_name
    }
    /// Getter for the public host mask.
    ///
    /// This is the host mask that is send out to other users.
    pub fn public_hostmask(&self) -> &HostMask {
        &self.hostmask
    }
    /// Getter for the real host mask
    pub fn real_hostmask(&self) -> &HostMask {
        &self.hostmask
    }
    
}

/// Struct representing a peer.
///
/// Allows access to the globally shared user data can easily be cloned
#[deriving(Clone)]
pub struct Peer {
    info: SharedInfo,
    tx: Sender<RawMessage>,
}
impl Peer {
    /// Create a new peer struct.
    pub fn new(info: UserInfo, tx: Sender<RawMessage>) -> Peer {
        Peer {
            info: Arc::new(RWLock::new(info)),
            tx: tx
        }
    }

    /// Sends a message to the peer.
    pub fn send_msg(&self, msg: RawMessage) {
        let _ = self.tx.send_opt(msg);
    }
    
    /// Sends a response to the peer. 
    ///
    /// This should be the preferred way of sending responses. Do
    /// not construct raw responsed. This method prepends the params
    /// with the nick name to create well-formed responses.
    pub fn send_response<'a>(&'a self, command: cmd::ResponseCode, 
                         params: &[&'a str], origin: &str) {
        let info = self.info.read();
        let msg = RawMessage::new(
            cmd::REPLY(command), 
            (vec![info.nick().as_slice()].append(params)).as_slice(), 
            Some(origin)
        );
        let _ = self.tx.send_opt(msg);
    }
    
    /// Getter for the shared user info
    pub fn info(&self) -> &SharedInfo {
        &self.info
    }
    
    /// Getter for the peer id
    pub fn id(&self) -> super::PeerId {
        self.info().read().id()
    }
}
