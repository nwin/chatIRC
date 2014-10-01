use std::collections::{HashSet};

use con::{PeerId, Peer};
use msg::{RawMessage};
use util::{HostMask};
use cmd;

use super::util::{Flags, ChannelMode, OperatorPrivilege, VoicePrivilege};

/// Represents a channel member
pub struct Member {
    id: PeerId,
    peer: Peer,
    nick: String,
    mask: HostMask,
    hostname: String,
    username: String,
    realname: String,
    decorated_nick: String,
    flags: Flags,
    server_name: String,
}





impl Member {
    /// Creates a new member
    pub fn new(peer: Peer) -> Member {
        Member {
            id: peer.id(),
            nick: peer.info().read().nick().clone(),
            hostname: peer.info().read().hostname().clone(),
            username: peer.info().read().username().clone(),
            realname: peer.info().read().realname().clone(),
            mask: peer.info().read().real_hostmask().clone(),
            decorated_nick: peer.info().read().nick().clone(),
            flags: HashSet::new(),
            server_name: peer.info().read().server_name().clone(),
            peer: peer,
        }
    }
    
    pub fn send_response(&self, command: cmd::ResponseCode, params: &[&str]) {
        self.peer.send_response(command, params, self.server_name.as_slice())
    }
    
    /// Sends a message to the client
    pub fn send_msg(&self, msg: RawMessage) {
        self.peer.send_msg(msg)
    }
    
    /// Grant a privilege to a member
    pub fn promote(&mut self, flag: ChannelMode) {
        self.flags.insert(flag);
        self.update_decorated_nick();
    }
    
    /// Take a privilege from a member
    pub fn demote(&mut self, flag: ChannelMode) {
        self.flags.remove(&flag);
        self.update_decorated_nick();
    }
    
    /// User channel flags as a string
    pub fn flags(&self) -> String {
        self.flags.iter().map( |c| *c as u8 as char).collect() 
    }
    
    /// Checks whether a member has a certain privilege
    fn has_privilege(&self, privilege: ChannelMode) -> bool {
        self.flags.contains(&privilege)
    }
    
    /// Get flag as string
    pub fn decoration(&self) -> String {
        if self.has_privilege(OperatorPrivilege) {
            "@".to_string()
        } else if self.has_privilege(VoicePrivilege) {
            "+".to_string()
        } else {
            "".to_string()
        }
    }
    
    /// Checks whether a member is the operator of the channel
    pub fn is_op(&self) -> bool {
        self.has_privilege(OperatorPrivilege) 
    }
    
    /// Checks whether a member has the voice privilege
    pub fn has_voice(&self) -> bool {
        self.has_privilege(VoicePrivilege) 
        || self.has_privilege(OperatorPrivilege) 
    }
    
    /// Checks if any of members host mask matches any in the given set
    pub fn mask_matches_any(&self, masks: &HashSet<HostMask>) -> bool {
        for mask in masks.iter() {
            if mask.matches(self.mask.as_str()) {
                return true
            }
        }
        false
    }
    
    /// Updates the cached decorated nick
    fn update_decorated_nick(&mut self) {
        self.decorated_nick = self.decoration() + self.nick()
    }
    
    /// Returns the nickname, prefixed with:
    /// @ for op
    /// v for voice
    pub fn decorated_nick(&self) -> &str {
        return self.decorated_nick.as_slice()
    }
    
    /// Getter for nick
    pub fn nick(&self) -> &str {
        return self.nick.as_slice()
    }
    
    /// Getter for host
    pub fn hostname(&self) -> &str {
        return self.hostname.as_slice()
    }
    
    /// Getter for username
    pub fn username(&self) -> &str {
        return self.username.as_slice()
    }
    
    /// Getter for realname
    pub fn realname(&self) -> &str {
        return self.realname.as_slice()
    }
    
    /// Setter for nick
    pub fn set_nick(&mut self, nick: String) {
        self.nick = nick;
        self.update_decorated_nick()
    }
    
    /// Getter for client id
    pub fn id(&self) -> PeerId {
        self.id.clone()
    }
    
    /// Getter for the client proxy
    pub fn proxy(&self) -> &Peer {
        &self.peer
    }
}

impl PartialEq for Member {
    #[inline]
    fn eq(&self, other: &Member) -> bool {
        self.id == other.id
    }
}

impl Eq for Member {}
