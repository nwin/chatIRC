use std::collections::{HashSet};

use client::{ClientId, ClientProxy};
use msg::{RawMessage};
use msg::util::{HostMask};
use cmd;

use super::util::{Flags, ChannelMode, OperatorPrivilege, VoicePrivilege};

/// Represents a channel member
pub struct Member {
    id: ClientId,
    proxy: ClientProxy,
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
    pub fn new(id: ClientId, realname: String, mask: HostMask, server_name: String, proxy: ClientProxy) -> Member {
        Member {
            id: id,
            proxy: proxy,
            nick: mask.nick().unwrap().to_string(),
            hostname: mask.host().unwrap().to_string(),
            username: mask.user().unwrap().to_string(),
            realname: realname,
            mask: mask.clone(),
            decorated_nick: mask.nick().unwrap().to_string(),
            flags: HashSet::new(),
            server_name: server_name
        }
    }
    
    pub fn send_response(&self, command: cmd::ResponseCode, params: &[&str]) {
        self.proxy.send_response(command, params, self.server_name.as_slice())
    }
    
    /// Sends a message to the client
    pub fn send_msg(&self, msg: RawMessage) {
        self.proxy.send_msg(msg)
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
    
    /// Updates the cached decorated nick
    fn update_decorated_nick(&mut self) {
        self.decorated_nick = self.decoration().append(self.nick())
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
    pub fn id(&self) -> ClientId {
        self.id.clone()
    }
    
    /// Getter for the client proxy
    pub fn proxy(&self) -> &ClientProxy {
        &self.proxy
    }
}

impl PartialEq for Member {
    #[inline]
    fn eq(&self, other: &Member) -> bool {
        self.id == other.id
    }
}

impl Eq for Member {}
