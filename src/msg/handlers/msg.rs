use cmd;
use channel;
use channel::{Channel};
use channel::util::{MemberOnly, VoicePrivilege};
use msg::RawMessage;
use util;

use server::{Server};
use client::{SharedClient, ClientId};

/// handles private messages
#[allow(dead_code)]
pub struct Privmsg {
    raw: RawMessage,
    receiver: Vec<util::Receiver>,
    message: Vec<u8>
}
impl Privmsg {
    
    pub fn handle_privmsg(channel: &Channel, client_id: ClientId, message: RawMessage) {
        let maybe_member = channel.member_with_id(client_id);
        if channel.has_flag(MemberOnly) || channel.has_flag(VoicePrivilege) {
            match maybe_member {
                Some(sender) => {
                    if channel.has_flag(VoicePrivilege) && !sender.has_voice() {
                        return // TODO error message
                    }
                    for member in channel.members() {
                        if member != sender {
                            member.send_msg(message.clone())
                        }
                    }
                },
                None => {
                    return // TODO error message
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
impl super::MessageHandler for Privmsg {
    fn from_message(message: RawMessage) -> Result<Box<Privmsg>, RawMessage> {
        let params = message.params();
        if params.len() > 1 {
            Ok(box Privmsg {
                raw: message.clone(), 
                receiver: params[0].as_slice()
                                   .split(|&v| v == b',' )
                                   .map(|v| util::verify_receiver(v))
                                   .collect(),
                message: params[1].to_vec()
            })
        } else {
             return Err(RawMessage::new(cmd::REPLY(cmd::ERR_NEEDMOREPARAMS), [
                "*", message.command().to_string().as_slice(),
                "not enought params given"
            ], None))
        }
    }
    fn invoke(mut self, server: &mut Server, origin: SharedClient) {
        self.raw.set_prefix(origin.borrow().nickname.as_slice());
        for receiver in self.receiver.move_iter() {
            match receiver {
                util::ChannelName(name) => match server.channels.find_mut(&name.to_string()) {
                    Some(channel) => {
                        let id = origin.borrow().id();
                        let message = self.raw.clone();
                        channel.send(channel::Handle(proc(channel) {
                            Privmsg::handle_privmsg(channel, id, message)
                        }))
                    },
                    None => {}
                },
                util::NickName(nick) => match server.registered.find_mut(&nick.to_string()) {
                    Some(client) => {
                        client.borrow_mut().send_msg(self.raw.clone());
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