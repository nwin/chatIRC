use cmd;
use channel;
use channel::{Channel};
use channel::util::{MemberOnly, VoicePrivilege};
use msg::RawMessage;
use util;

use server::{Server};
use con::{Peer, PeerId};

/// handles PRIVMSG and NOTICE messages
#[allow(dead_code)]
pub struct Msg {
    raw: RawMessage,
    receiver: Vec<util::Receiver>,
    message: Vec<u8>
}
impl Msg {
    pub fn handle_msg(channel: &Channel, client_id: PeerId, message: RawMessage) {
        let maybe_member = channel.member_with_id(client_id);
        if channel.has_flag(MemberOnly) || channel.has_flag(VoicePrivilege) {
            match maybe_member {
                Some(sender) => {
                    if channel.has_flag(VoicePrivilege) && !sender.has_voice() {
                        return // TODO error message if not NOTICE
                    }
                    for member in channel.members() {
                        if member != sender {
                            member.send_msg(message.clone())
                        }
                    }
                },
                None => {
                    return // TODO error message if not NOTICE
                }
            }
        } else { // Message goes to everybody
            match maybe_member {
                Some(sender) => for member in channel.members() {
                    if member != sender {
                        member.send_msg(message.clone())
                    }
                },
                None => channel.broadcast(message)
            }
        }
    }
}
impl super::MessageHandler for Msg {
    fn from_message(message: RawMessage) -> Result<Box<Msg>, Option<RawMessage>> {
        let params = message.params();
        if params.len() > 1 {
            Ok(box Msg {
                raw: message.clone(), 
                receiver: params[0].as_slice()
                                   .split(|&v| v == b',' )
                                   .map(|v| util::verify_receiver(v))
                                   .collect(),
                message: params[1].to_vec()
            })
        } else {
            if message.command() != cmd::NOTICE {
                return Err(Some(RawMessage::new(cmd::REPLY(cmd::ERR_NEEDMOREPARAMS), [
                   "*", message.command().to_string().as_slice(),
                   "not enought params given"
                ], None)))
            } else { Err(None) }
        }
    }
    fn invoke(mut self, server: &mut Server, origin: Peer) {
        self.raw.set_prefix(origin.info().read().nick().as_slice());
        for receiver in self.receiver.into_iter() {
            match receiver {
                util::ChannelName(name) => match server.channels.find_mut(&name.to_string()) {
                    Some(channel) => {
                        let id = origin.id();
                        let message = self.raw.clone();
                        channel.send(channel::Handle(proc(channel) {
                            Msg::handle_msg(channel, id, message)
                        }))
                    },
                    None => {}
                },
                util::NickName(nick) => match server.find_peer(&nick.to_string()) {
                    Some(client) => {
                        client.send_msg(self.raw.clone());
                    },
                    None => {}
                },
                _ => {}
            }
        }
    }
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}