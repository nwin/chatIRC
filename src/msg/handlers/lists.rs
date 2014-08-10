use cmd;
use channel;
use channel::{Channel, Secret, Private};
use msg::RawMessage;
use msg::util;

use server::{Server, ChannelProxy};
use client::{SharedClient, ClientProxy};

/// Handles the WHO message
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
pub struct Who {
    raw: RawMessage,
    mask: String, 
    op_only: bool
}
impl Who {
    pub fn handle_who(&self, channel: &Channel, client: ClientProxy) {
        if (channel.has_flag(Private) || channel.has_flag(Secret))
        && !channel.member_with_id(client.id()).is_some() {
            // Don't give information about this channel to the outside
            // this should also be ok for secret because RPL_ENDOFWHO is
            // always sent.
        } else {
            for member in channel.members() {
                channel.send_response(&client, cmd::RPL_WHOREPLY, [
                    channel.name(),
                    member.username(),
                    member.hostname(),
                    channel.server_name(),
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
        channel.send_response(&client, cmd::RPL_ENDOFWHO, [
            self.mask.as_slice(), "End of WHO list"
        ]);
    }
}
impl super::MessageHandler for Who {
    fn from_message(message: RawMessage) -> Result<Box<Who>, RawMessage> {
        let mask = message.params().as_slice().get(0).map_or("0".to_string(),
            |&v| String::from_utf8_lossy(v).to_string());
        let op_only = match message.params().as_slice().get(1) {
            Some(&o) => o == b"o",
            None => false
        };
        Ok(box Who {
            raw: message, mask: mask, op_only: op_only
        })
    }
    fn invoke(self, server: &mut Server, origin: SharedClient) {
        match server.channels.find(&self.mask) {
            Some(channel) => {
                let proxy = origin.borrow().proxy();
                channel.send(channel::Handle(proc(channel) {
                    self.handle_who(channel, proxy)
                }))
            },
            None => {} // handle later
        }
    }
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}

/// Handles NAMES message
pub struct Names {
    raw: RawMessage,
    receivers: Vec<util::Receiver>
}
impl Names {
    /// Sends the list of users to the client
    pub fn handle_names(channel: &Channel, proxy: &ClientProxy) {
        // TODO check if channel is visible to user…
        // TODO replace with generic list sending function
        for member in channel.members() {
            channel.send_response(proxy, cmd::RPL_NAMREPLY, [
                String::from_str("= ").append(channel.name()).as_slice(),
                member.decorated_nick()   
            ])
        }
        channel.send_response(proxy, cmd::RPL_ENDOFNAMES, 
            [channel.name(), "End of /NAMES list"])
    }
}
impl super::MessageHandler for Names {
    fn from_message(message: RawMessage) -> Result<Box<Names>, RawMessage> {
        if message.params().len() > 0 {
            Ok(box Names {
                raw: message.clone(),
                receivers: message.params()[0].as_slice().split(|c| *c == b',').map(|v|
                    util::verify_receiver(v)
                ).collect()
            })
        } else {
            Err(RawMessage::new(cmd::REPLY(cmd::ERR_NEEDMOREPARAMS), [
                "*", message.command().to_string().as_slice(),
                "not enought params given"
            ], None))
        }
    }
    fn invoke(self, server: &mut Server, origin: SharedClient) {
        for &recv in self.receivers.iter() {
            match recv {
                util::ChannelName(ref name) => {
                    match server.channels.find_mut(&name.to_string()) {
                        Some(channel) => { 
                            let proxy = origin.borrow().proxy();
                            channel.send(channel::Handle(proc(channel) {
                                Names::handle_names(channel, &proxy)
                            }))
                        },
                        None => origin
                            .borrow_mut().send_response(cmd::ERR_NOSUCHCHANNEL,
                                Some(name.as_slice()), Some("No such channel"))
                    }
                },
                _ => {}
            }
            
        }
    }
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}