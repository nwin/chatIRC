use std::collections::{HashMap, EnumSet};
use std::collections::enum_set::{CLike};

use client::{ClientId, SharedClient, WeaklySharedClient};
use message::{RawMessage};

use cmd::*;

/// Enumeration of possible channel modes
/// as of http://tools.ietf.org/html/rfc2811#section-4
#[deriving(FromPrimitive, Show, Clone)]
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

struct Member {
    tx: Sender<RawMessage>,
    id: ClientId,
    nick: String,
    decorated_nick: String,
    flags: Flags,
}

impl Member {
    /// Creates a new member
    pub fn new(id: ClientId, nick: String, tx: Sender<RawMessage>) -> Member {
        Member {
            id: id,
            tx: tx,
            nick: nick.clone(),
            decorated_nick: nick,
            flags: EnumSet::empty()
        }
    }
    
    /// Sends a message to the client
    pub fn send_msg(&self, msg: RawMessage) {
        self.tx.send(msg)
    }
    
    /// Grant a privilege to a member
    pub fn promote(&mut self, flag: ChannelMode) {
        self.flags.add(flag)
    }
    
    /// Checks whether a member has a certain privilege
    pub fn has_privilege(&self, privilege: ChannelMode) -> bool {
        self.flags.contains_elem(privilege)
    }
    
    /// Checks whether a member has the voice privilege
    pub fn has_voice(&self) -> bool {
        self.flags.contains_elem(VoicePrivilege) 
        || self.flags.contains_elem(OperatorPrivilege) 
    }
    
    /// Updates the cached decorated nick
    fn update_decorated_name(&mut self) {
        self.decorated_nick = if self.flags.contains_elem(OperatorPrivilege) {
            "@".to_string().append(self.nick())
        } else if self.flags.contains_elem(OperatorPrivilege) {
            "+".to_string().append(self.nick())
        } else {
            self.nick.to_string()
        }
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
    
    /// Setter for nick
    pub fn set_nick(&mut self, nick: String) {
        self.nick = nick;
        self.update_decorated_name()
    }
}

impl PartialEq for Member {
    #[inline]
    fn eq(&self, other: &Member) -> bool {
        self.id == other.id
    }
}

impl Eq for Member {}

pub type Flags = EnumSet<ChannelMode>;

/// Represents an IRC channel
pub struct Channel {
    name: String,
    topic: String,
    password: Vec<u8>,
    flags: Flags,
    members: HashMap<String, Member>,
    nicknames: HashMap<ClientId, String>,
}

impl Channel {
    pub fn new(name: String) -> Channel {
        Channel {
            name: name,
            topic: "".to_string(),
            password: Vec::new(),
            flags: EnumSet::empty(),
            members: HashMap::new(),
            nicknames: HashMap::new()
        }
    }
    
    /// Sends the list of users to the client
    pub fn handle_names(&self, client: SharedClient) {
        //for (nick, privileges) in self.user_privileges.iter() {
        //    client.borrow_mut().send_response(RPL_NAMREPLY, 
        //        Some(String::from_str("= ").append(self.name.as_slice()).as_slice()),
//      //          Some(self.name.as_slice()),
        //        Some(add_flag(nick.as_slice(), privileges).as_slice())
        //    )
        //}
        //client.borrow_mut().send_response(RPL_ENDOFNAMES, Some(self.name.as_slice()), 
        //    Some("End of /NAMES list"));
    }
    
    /// Handles the join attempt of a user
    pub fn handle_join(&mut self, client: SharedClient, password: Option<&[u8]>) {
        //if self.password.len() != 0 {
        //    if !match password { Some(password) => password == self.password.as_slice(),
        //                         None => false } {
        //        client.borrow_mut().send_response(ERR_BADCHANNELKEY,
        //            Some(self.name.as_slice()),
        //            Some("Password is wrong")
        //        );
        //        return
        //    }
        //}
        //self.clients.push(client.downgrade()); // TODO check doublets
        //{ // rust bug, drop does not give back borrows
        //let msg = RawMessage::new(
        //    JOIN, 
        //    &[self.name.as_slice()],
        //    Some(client.borrow().nickname.as_slice())
        //);
        //self.active_clients_do(|c| c.borrow_mut().send_msg(msg.clone()));
        //{
        //    let mut c = client.borrow_mut();
        //    self.user_privileges.insert(c.nickname.clone(), EnumSet::empty());
        //    c.send_response(RPL_NOTOPIC, 
        //        Some(self.name.as_slice()), Some("No topic set."));
        //}
        //} // rust bug
        //if self.clients.len() == 1 { // first user
        //    self.grant(&client, OperatorPrivilege)
        //}
        //self.handle_names(client);
    }
    
    /// Handles the quit/part event
    pub fn handle_leave(&mut self, client: SharedClient, mut message: RawMessage) {
        //let nickname = client.borrow().nickname.clone();
        //message.set_prefix(nickname.as_slice());
        //let mut client_id = None;
        //for (i, chan_client) in self.update_active_clients().iter().enumerate() {
        //    if *client.borrow() == *chan_client.borrow() {
        //        client_id = Some(i)
        //    } else {
        //        chan_client.borrow_mut().send_msg(message.clone())
        //    }
        //}
        //if client_id.is_some() {
        //    self.clients.remove(client_id.unwrap());
        //}
        //self.user_privileges.remove(&nickname);
    }
    
    /// handles the mode message
    pub fn handle_mode(&mut self, client: SharedClient, message: RawMessage) {
        //let params = message.params();
        //if params.len() > 1 {
        //    if self.has_privilege(&client, OperatorPrivilege) {
        //    }
        //    
        //    error!("TODO: implement mode setting") 
        //} else {
        //    client.borrow_mut().send_response(RPL_CHANNELMODEIS,
        //        Some(self.name.as_slice()),
        //        Some(("+".to_string() +
        //            self.flags.iter().map(
        //                |c| c as u8 as char).collect::<String>() 
        //            ).as_slice())
        //    )
        //}
    }
    
    /// handles private messages
    pub fn handle_msg(&mut self, client_id: ClientId, message: RawMessage) {
        let nick = self.nicknames.find(&client_id).clone();
        let maybe_member = match nick {
            Some(nick) => self.members.find(nick),
            None => None
        };
        
        if self.flags.contains_elem(MemberOnly)
        || self.flags.contains_elem(VoicePrivilege) {
            match maybe_member {
                Some(sender) => {
                    if self.flags.contains_elem(VoicePrivilege) && !sender.has_voice() {
                        return // TODO error message
                    }
                    for (_, member) in self.members.iter() {
                        if member != sender {
                            member.send_msg(message.clone())
                        }
                    }
                },
                None => {
                    return // TODO error message
                }
            }
        } else { // Message goes to everybody
            match maybe_member {
                Some(sender) => for (_, member) in self.members.iter() {
                    if member != sender {
                        member.send_msg(message.clone())
                    }
                },
                None => for (_, member) in self.members.iter() {
                    member.send_msg(message.clone())
                }
            }
        }
    }
    
    
}