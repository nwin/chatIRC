use cmd;
use client;
use client::SharedClient;
use channel;
use channel::util::{AnonChannel, InviteOnly, Moderated, MemberOnly,
    Quiet, Private, Secret, ReOpFlag, TopicProtect, OperatorPrivilege,
    VoicePrivilege, ChannelKey, UserLimit, BanMask, ExceptionMask,
    InvitationMask, ChannelCreator
};
use channel::util::{Add, Remove, Show};
use msg::RawMessage;
use msg::util;

use server::{Server};


/// Handles the MODE command
pub struct Mode {
    raw: RawMessage,
    receiver: ::msg::Receiver,
    //params: Vec<Vec<u8>>
}
impl Mode {

    
    /// Handles the channel mode message
    pub fn handle_mode(channel: &mut channel::Channel, client_id: client::ClientId, message: RawMessage) {
        // TODO broadcast changes
        let is_op = { match channel.member_with_id(client_id) {
            Some(member) => member.is_op(),
            None => false
        }};
        let params = message.params();
        if params.len() > 1 {
            if !is_op { return } // TODO: error message
            channel::modes_do(params.slice_from(1), | action, mode, parameter | {
                match mode {
                    AnonChannel | InviteOnly | Moderated | MemberOnly 
                    | Quiet | Private | Secret | ReOpFlag | TopicProtect => {
                        match action {
                            Add => {channel.add_flag(mode);},
                            Remove => {channel.remove_flag(mode);},
                            Show => {} // ignore
                        }
                        
                    },
                    OperatorPrivilege | VoicePrivilege => {
                        match parameter { Some(name) => {
                                match channel.mut_member_with_nick(&name.to_string()) {
                                    Some(member) => match action {
                                        Add => member.promote(mode),
                                        Remove => member.demote(mode),
                                        Show => {}
                                    }, None => {}
                                }
                            }, None => {}
                        }
                    },
                    ChannelKey => match action {
                        Add => if parameter.is_some() {
                            channel.set_password(parameter.and_then(|v| Some(v.to_vec())))
                        },
                        Remove => channel.set_password(None),
                        Show => {} // this might not be a good idea
                    },
                    UserLimit => match action {
                        Add => match parameter.and_then(|v| from_str::<uint>(String::from_utf8_lossy(v).as_slice())) {
                            Some(limit) => channel.set_limit(Some(limit)),
                            _ => {}
                        },
                        Remove => channel.set_limit(None),
                        Show => {} // todo show
                    },
                    BanMask | ExceptionMask | InvitationMask => match action {
                        Show => {}, // TODO handle show
                        _ => match parameter { 
                            Some(mask) => {
                                let host_mask = util::HostMask::new(
                                    String::from_utf8_lossy(mask).to_string()
                                );
                                match mode {
                                    BanMask => match action {
                                        Add => {channel.add_ban_mask(host_mask);},
                                        Remove => {channel.remove_ban_mask(host_mask);},
                                        Show => {} // handled above
                                    },
                                    ExceptionMask => match action {
                                        Add => {channel.add_except_mask(host_mask);},
                                        Remove => {channel.remove_except_mask(host_mask);},
                                        Show => {} // handled above
                                    },
                                    InvitationMask => match action {
                                        Add => {channel.add_invite_mask(host_mask);},
                                        Remove => {channel.remove_invite_mask(host_mask);},
                                        Show => {} // handled above
                                    },
                                    _ => unreachable!()
                                }
                            },
                            None => {}
                        }
                    },
                    ChannelCreator => {
                        match action {
                            Add | Remove => {} // This is can't be set after channel creation 
                            Show => {} // TODO show
                        }
                    },
                }
            });
        } else {
            let member = match channel.member_with_id(client_id) {
                Some(member) => member,
                None => return // todo error message
            };
            member.send_response(cmd::RPL_CHANNELMODEIS,
                [channel.name(), 
                 ("+".to_string() + channel.flags()
                 ).as_slice()
                ]
            )
        }
    }
}
impl super::MessageHandler for Mode {
    fn from_message(message: RawMessage) -> Result<Box<Mode>, RawMessage> {
        let params = message.params();
        if params.len() > 0 {
            match util::verify_receiver(params[0]) {
                util::InvalidReceiver(name) => return Err(RawMessage::new(cmd::REPLY(cmd::ERR_USERNOTINCHANNEL), [
                    "*", message.command().to_string().as_slice(),
                    format!("invalid channel name {}", name).as_slice()
                    ], None)
                ),
                receiver => Ok(box Mode{
                    raw: message.clone(),
                    receiver: receiver,
                    //params: message.params().slice_from(1)
                    //               .iter().map(|v| v.to_vec()).collect()
                })
            }
        } else {
             return Err(RawMessage::new(cmd::REPLY(cmd::ERR_NEEDMOREPARAMS), [
                "*", message.command().to_string().as_slice(),
                "receiver given"
            ], None))
        }
    }
    fn invoke(self, server: &mut Server, origin: SharedClient) {
        let raw = self.raw;
        match self.receiver {
            util::ChannelName(name) => {
                match server.channels.find_mut(&name.to_string()) {
                    Some(channel) =>  {
                        let id = origin.borrow().id();
                        channel.send(channel::HandleMut(proc(channel) {
                            Mode::handle_mode(channel, id, raw)
                        }))
                    },
                    None => origin
                        .borrow_mut().send_response(cmd::ERR_NOSUCHCHANNEL,
                            Some(name.as_slice()), Some("No such channel"))
                    
                    
                }
            },
            _ => error!("user modes not supported yet")
        }
    }
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}