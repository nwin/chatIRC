use std::collections::{HashMap, HashSet};
use std::collections::hashmap;

use client::{ClientId, ClientProxy};
use msg::{RawMessage};
use msg::util::{HostMask};

use cmd;
use server;

pub use self::member::{Member};
pub use self::util::{Flags, ChannelMode, modes_do};

pub mod util;
mod member;


/// Forwards the message to a channel
pub struct Proxy {
    name: String,
    tx: Sender<Event>,
    server_tx: Sender<server::Event>
}

impl Proxy {
    pub fn new(name: String,
           tx: Sender<Event>, 
           server_tx: Sender<server::Event>) -> Proxy {
        Proxy {
            name: name,
            tx: tx,
            server_tx: server_tx
        }
    }
    pub fn send(&self, event: Event) {
        match self.tx.send_opt(event) {
            Ok(_) => {},
            Err(_) => {
                let _ = self.server_tx.send_opt(server::ChannelLost(self.name.clone()));
            }
        }
    }
}


/// Enumeration of events a channel can receive
pub enum Event {
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
    
    /// Starts listening for events in a separate thread
    pub fn listen(self) -> Sender<Event> {
        let (tx, rx) = channel();
        spawn(proc() {
            let mut this = self;
            for event in rx.iter() {
                this.dispatch(event)
            }
        });
        tx
    }

    /// Message dispatcher
    fn dispatch(&mut self, event: Event) {
        match event {
            Handle(handler) => handler(self),
            HandleMut(handler) => handler(self),
            //Message(command, client_id, message) => {
            //    match command {
            //        PRIVMSG => self.handle_privmsg(client_id, message),
            //    }
            //}
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
    
    /// Returns a view into the channel members
    pub fn members<'a>(&'a self) -> hashmap::Values<'a, String, Member> {
        self.members.values()
    }
    
    /// Adds a flag to the channel
    pub fn add_flag(&mut self, flag: ChannelMode) -> bool {
        self.flags.insert(flag)
    }
    
    /// Removes a flag from the channel
    pub fn remove_flag(&mut self, flag: ChannelMode) -> bool {
        self.flags.remove(&flag)
    }
    
    /// Checks if the channel has flag `flag`
    pub fn has_flag(&self, flag: ChannelMode) -> bool {
        self.flags.contains(&flag)
    }
    
    /// Channel flags as a string
    pub fn flags(&self) -> String {
        self.flags.iter().map( |c| *c as u8 as char).collect() 
    }
    
    /// Adds a ban mask to the channel
    pub fn add_ban_mask(&mut self, mask: HostMask) -> bool {
        self.ban_masks.insert(mask)
    }
    
    /// Removes a ban mask from the channel
    pub fn remove_ban_mask(&mut self, mask: HostMask) -> bool {
        self.ban_masks.remove(&mask)
    }
    
    /// Adds a ban mask to the channel
    pub fn add_except_mask(&mut self, mask: HostMask) -> bool {
        self.except_masks.insert(mask)
    }
    
    /// Removes a ban mask from the channel
    pub fn remove_except_mask(&mut self, mask: HostMask) -> bool {
        self.except_masks.remove(&mask)
    }
    
    /// Adds a ban mask to the channel
    pub fn add_invite_mask(&mut self, mask: HostMask) -> bool {
        self.invite_masks.insert(mask)
    }
    
    /// Removes a ban mask from the channel
    pub fn remove_invite_mask(&mut self, mask: HostMask) -> bool {
        self.invite_masks.remove(&mask)
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
    
    pub fn member_with_nick(&self, nick: &String) -> Option<&Member> {
        self.members.find(nick)
    }
    
    pub fn mut_member_with_nick(&mut self, nick: &String) -> Option<&mut Member> {
        self.members.find_mut(nick)
    }
    
    /// Broadcasts a message to all members
    #[inline]
    pub fn broadcast(&self, message: RawMessage) {
        for member in self.members() {
            member.send_msg(message.clone())
        }
    }
}

#[cfg(test)]
mod tests {
	use super::util::{modes_do, BanMask, ExceptionMask};
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