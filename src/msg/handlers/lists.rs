use cmd;
use channel;
use channel::{Channel};
use channel::util::{Secret, Private};
use msg::RawMessage;
use util;

use server::{Server};
use con::{Peer};
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
    pub fn handle_who(&self, channel: &Channel, client: Peer) {
        if (channel.has_flag(Private) || channel.has_flag(Secret))
        && !channel.member_with_id(client.id()).is_some() {
            // Don't give information about this channel to the outside
            // this should also be ok for secret because RPL_ENDOFWHO is
            // always sent.
            channel.send_response(&client, cmd::RPL_ENDOFWHO, [
                self.mask.as_slice(), "End of WHO list"
            ]);
        } else {
            let sender = channel.list_sender(&client, cmd::RPL_WHOREPLY, cmd::RPL_ENDOFWHO);
            for member in channel.members() {
                if !self.op_only || member.is_op() {
                    sender.feed_line(&[
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
            sender.end_of_list()
        }
    }
}
impl super::MessageHandler for Who {
    fn from_message(message: RawMessage) -> Result<Box<Who>, Option<RawMessage>> {
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
    fn invoke(self, server: &mut Server, origin: Peer) {
        match server.channels.find(&self.mask) {
            Some(channel) => {
                channel.send(channel::Handle(proc(channel) {
                    self.handle_who(channel, origin)
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
    pub fn handle_names(channel: &Channel, proxy: &Peer) {
        // TODO check if channel is visible to user…
        // TODO replace with generic list sending function
        for member in channel.members() {
            let mut tmp = String::from_str("= ");
            tmp.push_str(channel.name());
            channel.send_response(proxy, cmd::RPL_NAMREPLY, [
                tmp.as_slice(),
                member.decorated_nick()   
            ])
        }
        channel.send_response(proxy, cmd::RPL_ENDOFNAMES, 
            [channel.name(), "End of /NAMES list"])
    }
}
impl super::MessageHandler for Names {
    fn from_message(message: RawMessage) -> Result<Box<Names>, Option<RawMessage>> {
        if message.params().len() > 0 {
            Ok(box Names {
                raw: message.clone(),
                receivers: message.params()[0].as_slice().split(|c| *c == b',').map(|v|
                    util::verify_receiver(v)
                ).collect()
            })
        } else {
            Err(Some(RawMessage::new(cmd::REPLY(cmd::ERR_NEEDMOREPARAMS), [
                "*", message.command().to_string().as_slice(),
                "not enought params given"
            ], None)))
        }
    }
    fn invoke(self, server: &mut Server, origin: Peer) {
        let host = server.host().to_string(); // clone due to #6393
        for recv in self.receivers.iter() {
            match recv {
                &util::ChannelName(ref name) => {
                    match server.channels.find_mut(&name.to_string()) {
                        Some(channel) => { 
                            let proxy = origin.clone();
                            channel.send(channel::Handle(proc(channel) {
                                Names::handle_names(channel, &proxy)
                            }))
                        },
                        None => origin.send_response(cmd::ERR_NOSUCHCHANNEL,
                            &[name.as_slice(), "No such channel"],
                            host.as_slice()
                        )
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