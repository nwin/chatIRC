use cmd::{REPLY, UNKNOWN, ResponseCode};
use cmd;

use super::{RawMessage};
use super::util;

macro_rules! parse_messages {
    {$(
        $name:ident for $command:ident as $enum_name:ident 
        { $($attr:ident: $ty:ty),* }
        <- fn ($message:ident) $parser:block;
    )*} => {

pub enum Message {
    $($enum_name($name),)*
    /// Numeric reply codes, see `cmd::ResponseCode`
    Reply(ResponseCode),
    /// Catch all unknown/unsupported commands
    Unknown(Vec<u8>),
}

impl Message {
    pub fn from_raw_message(message: RawMessage) -> Result<Message, RawMessage> {
        let cmd = message.command().to_bytes(); // temporary workaround the borrow checker
        match message.command() {
            $(cmd::$command => Ok($enum_name(try!($name::from_raw_message(message)))),)*
            REPLY(code) => Ok(Reply(code)),
            UNKNOWN(_) => Ok(Unknown(cmd))
        }
    }
}

$(

#[deriving(Clone)]
pub struct $name {
    pub raw: RawMessage,
    $(pub $attr: $ty,)*
}
impl $name {
    pub fn from_raw_message($message: RawMessage) -> Result<$name, RawMessage> 
        $parser
}
)*

}}

parse_messages!{
ModeMessage for MODE as Mode { receiver: ::msg::Receiver, params: Vec<Vec<u8>> } <- fn(message) {
    let params = message.params();
    if params.len() > 0 {
        match util::verify_receiver(params[0]) {
            util::InvalidReceiver(name) => return Err(RawMessage::new(REPLY(cmd::ERR_USERNOTINCHANNEL), [
                "*", message.command().to_string().as_slice(),
                format!("invalid channel name {}", name).as_slice()
                ], None)
            ),
            receiver => Ok(ModeMessage{
                raw: message.clone(),
                receiver: receiver,
                params: message.params().slice_from(1)
                               .iter().map(|v| v.to_vec()).collect()
            })
        }
    } else {
         return Err(RawMessage::new(REPLY(cmd::ERR_NEEDMOREPARAMS), [
            "*", message.command().to_string().as_slice(),
            "receiver given"
        ], None))
    }
};

PrivMessage for PRIVMSG as Priv { receiver: Vec<::msg::Receiver>, message: Vec<u8> } <- fn(message) {
    let params = message.params();
    if params.len() > 1 {
        Ok(PrivMessage {
            raw: message.clone(), 
            receiver: params[0].as_slice()
                               .split(|&v| v == b',' )
                               .map(|v| util::verify_receiver(v))
                               .collect(),
            message: params[1].to_owned()
        })
    } else {
         return Err(RawMessage::new(REPLY(cmd::ERR_NEEDMOREPARAMS), [
            "*", message.command().to_string().as_slice(),
            "not enought params given"
        ], None))
    }
};

JoinMessage for JOIN as Join { targets: Vec<String>, passwords: Vec<Option<Vec<u8>>> } <- fn(message) { 
    let params = message.params();
    let mut targets = Vec::new();
    let mut passwords = Vec::new();
    if params.len() > 0 {
        let channels_passwords: Vec<&[u8]> = if params.len() > 1 {
            params[1].as_slice().split(|c| *c == b',').collect()
        } else {
            Vec::new()
        };
        for (i, channel_name) in params[0].as_slice().split(|c| *c == b',').enumerate() {
            match util::verify_channel(channel_name) {
                Some(channel) => {
                    targets.push(channel.to_string());
                    if channels_passwords.len() > i {
                        passwords.push(Some(channels_passwords[i].to_owned()));
                    } else {
                        passwords.push(None);
                    }
                },
                None => return Err(RawMessage::new(REPLY(cmd::ERR_NOSUCHCHANNEL), [
                    "*", String::from_utf8_lossy(channel_name).as_slice(),
                    "Invalid channel name."
                ], None))
            }
        }
    } else {
         return Err(RawMessage::new(REPLY(cmd::ERR_NEEDMOREPARAMS), [
            "*", message.command().to_string().as_slice(),
            "no params given"
        ], None))
    }
    Ok(JoinMessage {
        raw: message.clone(), targets: targets, passwords: passwords
    })
};

NamesMessage for NAMES as Names { receivers: Vec<util::Receiver> } <- fn(message) { 
    if message.params().len() > 0 {
        Ok(NamesMessage {
            raw: message.clone(),
            receivers: message.params()[0].as_slice().split(|c| *c == b',').map(|v|
                util::verify_receiver(v)
            ).collect()
        })
    } else {
        Err(RawMessage::new(REPLY(cmd::ERR_NEEDMOREPARAMS), [
            "*", message.command().to_string().as_slice(),
            "not enought params given"
        ], None))
    }
};

QuitMessage for QUIT as Quit { reason: Option<String> } <- fn(message) { 
    let reason = message.params().as_slice().get(0).map(
        |&v| String::from_utf8_lossy(v).to_string());
    Ok(QuitMessage {
        raw: message, reason: reason
    })
};

PartMessage for PART as Part { channels: Vec<String>, reason: Option<Vec<u8>> } <- fn(message) {
    let params = message.params();
    let mut channels = Vec::new();
    if params.len() > 0 {
        for channel_name in params[0].as_slice().split(|c| *c == b',') {
            match util::verify_channel(channel_name) {
                Some(channel) => {
                    channels.push(channel.to_string());
                },
                None => return Err(RawMessage::new(REPLY(cmd::ERR_NOSUCHCHANNEL), [
                    "*", String::from_utf8_lossy(channel_name).as_slice(),
                    "Invalid channel name."
                ], None))
            }
        }
        Ok(PartMessage{
            raw: message.clone(),
            channels: channels,
            reason: params.as_slice().get(1).map(|v| v.to_vec())
        })
    } else {
         Err(RawMessage::new(REPLY(cmd::ERR_NEEDMOREPARAMS), [
            "*", message.command().to_string().as_slice(),
            "no params given"
        ], None))
    }
};


PingMessage for PING as Ping { payload: Option<String> } <- fn(message) {
    let payload = message.params().as_slice().get(0).map(
        |&v| String::from_utf8_lossy(v).to_string());
    Ok(PingMessage {
        raw: message, payload: payload
    })
};

PongMessage for PONG as Pong { payload: Option<String> } <- fn(message) { 
    let payload = message.params().as_slice().get(0).map(
        |&v| String::from_utf8_lossy(v).to_string());
    Ok(PongMessage {
        raw: message, payload: payload
    })
};


NickMessage for NICK as Nick { nick: String } <- fn(message) { 
    let params = message.params();
    if params.len() > 0 {
        match util::verify_nick(params[0].as_slice()) {
            Some(nick) => Ok(NickMessage {
                raw: message.clone(),
                nick: nick.to_string()
            }),
            None => 
                Err(RawMessage::new(REPLY(cmd::ERR_ERRONEUSNICKNAME), [
                    "*", String::from_utf8_lossy(params[0].as_slice()).as_slice(),
                    "invalid nick name"
                ], None))
        }
    } else {
        Err(RawMessage::new(REPLY(cmd::ERR_NONICKNAMEGIVEN), [
            "*", "no nickname given"
        ], None))
    }
};

UserMessage for USER as User { username: String, realname: String } <- fn(message) { 
    let params = message.params();
    if params.len() >= 4 {
        let username = String::from_utf8_lossy(params[0].as_slice()).to_string();
        let realname = String::from_utf8_lossy(params[3].as_slice()).to_string();
        Ok(UserMessage {
            raw: message.clone(), username: username, realname: realname
        })
    } else {
        Err(RawMessage::new(REPLY(cmd::ERR_NEEDMOREPARAMS), [
            "*", message.command().to_string().as_slice(),
            "not enought params given"
        ], None))
    }
};

}