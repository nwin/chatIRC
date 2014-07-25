use std::collections::{HashMap, HashSet};

use client::{ClientId};
use message::{RawMessage};

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

/// Represents a channel member
pub struct Member {
    tx: Sender<RawMessage>,
    id: ClientId,
    nick: String,
    decorated_nick: String,
    flags: Flags,
    server_name: String,
}

impl Member {
    /// Creates a new member
    pub fn new(id: ClientId, nick: String, server_name: String, tx: Sender<RawMessage>) -> Member {
        Member {
            id: id,
            tx: tx,
            nick: nick.clone(),
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
        self.tx.send(msg)
    }
    
    /// Grant a privilege to a member
    pub fn promote(&mut self, flag: ChannelMode) {
        self.flags.insert(flag);
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
enum ChannelCommand {
    PRIVMSG,
    LEAVE,
    MODE
}

/// Enumeration of events a channel can receive
enum ChannelEvent {
    Message(ClientId, ChannelCommand, RawMessage),
    Join(Member, Option<Vec<u8>>),
    Reply(ClientId, ChannelCommand, |RawMessage|: 'static+Send)
}

/// Represents an IRC channel
pub struct Channel {
    name: String,
    topic: String,
    password: Vec<u8>,
    flags: Flags,
    members: HashMap<String, Member>,
    nicknames: HashMap<ClientId, String>,
    server_name: String,
}

impl Channel {
    pub fn new(name: String, server_name: String) -> Channel {
        Channel {
            name: name,
            topic: "".to_string(),
            password: Vec::new(),
            flags: HashSet::new(),
            members: HashMap::new(),
            nicknames: HashMap::new(),
            server_name: server_name,
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
            Message(client_id, command, message) => {
                match command {
                    PRIVMSG => self.handle_privmsg(client_id, message),
                    LEAVE => self.handle_leave(client_id, message),
                    MODE => self.handle_mode(client_id, message),
                }
            }
            Join(member, password) => 
                self.handle_join(member, password),
            Reply(client_id, _, callback) => 
                self.handle_names(client_id, callback)
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
    pub fn handle_names(&self, client_id: ClientId, sender: |RawMessage|) {
        // TODO check if channel is visible to user…
        for (_, member) in self.members.iter() {
            sender(
                RawMessage::new(cmd::REPLY(cmd::RPL_NAMREPLY), 
                [String::from_str("= ").append(self.name.as_slice()).as_slice(),
                 member.decorated_nick()   
                ],
                Some(self.server_name.as_slice()))
            )
        }
        sender(
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
        self.handle_names(id, |msg| {
            member.send_msg(msg)
        });
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
    
    /// handles the mode message
    pub fn handle_mode(&mut self, client_id: ClientId, message: RawMessage) {
        let member = match self.member_with_id(client_id) {
            Some(member) => member,
            None => return // TODO error message
        };
        let params = message.params();
        if params.len() > 1 {
            if member.is_op() {
            }
            
            error!("TODO: implement mode setting") 
        } else {
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