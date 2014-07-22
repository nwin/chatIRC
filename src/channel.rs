use std::collections::{HashMap, EnumSet};
use std::collections::enum_set::{CLike};

use client::{SharedClient, WeaklySharedClient};
use message::{Message};

use cmd::*;

/// Enumeration of possible channel modes
/// as of http://tools.ietf.org/html/rfc2811#section-4
#[deriving(FromPrimitive, Show)]
pub enum ChannelMode {
    /// give "channel creator" status
    GiveOp = b'O' as int,
    /// give/take channel operator privilege
    OperatorPrivilege = b'o' as int,
    /// give/take the voice privilege
    VoicePrivilege = b'v' as int,
    /// toggle the anonymous channel flag
    AnonFlag = b'a' as int,
    /// toggle the invite-only channel flag
    InviteOnly = b'i' as int,
    /// toggle the moderated channel
    Moderated = b'm' as int,
    /// toggle the no messages to channel from clients on the
    /// outside
    MemberOnly = b'n' as int,
    /// toggle the quiet channel flag
    Quiet = b'q' as int,
    /// toggle the private channel flag
    Private = b'p' as int,
    /// toggle the secret channel flag
    Secret = b's' as int,
    /// toggle the server reop channel flag
    ReOpFlag = b'r' as int,
    /// toggle the topic settable by channel operator only flag
    TopicProtect = b't' as int,
    /// set/remove the channel key (password)
    ChannelKey = b'k' as int,
    /// set/remove the user limit to channel
    UserLimit = b'l' as int,
    /// set/remove ban mask to keep users out
    BanMask = b'b' as int,
    /// set/remove an exception mask to override a ban mask
    ExceptionMask = b'e' as int,
    /// set/remove an invitation mask to automatically override
    /// the invite-only flag
    InvitationMask = b'I' as int
}

impl CLike for ChannelMode {
    #[inline]
    fn to_uint(&self) -> uint {
        *self as uint
    }

    #[inline]
    fn from_uint(val: uint) -> ChannelMode {
        // This should never fail because only ChannelMode
        // can be put into the EnumSet
        FromPrimitive::from_uint(val).unwrap()
    }
}

fn add_flag(nick: &str, privs: &Privileges) -> String {
    if privs.contains_elem(OperatorPrivilege) {
        "@".to_string().append(nick)
    } else if privs.contains_elem(OperatorPrivilege) {
        "+".to_string().append(nick)
    } else {
        nick.to_string()
    }
}

pub type Privileges = EnumSet<ChannelMode>;

/// Represents an IRC channel
pub struct Channel {
    name: String,
    topic: String,
    password: Vec<u8>,
    flags: Privileges,
    clients: Vec<WeaklySharedClient>,
    user_priviliges: HashMap<String, Privileges> 
}

impl Channel {
    pub fn new(name: String) -> Channel {
        Channel {
            name: name,
            topic: "".to_string(),
            password: Vec::new(),
            flags: EnumSet::empty(),
            clients: Vec::new(),
            user_priviliges: HashMap::new()
        }
    }
    
    /// Filters out invalid users refs and updates list
    /// This is only called during join and part events due 
    /// to performance considerations
    fn update_active_clients(&mut self) -> Vec<SharedClient> {
        let active_clients: Vec<SharedClient> = self.clients.iter().filter_map(
            |c| c.upgrade()).collect();
        self.clients = active_clients.iter().map(
            |c| c.downgrade()).collect();
        active_clients
    }
    
    /// Perform an action on all active clients
    fn active_clients_do(&self, closure: |SharedClient|) {
        for client in self.clients.iter().filter_map(|c| c.upgrade()) {
            closure(client)
        }
    }
    
    /// Grant a user with a privilege
    pub fn grant(&mut self, client: &SharedClient, privilege: ChannelMode) {
        self.user_priviliges.find_or_insert_with(
            client.borrow().nickname.to_string(), 
            |_| { EnumSet::empty() } 
        ).add(privilege)
    }
    
    /// Sends the list of users to the client
    pub fn handle_names(&self, client: SharedClient) {
        let mut c = client;
        for (nick, privileges) in self.user_priviliges.iter() {
            c.borrow_mut().send_response(RPL_NAMREPLY, 
                Some(String::from_str("= ").append(self.name.as_slice()).as_slice()),
//                Some(self.name.as_slice()),
                Some(add_flag(nick.as_slice(), privileges).as_slice())
            )
        }
        c.borrow_mut().send_response(RPL_ENDOFNAMES, Some(self.name.as_slice()), 
            Some("End of /NAMES list"));
    }
    
    /// Handles the join attempt of a user
    pub fn handle_join(&mut self, client: SharedClient, password: Option<&[u8]>) {
        if self.password.len() != 0 {
            if !match password { Some(password) => password == self.password.as_slice(),
                                 None => false } {
                client.borrow_mut().send_response(ERR_BADCHANNELKEY,
                    Some(self.name.as_slice()),
                    Some("Password is wrong")
                );
                return
            }
        }
        self.clients.push(client.downgrade()); // TODO check doublets
        { // rust bug, drop does not give back borrows
        let msg = Message::new(
            JOIN, 
            &[self.name.as_slice()],
            Some(client.borrow().nickname.as_slice())
        );
        self.active_clients_do(|c| c.borrow_mut().send_msg(msg.clone()));
        {
            let mut c = client.borrow_mut();
            self.user_priviliges.insert(c.nickname.clone(), EnumSet::empty());
            c.send_response(RPL_NOTOPIC, 
                Some(self.name.as_slice()), Some("No topic set."));
        }
        } // rust bug
        if self.clients.len() == 1 { // first user
            self.grant(&client, OperatorPrivilege)
        }
        self.handle_names(client);
    }
    
    /// Handles the quit/part event
    pub fn handle_leave(&mut self, client: SharedClient, mut message: Message) {
        let nickname = client.borrow().nickname.clone();
        message.set_prefix(nickname.as_slice());
        let mut client_id = None;
        for (i, chan_client) in self.update_active_clients().iter().enumerate() {
            if *client.borrow() == *chan_client.borrow() {
                client_id = Some(i)
            } else {
                chan_client.borrow_mut().send_msg(message.clone())
            }
        }
        if client_id.is_some() {
            self.clients.remove(client_id.unwrap());
        }
        self.user_priviliges.remove(&nickname);
    }
    
    /// handles the mode message
    pub fn handle_mode(&mut self, client: SharedClient, message: Message) {
        let params = message.params();
        if params.len() > 0 {
        } else {
            client.borrow_mut().send_response(RPL_CHANNELMODEIS,
                Some(self.name.as_slice()),
                Some(("+".to_string() +
                    self.flags.iter().map(
                        |c| c as u8 as char).collect::<String>() 
                    ).as_slice())
            )
        }
    }
    
    
    /// handles private messages
    pub fn handle_msg(&mut self, client: SharedClient, mut message: Message) {
        message.set_prefix(client.borrow().nickname.as_slice());
        self.active_clients_do(|c|
            if *client.borrow() != *c.borrow() {
                c.borrow_mut().send_msg(message.clone())
            }
        );
    }
    
    
}