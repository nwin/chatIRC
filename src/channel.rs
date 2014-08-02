use std::collections::{HashMap, HashSet};

use client::{ClientId, ClientProxy};
use msg::{RawMessage};
use util::{HostMask};

use cmd;

/// Enumeration of possible channel modes
/// as of http://tools.ietf.org/html/rfc2811#section-4
#[deriving(FromPrimitive, Show, Clone, Hash, PartialEq, Eq)]
pub enum ChannelMode {
    /// give "channel creator" status
    ChannelCreator = b'O' as int,
    /// give/take channel operator privilege
    OperatorPrivilege = b'o' as int,
    /// give/take the voice privilege
    VoicePrivilege = b'v' as int,
    /// toggle the anonymous channel flag
    AnonChannel = b'a' as int,
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

impl ChannelMode {
    fn has_parameter(&self) -> bool {
        match *self {
            ChannelKey | UserLimit | BanMask
            | ExceptionMask | InvitationMask => true,
            _ => false
        }
    }
}

/// Parses the channel modes
///
/// According to [RFC 2812] (http://tools.ietf.org/html/rfc2812#section-3.2.3) the
/// syntax for setting modes is:
/// ```
///    Command: MODE
/// Parameters: <channel> *( ( "-" / "+" ) *<modes> *<modeparams> )
/// ```
///
/// Additionally an example is given
///
/// ```
/// MODE &oulu +b *!*@*.edu +e *!*@*.bu.edu
///                                 ; Command to prevent any user from a
///                                 hostname matching *.edu from joining,
///                                 except if matching *.bu.edu
/// 
/// MODE #bu +be *!*@*.edu *!*@*.bu.edu
///                                 ; Comment to prevent any user from a
///                                 hostname matching *.edu from joining,
///                                 except if matching *.bu.edu
/// ```
/// 
/// 
fn modes_do(slice: &[&[u8]], block: |bool, ChannelMode, Option<&[u8]>|) {
    let mut current = slice;
    loop {
        let set = match current[0][0] {
            b'+' => true,
            b'-' => false,
            _ => {
                if current.len() > 1 {
                    current = current.slice_from(1);
                    continue
                } else { break }
            }
            
        };
        for mode in current[0].slice_from(1).iter().filter_map( |&v| {
            let m: Option<ChannelMode> = FromPrimitive::from_u8(v); m
        }) {
            let param = if mode.has_parameter() {
                let param = current.get(1).map(|v| *v);
                if current.len() > 1 {
                    current = current.slice_from(1);
                } else { current = &[]; }
                param
            } else {
                None
            };
            block(set, mode, param);
        }
        if current.len() > 1 {
            current = current.slice_from(1);
        } else { break }
    }
}

/// Represents a channel member
pub struct Member {
    id: ClientId,
    proxy: ClientProxy,
    nick: String,
    mask: HostMask,
    decorated_nick: String,
    flags: Flags,
    server_name: String,
}





impl Member {
    /// Creates a new member
    pub fn new(id: ClientId, nick: String, mask: HostMask, server_name: String, proxy: ClientProxy) -> Member {
        Member {
            id: id,
            proxy: proxy,
            nick: nick.clone(),
            mask: mask,
            decorated_nick: nick,
            flags: HashSet::new(),
            server_name: server_name
        }
    }
    
    pub fn send_response(&self, command: cmd::ResponseCode, params: &[&str]) {
        self.send_msg(RawMessage::new(cmd::REPLY(command), 
            params,
            Some(self.server_name.as_slice())))
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
    
    /// Get flags as string
    pub fn flags(&self) -> String {
        "+".to_string() + self.flags.iter().map(
            |c| *c as u8 as char).collect::<String>()
    }
    
    /// Checks whether a member is the operator of the channel
    pub fn is_op(&self) -> bool {
        self.flags.contains(&OperatorPrivilege) 
    }
    
    /// Checks whether a member has the voice privilege
    pub fn has_voice(&self) -> bool {
        self.flags.contains(&VoicePrivilege) 
        || self.flags.contains(&OperatorPrivilege) 
    }
    
    /// Updates the cached decorated nick
    fn update_decorated_nick(&mut self) {
        self.decorated_nick = if self.flags.contains(&OperatorPrivilege) {
            "@".to_string().append(self.nick())
        } else if self.flags.contains(&VoicePrivilege) {
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

/// Enumertion of channel modes / member flags
pub type Flags = HashSet<ChannelMode>;

/// Enumeration of command a channel can handle
pub enum ChannelCommand {
    PRIVMSG,
    LEAVE,
    MODE
}
pub enum ChannelResponse {
    NAMES
}

/// Enumeration of events a channel can receive
pub enum ChannelEvent {
    Message(ChannelCommand, ClientId, RawMessage),
    Join(Member, Option<Vec<u8>>),
    Reply(ChannelResponse, ClientId, ClientProxy)
}

/// Represents an IRC channel
pub struct Channel {
    name: String,
    server_name: String,
    topic: String,
    password: Vec<u8>,
    flags: Flags,
    members: HashMap<String, Member>,
    nicknames: HashMap<ClientId, String>,
    ban_masks: HashSet<HostMask>,
    except_masks: HashSet<HostMask>,
    invite_masks: HashSet<HostMask>,
}

impl Channel {
    pub fn new(name: String, server_name: String) -> Channel {
        Channel {
            name: name,
            server_name: server_name,
            topic: "".to_string(),
            password: Vec::new(),
            flags: HashSet::new(),
            members: HashMap::new(),
            nicknames: HashMap::new(),
            ban_masks: HashSet::new(),
            except_masks: HashSet::new(),
            invite_masks: HashSet::new(),
        }
    }
    
    /// Starts listening for events in a separate thread
    pub fn listen(self) -> Sender<ChannelEvent> {
        let (tx, rx) = channel();
        spawn(proc() {
            let mut this = self;
            for event in rx.iter() {
                this.dispatch(event)
            }
        });
        tx
    }
    
    fn dispatch(&mut self, event: ChannelEvent) {
        match event {
            Message(command, client_id, message) => {
                match command {
                    PRIVMSG => self.handle_privmsg(client_id, message),
                    LEAVE => self.handle_leave(client_id, message),
                    MODE => self.handle_mode(client_id, message),
                }
            }
            Join(member, password) => 
                self.handle_join(member, password),
            Reply( _, client_id, proxy) => 
                self.handle_names(client_id, &proxy)
        }
    }
    
    fn member_with_id(&self, client_id: ClientId) -> Option<&Member> {
        let nick = self.nicknames.find(&client_id).clone();
        match nick {
            Some(nick) => self.members.find(nick),
            None => None
        }
    }
    
    fn mut_member_with_id(&mut self, client_id: ClientId) -> Option<&mut Member> {
        let nick = self.nicknames.find(&client_id).clone();
        match nick {
            Some(nick) => self.members.find_mut(nick),
            None => None
        }
    }
    
    fn broadcast_mode(&self, member: &Member) {
        let msg = RawMessage::new(cmd::MODE, [
            self.name.as_slice(),
            member.flags().as_slice(), 
            member.nick()], Some(self.server_name.as_slice()));
        for (_, member) in self.members.iter() {
            member.send_msg(msg.clone())
        }
    }
    
    /// Sends the list of users to the client
    pub fn handle_names(&self, client_id: ClientId, proxy: &ClientProxy) {
        // TODO check if channel is visible to user…
        for (_, member) in self.members.iter() {
            proxy.send_msg(
                RawMessage::new(cmd::REPLY(cmd::RPL_NAMREPLY), 
                [String::from_str("= ").append(self.name.as_slice()).as_slice(),
                 member.decorated_nick()   
                ],
                Some(self.server_name.as_slice()))
            )
        }
        proxy.send_msg(
            RawMessage::new(cmd::REPLY(cmd::RPL_ENDOFNAMES), 
            [self.name.as_slice(), "End of /NAMES list"],
            Some(self.server_name.as_slice()))
        )
    }
    
    /// Handles the join attempt of a user
    pub fn handle_join(&mut self, mut member: Member, password: Option<Vec<u8>>) {
        if self.password.len() != 0 {
            if !match password { Some(password) => password == self.password,
                                 None => false } {
                member.send_response(cmd::ERR_BADCHANNELKEY,
                    [self.name.as_slice(),
                    "Password is wrong"]
                );
                return
            }
        }
        if self.member_with_id(member.id()).is_some() {
            //member already in channel
            return
        }
        member.send_response(cmd::RPL_NOTOPIC, 
            [self.name.as_slice(), "No topic set."]);
        let msg = RawMessage::new(
            cmd::JOIN, 
            &[self.name.as_slice()],
            Some(member.nick())
        );
        if self.members.len() == 0 { // first user
            member.promote(ChannelCreator);
            member.promote(OperatorPrivilege);
        }
        let id = member.id().clone();
        self.nicknames.insert(member.id(), member.nick().to_string());
        self.members.insert(member.nick().to_string(), member);
        for (_, member) in self.members.iter() {
            member.send_msg(msg.clone())
        }
        let member = self.member_with_id(id).unwrap();
        self.handle_names(id, member.proxy());
        if self.members.len() == 1 { // first user
            self.broadcast_mode(member)
        }
    }
    
    /// Handles the quit/part event
    pub fn handle_leave(&mut self, client_id: ClientId, mut message: RawMessage) {
        let nick = {
            let origin = match self.member_with_id(client_id) {
                Some(member) => member,
                None => return // TODO error message
            };
            message.set_prefix(origin.nick());
            for (_, member) in self.members.iter() {
                if origin != member {
                    member.send_msg(message.clone())
                }
            }
            origin.nick().to_string()
        };
        self.nicknames.remove(&client_id);
        self.members.remove(&nick);
    }
    
    /// Handles the channel mode message
    pub fn handle_mode(&mut self, client_id: ClientId, message: RawMessage) {
        let is_op = { match self.member_with_id(client_id) {
            Some(member) => member.is_op(),
            None => false
        }};
        let params = message.params();
        if params.len() > 1 {
            if !is_op { return } // TODO: error message
            modes_do(params.slice_from(1), | set, mode, parameter | {
                match mode {
                    AnonChannel | InviteOnly | Moderated | MemberOnly 
                    | Quiet | Private | Secret | ReOpFlag | TopicProtect => {
                        if set {
                            self.flags.insert(mode);
                        } else {
                            self.flags.remove(&mode);
                        }
                        
                    },
                    OperatorPrivilege | VoicePrivilege => {
                        match parameter { Some(name) => {
                                match self.members
                                .find_mut(&name.to_string()) {
                                    Some(member) => if set {
                                        member.promote(mode)
                                    } else {
                                        member.demote(mode)
                                    }, None => {}
                                }
                            }, None => {}
                        }
                    },
                    ChannelKey => {
                        match parameter { Some(password) => {
                                self.password = password.to_vec()
                            }, None => {}
                        }
                    },
                    UserLimit => {
                        error!("UserLimit mode not implemented yet")
                    },
                    BanMask | ExceptionMask | InvitationMask => match parameter { 
                        Some(mask) => {
                            let host_mask = HostMask::new(
                                String::from_utf8_lossy(mask).to_string()
                            );
                            match mode {
                                BanMask =>
                                    {let _ = self.ban_masks.insert(host_mask);},
                                ExceptionMask =>
                                    {let _ = self.except_masks.insert(host_mask);},
                                InvitationMask =>
                                    {let _ = self.invite_masks.insert(host_mask);},
                                _ => unreachable!()
                            }
                        }, None => {}
                    },
                    ChannelCreator => {
                        // This is can't be set after channel creation 
                    },
                }
            });
        } else {
            let member = match self.member_with_id(client_id) {
                Some(member) => member,
                None => return // todo error message
            };
            member.send_response(cmd::RPL_CHANNELMODEIS,
                [self.name.as_slice(), 
                 ("+".to_string() + self.flags.iter().map( |c| 
                     *c as u8 as char).collect::<String>() 
                 ).as_slice()
                ]
            )
        }
    }
    
    /// handles private messages
    pub fn handle_privmsg(&mut self, client_id: ClientId, message: RawMessage) {
        let nick = self.nicknames.find(&client_id).clone();
        let maybe_member = match nick {
            Some(nick) => self.members.find(nick),
            None => None
        };
        
        if self.flags.contains(&MemberOnly)
        || self.flags.contains(&VoicePrivilege) {
            match maybe_member {
                Some(sender) => {
                    if self.flags.contains(&VoicePrivilege) && !sender.has_voice() {
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

#[cfg(test)]
mod tests {
	use super::{modes_do, BanMask, ExceptionMask};
    use message::{RawMessage};
	/// Tests the mode parser
    
    
    
	#[test]
	fn test_mode_parser() {
        let msgs = [
            b"MODE &oulu +b *!*@*.edu +e *!*@*.bu.edu",
            b"MODE #bu +be *!*@*.edu *!*@*.bu.edu",
            b"MODE #bu /i", // Invalid mode should be skipped
            b"MODE #bu +g", // Invalid mode should be skipped
        ];
        let modes = [
            vec![(true, BanMask, Some(b"*!*@*.edu")),
            (true, ExceptionMask, Some(b"*!*@*.bu.edu"))],
            vec![(true, BanMask, Some(b"*!*@*.edu")),
            (true, ExceptionMask, Some(b"*!*@*.bu.edu"))],
            Vec::new(),
            Vec::new(),
        ];
        for (msg, modes) in msgs.iter().zip(modes.iter()) {
            let m = RawMessage::parse(*msg).unwrap();
            let mut mode_iter = modes.iter();
            modes_do(m.params().slice_from(1), |set, mode, parameter| {
                println!("{}",set);
                let (set_, mode_, parameter_) = *mode_iter.next().unwrap();
                assert_eq!(set_, set);
                assert_eq!(mode_, mode);
                assert_eq!(parameter_.to_string(), parameter.to_string());
            })
        }
	}
}