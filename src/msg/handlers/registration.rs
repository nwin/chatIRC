use cmd;
use msg::RawMessage;
use util;

use server::{Server};
use client::SharedClient;




fn try_register(server: &mut Server, origin: &SharedClient) {
    let nick = origin.borrow().nickname.clone();
    if nick.len() > 0 && server.registered.contains_key(&nick) {
        origin.borrow().send_response(cmd::ERR_ALREADYREGISTRED, None,
            Some("somebody already registered with the same nickname")
        );
    } else {
        server.registered.insert(nick, origin.clone());
        server.send_welcome_msg(origin);
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
    fn from_message(message: RawMessage) -> Result<Box<Nick>, RawMessage> {
        let params = message.params();
        if params.len() > 0 {
            match util::verify_nick(params[0].as_slice()) {
                Some(nick) => Ok(box Nick {
                    raw: message.clone(),
                    nick: nick.to_string()
                }),
                None => 
                    Err(RawMessage::new(cmd::REPLY(cmd::ERR_ERRONEUSNICKNAME), [
                        "*", String::from_utf8_lossy(params[0].as_slice()).as_slice(),
                        "invalid nick name"
                    ], None))
            }
        } else {
            Err(RawMessage::new(cmd::REPLY(cmd::ERR_NONICKNAMEGIVEN), [
                "*", "no nickname given"
            ], None))
        }
    }
    fn invoke(self, server: &mut Server, origin: SharedClient) {
        if server.registered.contains_key(&self.nick) {
            origin.borrow().send_response(
                cmd::ERR_NICKNAMEINUSE,
                Some(self.nick.as_slice()), 
                Some("nickname in use")
            );
        } else {
            origin.borrow_mut().nickname = self.nick;
        }
        if origin.borrow().username.len() > 0 && !server.registered.contains_key(&origin.borrow().nickname){
            // user message already send but not yet registered
            try_register(server, &origin)
        }
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
    fn from_message(message: RawMessage) -> Result<Box<User>, RawMessage> {
        let params = message.params();
        if params.len() >= 4 {
            let username = String::from_utf8_lossy(params[0].as_slice()).to_string();
            let realname = String::from_utf8_lossy(params[3].as_slice()).to_string();
            Ok(box User {
                raw: message.clone(), username: username, realname: realname
            })
        } else {
            Err(RawMessage::new(cmd::REPLY(cmd::ERR_NEEDMOREPARAMS), [
                "*", message.command().to_string().as_slice(),
                "not enought params given"
            ], None))
        }
        
    }
    fn invoke(self, server: &mut Server, origin: SharedClient) {
        origin.borrow_mut().username = self.username;
        origin.borrow_mut().realname = self.realname;
        try_register(server, &origin)
    }
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}