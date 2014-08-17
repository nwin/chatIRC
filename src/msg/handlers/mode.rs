use cmd;
use con::Peer;
use channel;
use channel::util::{AnonChannel, InviteOnly, Moderated, MemberOnly,
    Quiet, Private, Secret, ReOpFlag, TopicProtect, OperatorPrivilege,
    VoicePrivilege, ChannelKey, UserLimit, BanMask, ExceptionMask,
    InvitationMask, ChannelCreator
};
use channel::util::{ChannelMode, Action, Add, Remove, Show};
use msg::RawMessage;
use util;

use server::{Server};


/// Handles the MODE command
pub struct Mode {
    raw: RawMessage,
    receiver: util::Receiver,
    //params: Vec<Vec<u8>>
}
impl Mode {
    
    pub fn broadcast_change(channel: &channel::Channel, nick: &str, action: Action,
                            flag: ChannelMode, param: Option<&str>) {
        let flag_str = match action {
            Add => "+",
            Remove => "-",
            Show => ""
        }.to_string() + (flag as u8 as char).to_string();
        channel.broadcast(RawMessage::new(cmd::MODE,
            match param {
                Some(param) => vec![channel.name(), flag_str.as_slice(), param],
                None => vec![channel.name(), flag_str.as_slice()]
            }.as_slice(),
            Some(nick.as_slice())
        ))
    }
    
    /// Handles the channel mode message
    pub fn handle_mode(channel: &mut channel::Channel, proxy: Peer, message: RawMessage) {
        // TODO broadcast changes
        // TODO send ERR_UNKNOWNMODE
        let is_op = { match channel.member_with_id(proxy.id()) {
            Some(member) => member.is_op(),
            None => false
        }};
        let peer_nick: String = proxy.info().read().nick().clone();
        let params = message.params();
        if params.len() > 1 {
            if !is_op { 
                proxy.send_response(cmd::ERR_CHANOPRIVSNEEDED,
                    [channel.name(), "You are not a channel operator"], 
                    peer_nick.as_slice().as_slice()
                );
                return 
            }
            channel::modes_do(params.slice_from(1), | action, mode, parameter | {
                match mode {
                    AnonChannel | InviteOnly | Moderated | MemberOnly 
                    | Quiet | Private | Secret | ReOpFlag | TopicProtect => {
                        match action {
                            Add => {
                                channel.add_flag(mode);
                                Mode::broadcast_change(channel, peer_nick.as_slice(), action, mode, None)
                            },
                            Remove => {
                                channel.remove_flag(mode);
                                Mode::broadcast_change(channel, peer_nick.as_slice(), action, mode, None)
                            },
                            Show => {} // ignore
                        }
                        
                    },
                    OperatorPrivilege | VoicePrivilege => {
                        match parameter { Some(name) => {
                            let nick = match channel.mut_member_with_nick(&String::from_utf8_lossy(name).to_string()) {
                                Some(member) => match action {
                                    Add => {
                                        member.promote(mode);
                                        Some(member.nick().to_string())
                                    },
                                    Remove => {
                                        member.demote(mode);
                                        Some(member.nick().to_string())
                                    },
                                    Show => None // make not much sense
                                }, None => None
                            };
                            match nick {
                                Some(nick) => Mode::broadcast_change(
                                    channel, peer_nick.as_slice(), action, mode, Some(nick.as_slice())
                                ),
                                None => {}
                            }
                        }, None => {}}
                    },
                    ChannelKey => match action {
                        Add => if parameter.is_some() {
                            channel.set_password(parameter.and_then(|v| Some(v.to_vec())));
                            Mode::broadcast_change(channel, peer_nick.as_slice(), action, mode, None)
                        },
                        Remove => {
                            channel.set_password(None);
                            Mode::broadcast_change(channel, peer_nick.as_slice(), action, mode, None)
                        },
                        Show => {} // this might not be a good idea
                    },
                    UserLimit => match action {
                        Add => match parameter.and_then(|v| from_str::<uint>(String::from_utf8_lossy(v).as_slice())) {
                            Some(limit) => {
                                channel.set_limit(Some(limit));
                                Mode::broadcast_change(
                                    channel, peer_nick.as_slice(), action, mode, 
                                    Some(limit.to_string().as_slice())
                                )
                            },
                            _ => {}
                        },
                        Remove => {
                            channel.set_limit(None);
                            Mode::broadcast_change(channel, peer_nick.as_slice(), action, mode, None)
                        },
                        Show => {} // todo show
                    },
                    BanMask | ExceptionMask | InvitationMask => match parameter { 
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
                        None => {
                            let (start_code, end_code, masks) = match mode {
                                BanMask => (
                                    cmd::RPL_BANLIST,
                                    cmd::RPL_ENDOFBANLIST,
                                    channel.ban_masks()
                                ),
                                ExceptionMask => (
                                    cmd::RPL_EXCEPTLIST,
                                    cmd::RPL_ENDOFEXCEPTLIST,
                                    channel.except_masks()
                                ),
                                InvitationMask => (
                                    cmd::RPL_INVITELIST,
                                    cmd::RPL_ENDOFINVITELIST,
                                    channel.invite_masks()
                                ),
                                _ => unreachable!()
                            };
                            let sender = channel.list_sender(
                                &proxy, start_code, end_code
                            );
                            for mask in masks.iter() {
                                sender.feed_line(&[mask.as_str()])
                            }
                            sender.end_of_list()
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
            // TODO secret channel??
            // TODO things with parameters?
            proxy.send_response(cmd::RPL_CHANNELMODEIS,
                [channel.name(), ("+".to_string() + channel.flags()).as_slice()],
                channel.server_name()
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
    fn invoke(self, server: &mut Server, origin: Peer) {
        let host = server.host().to_string(); // clone due to #6393
        let raw = self.raw;
        match self.receiver {
            util::ChannelName(name) => {
                match server.channels.find_mut(&name.to_string()) {
                    Some(channel) =>  {
                        channel.send(channel::HandleMut(proc(channel) {
                            Mode::handle_mode(channel, origin, raw)
                        }))
                    },
                    None => origin.send_response(cmd::ERR_NOSUCHCHANNEL,
                            &[name.as_slice(), "No such channel"],
                            host.as_slice()
                    )
                }
            },
            _ => error!("user modes not supported yet")
        }
    }
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}