use cmd;
use client;
use channel;
use msg::RawMessage;
use msg::util;

use server::{Server, ChannelProxy};
use client::SharedClient;

/// Handles the PART command
pub struct Part {
    raw: RawMessage,
    channels: Vec<String>,
    reason: Option<Vec<u8>> 
}
impl Part {
    fn handle_part(channel: &mut channel::Channel, client: client::ClientProxy, reason: Option<Vec<u8>>) {
        let nick = channel.member_with_id(client.id()).map(|v| v.nick().to_string());
        match nick {
            Some(nick) => {
                let msg = {
                    let payload = match reason {
                        None => vec![channel.name().as_bytes()],
                        Some(ref reason) => vec![channel.name().as_bytes(), reason.as_slice()],
                        };
                    RawMessage::new_raw(
                        cmd::PART, payload.as_slice(), Some(nick.as_bytes())
                    )
                };
                channel.broadcast(msg);
                channel.remove_member(&client.id());
            },
            None => channel.send_response(&client, cmd::ERR_NOTONCHANNEL,
                [channel.name(), "You are not on this channel."]
            )
        }
    }
}
impl super::MessageHandler for Part {
    fn from_message(message: RawMessage) -> Result<Box<Part>, RawMessage> {
        let params = message.params();
        let mut channels = Vec::new();
        if params.len() > 0 {
            for channel_name in params[0].as_slice().split(|c| *c == b',') {
                match util::verify_channel(channel_name) {
                    Some(channel) => {
                        channels.push(channel.to_string());
                    },
                    None => return Err(RawMessage::new(cmd::REPLY(cmd::ERR_NOSUCHCHANNEL), [
                        "*", String::from_utf8_lossy(channel_name).as_slice(),
                        "Invalid channel name."
                    ], None))
                }
            }
            Ok(box Part {
                raw: message.clone(),
                channels: channels,
                reason: params.as_slice().get(1).map(|v| v.to_vec())
            })
        } else {
             Err(RawMessage::new(cmd::REPLY(cmd::ERR_NEEDMOREPARAMS), [
                "*", message.command().to_string().as_slice(),
                "no params given"
            ], None))
        }
    }
    fn invoke(self, server: &mut Server, origin: SharedClient) {
        for channel_name in self.channels.iter() {
            match server.channels.find_mut(channel_name) {
                Some(channel) => {
                    let reason = self.reason.clone();
                    let proxy = origin.borrow().proxy();
                    channel.send(channel::HandleMut(proc(channel) {
                        Part::handle_part(
                            channel, 
                            proxy,
                            reason
                        )
                    }))
                },
                None => origin
                    .borrow_mut().send_response(cmd::ERR_NOSUCHCHANNEL,
                        Some(channel_name.as_slice()), Some("No such channel"))
                    
                    
            }
        }
    }
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}