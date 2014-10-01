use std::sync::{Arc, RWLock};

use std::collections::{HashSet};
use util::{HostMask};

use msg::{RawMessage};
use cmd;

pub mod flag {
    #[deriving(FromPrimitive, PartialEq)]
    pub enum RegistrationStatus {
        Connected = 0,
        GotNick = 1,
        GotUser = 2,
        Registered = 3,
    }
    #[deriving(FromPrimitive, PartialEq, Eq, Hash)]
    pub enum Extensions {
        Extensions,
        SASL
    }
}

/// Struct to hold the user info synchronized across all threads.
pub type SharedInfo = Arc<RWLock<UserInfo>>;

/// Struct holding the user info like nick name, host name etc.
pub struct UserInfo {
    id: super::PeerId,
    server_name: String,
    nick: String,
    username: String,
    realname: String,
    hostname: String,
    hostmask: HostMask,
    status: flag::RegistrationStatus,
    capabilities: HashSet<flag::Extensions>
}

impl UserInfo {
    /// Creates the user info struct.
    pub fn new(id: super::PeerId, server_name: String, hostname: String) -> UserInfo {
        let mask = HostMask::from_parts("*", "*", hostname.as_slice());
        UserInfo {
            id: id,
            server_name: server_name,
            nick: "*".to_string(),
            username: "".to_string(),
            realname: "John Doe".to_string(),
            hostname: hostname,
            hostmask: mask,
            status: flag::Connected,
            capabilities: HashSet::new()
        }
    }
    
    /// Getter for the peer id
    pub fn id(&self) -> super::PeerId {
        self.id.clone()
    }
    
    /// Getter for the nick name
    pub fn nick(&self) -> &String {
        &self.nick
    }
    /// Setter for the nick name
    pub fn set_nick(&mut self, nick: String) {
        self.nick = nick;
        self.update_mask()
    }
    /// Getter for the nick name
    pub fn username(&self) -> &String {
        &self.username
    }
    /// Getter for the user name
    pub fn set_username(&mut self, name: String) {
        self.username = name;
        self.update_mask()
    }
    /// Getter for the nick name
    pub fn realname(&self) -> &String {
        &self.realname
    }
    /// Getter for the real name
    pub fn set_realname(&mut self, name: String) {
        self.realname = name;
        self.update_mask()
    }
    /// Getter for the server name
    pub fn server_name(&self) -> &String {
        &self.server_name
    }
    /// Getter for the server name
    pub fn hostname(&self) -> &String {
        &self.hostname
    }
    /// Getter for the registration status/method
    pub fn registration_status(&self) -> flag::RegistrationStatus {
        self.status
    }
    /// Getter for the registration status/method
    pub fn mut_registration_status(&mut self) -> &mut flag::RegistrationStatus {
        &mut self.status
    }
    
    /// Updates the real hostmask
    fn update_mask(&mut self) {
        self.hostmask = HostMask::from_parts(
            self.nick.as_slice(),
            self.username.as_slice(),
            self.hostname.as_slice()
        )
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
