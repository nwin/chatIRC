use cmd;
use channel;
use channel::util::{ChannelCreator, OperatorPrivilege};
use msg::RawMessage;
use msg::util;

use server::{Server};
use client::SharedClient;

/// Handles the JOIN command
///
///    Command: JOIN
/// Parameters: <channel>{,<channel>} [<key>{,<key>}]
pub struct Join {
    raw: RawMessage,
    targets: Vec<String>,
    passwords: Vec<Option<Vec<u8>>>,
}

impl Join {
    fn handle_join(this: &mut channel::Channel, mut member: channel::Member, password: Option<Vec<u8>>) {
        if this.password.len() != 0 {
            if !match password { Some(password) => password == this.password,
                                 None => false } {
                member.send_response(cmd::ERR_BADCHANNELKEY,
                    [this.name(),
                    "Password is wrong"]
                );
                return
            }
        }
        if this.member_with_id(member.id()).is_some() {
            //member already in channel
            return
        }
        let msg = RawMessage::new(
            cmd::JOIN, 
            &[this.name()],
            Some(member.nick())
        );
        if this.member_count() == 0 { // first user
            member.promote(ChannelCreator);
            member.promote(OperatorPrivilege);
        }
        let id = member.id().clone();
        let _ = this.add_member(member);
        this.broadcast(msg);
        let member = this.member_with_id(id).unwrap();
        member.send_response(cmd::RPL_NOTOPIC, 
            [this.name(), "No topic set."]
        );
        super::lists::Names::handle_names(this, member.proxy());
        //this.handle_names(member.proxy());
        if this.member_count() == 1 { // first user
            let msg = RawMessage::new(cmd::MODE, [
                this.name(),
                format!("+{}", member.decoration()).as_slice(), 
                member.nick()], Some(this.server_name()));
            this.broadcast(msg)
        }
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
    
    fn invoke(self, server: &mut Server, origin: SharedClient) {
        let host = server.host().to_string();
        for (channel, password) in self.targets.move_iter()
                                   .zip(self.passwords.move_iter()) {
            let member = channel::Member::new(
                origin.borrow().id(),
                origin.borrow().realname.clone(),
                util::HostMask::new(origin.borrow().real_mask()),
                host.clone(),
                origin.borrow().proxy()
            );
            let tx = server.tx().unwrap();
            server.channels.find_or_insert_with(channel.to_string(), |key| {
                channel::Proxy::new(
                    key.clone(),
                    channel::Channel::new(key.clone(), host.clone()).listen(),
                    // this should exist by now
                    tx.clone()
                )
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