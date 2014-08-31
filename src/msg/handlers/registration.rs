use cmd;
use msg::RawMessage;
use util;

use server::{Server};
use con::{Peer, Connection};
use con;




fn try_register(server: &mut Server, origin: Peer) {
    if server.nicks.contains_key(origin.info().read().nick()) {
        origin.send_response(cmd::ERR_ALREADYREGISTRED, 
            &["somebody already registered with the same nickname"],
            server.host()
        )
    } else if origin.info().read().registration_status() == con::reg::Registered {
        server.send_welcome_msg(&origin);
        server.add_user(origin);
    }
}

/// Handles the nick command
///
///    Command: NICK
/// Parameters: <nickname> [ <hopcount> ]
pub struct Nick {
    raw: RawMessage,
    nick: String
}

impl super::MessageHandler for Nick {
    fn from_message(message: RawMessage) -> Result<Box<Nick>, Option<RawMessage>> {
        let params = message.params();
        if params.len() > 0 {
            match util::verify_nick(params[0].as_slice()) {
                Some(nick) => Ok(box Nick {
                    raw: message.clone(),
                    nick: nick.to_string()
                }),
                None => 
                    Err(Some(RawMessage::new(cmd::REPLY(cmd::ERR_ERRONEUSNICKNAME), [
                        "*", String::from_utf8_lossy(params[0].as_slice()).as_slice(),
                        "invalid nick name"
                    ], None)))
            }
        } else {
            Err(Some(RawMessage::new(cmd::REPLY(cmd::ERR_NONICKNAMEGIVEN), [
                "*", "no nickname given"
            ], None)))
        }
    }
    fn invoke(self, server: &mut Server, origin: Peer) {
        if server.nicks.contains_key(&self.nick) {
            origin.send_response(cmd::ERR_NICKNAMEINUSE,
                &[self.nick.as_slice(), "nickname in use"],
                server.host()
            );
        } else {
            if server.valid_nick(self.nick.as_slice()) {
                origin.info().write().set_nick(self.nick);
                try_register(server, origin)
            }
            
        }
    }
    fn invoke_con(self, server: &mut Server, origin: Connection) {
        self.invoke(server, origin.peer())
    }
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}

pub struct User {
    raw: RawMessage,
    username: String,
    realname: String
}
impl super::MessageHandler for User {
    fn from_message(message: RawMessage) -> Result<Box<User>, Option<RawMessage>> {
        let params = message.params();
        if params.len() >= 4 {
            let username = String::from_utf8_lossy(params[0].as_slice()).to_string();
            let realname = String::from_utf8_lossy(params[3].as_slice()).to_string();
            Ok(box User {
                raw: message.clone(), username: username, realname: realname
            })
        } else {
            Err(Some(RawMessage::new(cmd::REPLY(cmd::ERR_NEEDMOREPARAMS), [
                "*", message.command().to_string().as_slice(),
                "not enought params given"
            ], None)))
        }
        
    }
    fn invoke(self, server: &mut Server, origin: Peer) {
        {
            let mut info = origin.info().write();
            info.set_username(self.username);
            info.set_realname(self.realname);
            *info.mut_registration_status() = con::reg::Registered
        
        }
        if server.valid_nick(origin.info().read().nick().as_slice()) {
            try_register(server, origin)
        }
    }
    fn invoke_con(self, server: &mut Server, origin: Connection) {
        self.invoke(server, origin.peer())
    }
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}