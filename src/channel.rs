use std::collections::{HashMap, HashSet};

use client::{ClientId, ClientProxy};
use msg::{RawMessage};
use msg::util::{HostMask};
use msg;

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
pub fn modes_do(slice: &[&[u8]], block: |bool, ChannelMode, Option<&[u8]>|) {
    let mut current = slice;
    loop {
        // Bug: no +/- asking for modes
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
        if self.flags.contains(&OperatorPrivilege) {
            "@".to_string()
        } else if self.flags.contains(&VoicePrivilege) {
            "+".to_string()
        } else {
            "".to_string()
        }
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

/// Enumertion of channel modes / member flags
pub type Flags = HashSet<ChannelMode>;

/// Enumeration of command a channel can handle
pub enum ChannelCommand {
    PRIVMSG,
}
pub enum ChannelResponse {
    NAMES
}

/// Enumeration of events a channel can receive
pub enum ChannelEvent {
    Message(ChannelCommand, ClientId, RawMessage), // This will be removed later
    Who(ClientProxy, msg::WhoMessage),
    Reply(ChannelResponse, ClientProxy),
    Handle(proc(&Channel): Send),
    HandleMut(proc(&mut Channel): Send),
}

/// Represents an IRC channel
pub struct Channel {
    name: String,
    server_name: String,
    topic: String,
    pub password: Vec<u8>,
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
    
    /// Getter for channel name
    pub fn name(&self) -> &str {
        self.name.as_slice()
    }
    
    /// Getter for server name
    pub fn server_name(&self) -> &str {
        self.name.as_slice()
    }
    
    /// Returns the member count
    pub fn member_count(&self) -> uint {
        self.members.len()
    }
    
    /// Adds a flag to the channel
    pub fn add_flag(&mut self, flag: ChannelMode) -> bool {
        self.flags.insert(flag)
    }
    
    /// Removes a flag from the channel
    pub fn remove_flag(&mut self, flag: &ChannelMode) -> bool {
        self.flags.remove(flag)
    }
    
    /// Adds a ban mask to the channel
    pub fn add_ban_mask(&mut self, mask: HostMask) -> bool {
        self.ban_masks.insert(mask)
    }
    
    /// Removes a ban mask from the channel
    pub fn remove_ban_mask(&mut self, mask: &HostMask) -> bool {
        self.ban_masks.remove(mask)
    }
    
    /// Adds a ban mask to the channel
    pub fn add_except_mask(&mut self, mask: HostMask) -> bool {
        self.except_masks.insert(mask)
    }
    
    /// Removes a ban mask from the channel
    pub fn remove_except_mask(&mut self, mask: &HostMask) -> bool {
        self.except_masks.remove(mask)
    }
    
    /// Adds a ban mask to the channel
    pub fn add_invite_mask(&mut self, mask: HostMask) -> bool {
        self.invite_masks.insert(mask)
    }
    
    /// Removes a ban mask from the channel
    pub fn remove_invite_mask(&mut self, mask: &HostMask) -> bool {
        self.invite_masks.remove(mask)
    }
    
    /// Removes a flag to the channel
    pub fn flags(&self) -> String {
        self.flags.iter().map( |c| *c as u8 as char).collect() 
    }
    
    /// Adds a member to the channel
    pub fn add_member(&mut self, member: Member) -> bool {
        if self.member_with_id(member.id()).is_some() {
            false // member already in channel
        } else {
            self.nicknames.insert(member.id(), member.nick().to_string());
            self.members.insert(member.nick().to_string(), member);
            true
        }
    }
    
    /// Adds a member to the channel
    pub fn remove_member(&mut self, id: &ClientId) -> bool {
        let nick = { match self.nicknames.find(id) {
                Some(nick) => nick.clone(),
                None => return false
        }};
        self.nicknames.remove(id);
        self.members.remove(&nick);
        true
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
    
    pub fn send_response(&self, client: &ClientProxy, command: cmd::ResponseCode, 
                         params: &[&str]) {
        client.send_response(
            command, 
            params,
            self.server_name.as_slice()
        )
    }
    
    pub fn member_with_id(&self, client_id: ClientId) -> Option<&Member> {
        let nick = self.nicknames.find(&client_id).clone();
        match nick {
            Some(nick) => self.members.find(nick),
            None => None
        }
    }
    
    pub fn mut_member_with_id(&mut self, client_id: ClientId) -> Option<&mut Member> {
        let nick = self.nicknames.find(&client_id).clone();
        match nick {
            Some(nick) => self.members.find_mut(nick),
            None => None
        }
    }
    
    pub fn mut_member_with_nick(&mut self, nick: &String) -> Option<&mut Member> {
        self.members.find_mut(nick)
    }
    
    /// Broadcasts a message to all members
    #[inline]
    pub fn broadcast(&self, message: RawMessage) {
        for (_, member) in self.members.iter() {
            member.send_msg(message.clone())
        }
    }
    
    /// Message dispatcher
    fn dispatch(&mut self, event: ChannelEvent) {
        match event {
            Handle(handler) => handler(self),
            HandleMut(handler) => handler(self),
            Message(command, client_id, message) => {
                match command {
                    PRIVMSG => self.handle_privmsg(client_id, message),
                }
            },
            Who(proxy, msg) => self.handle_who(proxy, msg),
            Reply( _, proxy) => 
                self.handle_names(&proxy)
        }
    }
    
    /// Sends the list of users to the client
    pub fn handle_names(&self, proxy: &ClientProxy) {
        // TODO check if channel is visible to user…
        // TODO replace with generic list sending function
        for (_, member) in self.members.iter() {
            self.send_response(proxy, cmd::RPL_NAMREPLY, [
                String::from_str("= ").append(self.name.as_slice()).as_slice(),
                member.decorated_nick()   
            ])
        }
        self.send_response(proxy, cmd::RPL_ENDOFNAMES, 
            [self.name.as_slice(), "End of /NAMES list"])
    }
    
    /// Handles the who message
    /// The reply consists of two parts:
    /// 
    /// ```
    /// 352    RPL_WHOREPLY
    ///        "<channel> <user> <host> <server> <nick>
    ///        ( "H" / "G" > ["*"] [ ( "@" / "+" ) ]
    ///        :<hopcount> <real name>"
    /// 
    /// 315    RPL_ENDOFWHO
    ///        "<name> :End of WHO list"
    /// ```
    /// 
    /// Unfortunately the RFC 2812 does not specify what H, G, *, @ or + mean.
    /// @/+ is op/voice.
    /// * is maybe irc op
    /// H/G means here/gone in terms of the away status
    /// 
    pub fn handle_who(&mut self, client: ClientProxy, message: msg::WhoMessage) {
        if (self.flags.contains(&Private) || self.flags.contains(&Secret))
        && !self.member_with_id(client.id()).is_some() {
            // Don't give information about this channel to the outside
            // this should also be ok for secret because RPL_ENDOFWHO is
            // always sent.
        } else {
            for (_, member) in self.members.iter() {
                self.send_response(&client, cmd::RPL_WHOREPLY, [
                    self.name.as_slice(),
                    member.username(),
                    member.hostname(),
                    self.server_name.as_slice(),
                    member.nick(),
                    format!("{}{}{}", 
                        "H", // always here as long away is not implemented
                        "", // * is not supported yet
                        member.decoration()
                    ).as_slice(),
                    format!("0 {}", member.realname()).as_slice()
                ]);
            }
        }
        self.send_response(&client, cmd::RPL_ENDOFWHO, [
            message.mask.as_slice(), "End of WHO list"
        ]);
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
                None => self.broadcast(message)
            }
        }
    }
}

#[cfg(test)]
mod tests {
	use super::{modes_do, BanMask, ExceptionMask};
    use msg::{RawMessage};
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