use cmd;
use channel;
use channel::util::{InviteOnly, ChannelCreator, OperatorPrivilege, TopicProtect, MemberOnly, UserLimit};
use msg::RawMessage;
use util;

use server::{Server};
use con::Peer;

/// Handles the JOIN command.
///
///    Command: JOIN
/// Parameters: <channel>{,<channel>} [<key>{,<key>}]
pub struct Join {
    raw: RawMessage,
    targets: Vec<String>,
    passwords: Vec<Option<Vec<u8>>>,
}

impl Join {
    fn handle_join(channel: &mut channel::Channel, mut member: channel::Member, password: Option<Vec<u8>>) {
        match channel.password() {
            &Some(ref chan_pass) => if !match password { 
                    Some(password) => &password == chan_pass,
                    None => false } {
                member.send_response(cmd::ERR_BADCHANNELKEY,
                    [channel.name(),
                    "Cannot join channel (+k)"]
                );
                return
            },
            &None => {},
        }
        if channel.member_with_id(member.id()).is_some() {
            // Member already in channel
            return
        }
        if member.mask_matches_any(channel.ban_masks()) 
           && !member.mask_matches_any(channel.except_masks()) {
            // Member banned
            channel.send_response(
                member.proxy(), 
                cmd::ERR_BANNEDFROMCHAN, 
                &["Cannot join channel (+b)"]
            );
            return
        }
        if channel.has_flag(InviteOnly) 
           && !member.mask_matches_any(channel.invite_masks()) {
            // Member not invited
            channel.send_response(
                member.proxy(), 
                cmd::ERR_INVITEONLYCHAN, 
                &["Cannot join channel (+i)"]
            );
            return
        }
        if channel.has_flag(UserLimit)
           && channel.limit().map_or(false, |limit| channel.member_count() + 1 >= limit) {
            // User limit reached
            channel.send_response(
                member.proxy(), 
                cmd::ERR_CHANNELISFULL, 
                &["Cannot join channel (+l)"]
            );
            return
        }
        // Give op to first user
        if channel.member_count() == 0 {
            member.promote(ChannelCreator);
            member.promote(OperatorPrivilege);
        }
        
        // Broadcast that a new member joined the channel and add him
        let msg = RawMessage::new(
            cmd::JOIN, 
            &[channel.name()],
            Some(member.nick())
        );
        let id = member.id().clone();
        let _ = channel.add_member(member);
        channel.broadcast(msg);
        
        // Topic reply
        let member = channel.member_with_id(id).unwrap();
        member.send_response(cmd::RPL_NOTOPIC, 
            [channel.name(), "No topic set."]
        );
        // Send name list as per RFC
        super::lists::Names::handle_names(channel, member.proxy());
    }
}

impl super::MessageHandler for Join {
    fn from_message(message: RawMessage) -> Result<Box<Join>, RawMessage> { 
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
                            passwords.push(Some(channels_passwords[i].to_vec()));
                        } else {
                            passwords.push(None);
                        }
                    },
                    None => return Err(RawMessage::new(cmd::REPLY(cmd::ERR_NOSUCHCHANNEL), [
                        "*", String::from_utf8_lossy(channel_name).as_slice(),
                        "Invalid channel name."
                    ], None))
                }
            }
        } else {
             return Err(RawMessage::new(cmd::REPLY(cmd::ERR_NEEDMOREPARAMS), [
                "*", message.command().to_string().as_slice(),
                "no params given"
            ], None))
        }
        Ok(box Join {
            raw: message.clone(), targets: targets, passwords: passwords
        })
    }
    
    fn invoke(self, server: &mut Server, origin: Peer) {
        let host = server.host().to_string(); // clone due to #6393
        for (channel, password) in self.targets.move_iter()
                                   .zip(self.passwords.move_iter()) {
            let member = channel::Member::new(origin.clone());
            let tx = server.tx().unwrap(); // save to unwrap, this should exist by now
            server.channels.find_or_insert_with(channel.to_string(), |name| {
                let mut channel = channel::Channel::new(name.clone(), host.clone());
                channel.add_flag(TopicProtect);
                channel.add_flag(MemberOnly);
                channel.listen(tx.clone())
            }).send(
                channel::HandleMut(proc(channel) {
                    Join::handle_join(channel, member, password)
                })
            )
        }
    }
    
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}