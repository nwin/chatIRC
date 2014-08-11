use cmd;
use channel;
use channel::util::{TopicProtect};
use msg::RawMessage;
use msg::util;

use server::{Server};
use client::{SharedClient, ClientProxy};

pub struct Topic {
    raw: RawMessage,
    channel: String,
    topic: Vec<u8>
}

impl Topic {
    fn set(channel: &mut channel::Channel, proxy: ClientProxy, topic: Vec<u8>) {
        let set_topic = match channel.member_with_id(proxy.id()) {
            Some(member) => {
                if channel.has_flag(TopicProtect) && !member.is_op() {
                    proxy.send_response(cmd::ERR_CHANOPRIVSNEEDED,
                        [channel.name(), "You are not a channel operator (channel is +t)."], channel.server_name()
                    );
                    false
                } else {
                    let msg = RawMessage::new_raw(cmd::TOPIC, 
                        [channel.name().as_bytes(), topic.as_slice()], Some(member.nick().as_bytes()));
                    channel.broadcast(msg);
                    true
                }
            },
            None => {
                proxy.send_response(cmd::ERR_NOTONCHANNEL,
                    [channel.name(), "You are not on this channel."],
                    channel.server_name()
                );
                false
            }
        };  
        if set_topic {
            channel.set_topic(topic);
        }
    }
}


impl super::MessageHandler for Topic {
    fn from_message(message: RawMessage) -> Result<Box<Topic>, RawMessage> {
        if message.params().len() > 0 {
            let channel = match util::verify_channel(message.params()[0]) {
                Some(channel) => channel.to_string(),
                None => return Err(RawMessage::new(cmd::REPLY(cmd::ERR_NOSUCHCHANNEL), [
                    "*", String::from_utf8_lossy(message.params()[0]).as_slice(),
                    "Invalid channel name."
                ], None))
            };  
            let topic = message.params().as_slice().get(1).unwrap_or(&b"").to_vec();
            Ok(box Topic {
                raw: message,
                channel: channel,
                topic: topic
            })
        } else {
             Err(RawMessage::new(cmd::REPLY(cmd::ERR_NEEDMOREPARAMS), [
                "*", message.command().to_string().as_slice(),
                "no channel name given"
            ], None))
        }
    }
    fn invoke(self, server: &mut Server, origin: SharedClient) {
        match server.channels.find_mut(&self.channel) {
            Some(channel) => {
                let proxy = origin.borrow().proxy();
                channel.send(channel::HandleMut(proc(channel) {
                    Topic::set(channel, proxy, self.topic)
                }))
            },
            None => origin.borrow_mut().send_response(cmd::ERR_NOSUCHCHANNEL,
                Some(self.channel.as_slice()), Some("No such channel")
            )   
        }
    }
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}