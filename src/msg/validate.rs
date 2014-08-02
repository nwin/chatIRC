use cmd::{REPLY};
use cmd;

use super::{RawMessage};
use super::util;

macro_rules! parse_messages {
    {$(
        $name:ident for $command:ident as $enum_name:ident 
        { $($attr:ident: $ty:ty),+ }
        <- fn ($message:ident) $parser:block;
    )*} => {

pub enum Message {
    $($enum_name($name),)*
}

impl Message {
    pub fn from_raw_message(message: RawMessage) -> Result<Message, RawMessage> {
        match message.command() {
            $(cmd::$command => Ok($enum_name(try!($name::from_raw_message(message)))),)*
            _ => fail!("Not all commands handled yet")
        }
    }
}

$(

pub struct $name {
    $($attr: $ty,)*
}
impl $name {
    pub fn from_raw_message($message: RawMessage) -> Result<$name, RawMessage> 
        $parser
}
)*

}}

parse_messages!{
PrivMsg for PRIVMSG as Priv { receiver: Vec<::msg::Receiver>, message: Vec<u8> } <- fn(message) {
    let params = message.params();
    if params.len() > 1 {
        Ok(PrivMsg {
            receiver: params[0].as_slice()
                               .split(|&v| v == b',' )
                               .map(|v| util::verify_receiver(v))
                               .collect(),
            message: params[1].to_owned()
        })
    } else {
         return Err(RawMessage::new(REPLY(cmd::ERR_NEEDMOREPARAMS), [
            message.command().to_string().as_slice(),
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
                    String::from_utf8_lossy(channel_name).as_slice(),
                    "Invalid channel name."
                ], None))
            }
        }
    } else {
         return Err(RawMessage::new(REPLY(cmd::ERR_NEEDMOREPARAMS), [
            message.command().to_string().as_slice(),
            "no params given"
        ], None))
    }
    Ok(JoinMessage {
        targets: targets, passwords: passwords
    })
};

}