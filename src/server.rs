
use std::io::{TcpListener};
use std::io::{Listener, Acceptor};
use std::io::{IoResult};
use std::io::net;
use std::io;
use std::collections::hashmap::{HashMap};

use message::{RawMessage};
use channel::{Member, Channel};
use client::{SharedClient, Client, ClientId};
use util::{ChannelName, NickName, verify_nick, verify_channel, verify_receiver};

use cmd::*;

pub struct IrcServer {
    host: String,
    ip: String,
    port: u16, 
    clients: HashMap<ClientId, SharedClient>,
    nicknames: HashMap<String, SharedClient>,
    channels: HashMap<String, Channel>
}

pub enum Event {
    MessageReceived(ClientId, RawMessage),
    ClientConnected(Client),
}

pub fn run_server(host: &str) -> IoResult<IrcServer> {
    let server = try!(IrcServer::new(host));
    server.serve_forever()
}

impl IrcServer {
    /// Creates a new IRC server instance.
    pub fn new(host: &str) -> IoResult<IrcServer> {
        let mut addresses = try!(net::get_host_addresses(host));
        debug!("{}", addresses)
        let ip = match addresses.as_slice().get(0) {
            Some(ip) => ip,
            None => return Err(io::IoError {
                kind: io::OtherIoError,
                desc: "cannnot resolve ip",
                detail: None
            })
        };
        Ok(IrcServer {
            host: host.to_string(),
            ip: format!("{}", ip),
            port: 6667,
            clients: HashMap::new(),
            nicknames: HashMap::new(),
            channels: HashMap::new()
        })
    }
    
    /// Starts the main loop and listens on the specified host and port.
    pub fn serve_forever(mut self) -> IoResult<IrcServer> {
        // todo change this to a more general event dispatching loop
        let message_rx = try!(self.start_listening());
        for event in message_rx.iter() {
            match event {
                MessageReceived(client_id, message) => {
                    let client = match self.clients.find(&client_id) {
                        Some(client) => Some(client.clone()),
                        None => None
                    };
                    match client {
                        Some(client) => self.dispatch(client, message),
                        None => error!(
                            "Client {} not found when sending message {}.",
                            client_id,
                            message.command()
                        )
                    }
                },
                ClientConnected(client) => { 
                    let client = client.as_shared();
                    self.clients.insert(client.borrow().id(), client); 
                }
            }
        }
        Ok(self)
    }
    
    fn start_listening(&mut self) -> IoResult<Receiver<(Event)>>  {
        let listener = TcpListener::bind(self.ip.as_slice(), self.port);
        debug!("started listening on {}:{} ({})", self.ip, self.port, self.host);
        let acceptor = try!(listener.listen());
        let (tx, rx) = channel();
        let host = self.host.clone();
        spawn(proc() {
            let mut a = acceptor; // https://github.com/rust-lang/rust/issues/11958
            for maybe_stream in a.incoming() {
                match maybe_stream {
                    Err(err) => { error!("{}", err) }
                    Ok(stream) => {
                        match Client::listen(host.clone(), stream, tx.clone()) {
                            Ok(()) => {
                            },
                            Err(err) => {
                                error!("{}", err)
                            }
                        }
                    }
                }
            }
        });
        Ok(rx)
    }
    
    /// Main message dispatcher
    ///
    /// This method processes all messages comming from any client. Be carefull
    /// to keep the processing time of each message as short as possible to
    /// archive bester server performance. Should spawn new threads if the processing
    /// take more time.
    fn dispatch(&mut self, origin: SharedClient, message: RawMessage) {
        // TODO: wrap this in a proc?
        match message.command() {
            PRIVMSG => self.handle_privmsg(origin, message),
            MODE => self.handle_mode(origin, message),
            // ignoring PONG, this is basically handled
            // by the socket timeout
            PONG => {},
            JOIN => self.handle_join(origin, message),
            NAMES => self.handle_names(origin, message),
            NICK => self.handle_nick(origin, message),
            USER => self.handle_user(origin, message),
            PING => {}, // ignoring this message, I am a server
            QUIT => self.handle_quit(origin, message),
            REPLY(_) => {}, // should not come from a client, ignore
            UNKNOWN(_) => 
                error!(
                    "Handling of message {} not implemented yet.",
                    message.command().to_string())
        }
    }
    
    /// Sends a welcome message to a newly registered client
    fn send_welcome_msg(&self, client: SharedClient) {
        client.borrow_mut().send_response(RPL_WELCOME, None, None)
    }

    fn handle_privmsg(&mut self, origin: SharedClient, mut message: RawMessage) {
        message.set_prefix(origin.borrow().nickname.as_slice());
        let params = message.params();
        if params.len() > 1 {
            for receiver in params[0].as_slice().split(|&v| v == b',' )
                                     .map(|v| verify_receiver(v)) {
                match receiver {
                    ChannelName(name) => match self.channels.find_mut(&name.to_string()) {
                        Some(channel) => 
                            channel.handle_privmsg(origin.borrow().id(), message.clone()),
                        None => {}
                    },
                    NickName(nick) => match self.nicknames.find_mut(&nick.to_string()) {
                        Some(client) => {
                            client.borrow_mut().send_msg(message.clone());
                        },
                        None => {}
                    },
                    _ => {}
                }
            }
        } else {
            origin.borrow_mut().send_response(ERR_NEEDMOREPARAMS,
                Some(message.command().to_string().as_slice()),
                Some("not enought params given")
            )
        }
    }
    
    /// Handles the nick command
    ///    Command: NICK
    /// Parameters: <nickname> [ <hopcount> ]
    fn handle_nick(&self, origin: SharedClient, message: RawMessage) {
        let mut client = origin.borrow_mut();
        let params = message.params();
        if params.len() > 0 {
            match verify_nick(params[0].as_slice()) {
                Some(nick) => {
                    let nick = nick.to_string();
                    if self.nicknames.contains_key(&nick) {
                        client.send_response(
                            ERR_NICKNAMEINUSE,
                            Some(nick.as_slice()), 
                            Some("nickname in use")
                        );
                    } else {
                        client.nickname = nick;
                    }
                },
                None => {
                    client.send_response(
                        ERR_ERRONEUSNICKNAME,
                        Some(String::from_utf8_lossy(params[0].as_slice()).as_slice()),
                        Some("invalid nick name")
                    );
                }
            }
        } else {
            client.send_response(ERR_NONICKNAMEGIVEN, None,
                Some("no nickname given")
            )
        }
    }
    
    /// Handles the NAMES command
    fn handle_names(&mut self, origin: SharedClient, message: RawMessage) {
        message.receivers_do_or_else( | recv | {
            match recv {
                ChannelName(ref name) => {
                    match self.channels.find_mut(&name.to_string()) {
                        Some(channel) => channel.handle_names(
                            origin.borrow().id(),
                            | m | origin.borrow_mut().send_msg(m)
                        ),
                        None => origin
                            .borrow_mut().send_response(ERR_NOSUCHCHANNEL,
                                Some(*name), Some("No such channel"))
                    }
                },
                _ => {}
            }
        }, || origin.borrow_mut().send_response(ERR_NEEDMOREPARAMS,
            Some(message.command().to_string().as_slice()),
            Some("not enought params given")
        )
        )
    }
    
    /// Handles the USER command
    fn handle_user(&mut self, origin: SharedClient, message: RawMessage) {
        let params = message.params();
        if params.len() >= 4 {
            let username = String::from_utf8_lossy(params[0].as_slice());
            let realname = String::from_utf8_lossy(params[3].as_slice());
            let nick = {
                origin.borrow_mut().username = username.into_string();
                origin.borrow_mut().realname = realname.into_string();
                origin.borrow_mut().nickname.clone()
            };
            if self.nicknames.contains_key(&nick) {
                origin.borrow_mut().send_response(ERR_ALREADYREGISTRED, None,
                    Some("somebody already registered with the same nickname")
                );
            } else {
                self.nicknames.insert(nick, origin.clone());
                self.send_welcome_msg(origin);
            }
        } else {
            origin.borrow_mut().send_response(ERR_NEEDMOREPARAMS,
                Some(message.command().to_string().as_slice()),
                Some("not enought params given")
            )
        }
    }
    
    /// Handles the QUIT command
    fn handle_quit(&mut self, origin: SharedClient, _: RawMessage) {
        // TODO communicate this to other users
        let mut client = origin.borrow_mut();
        client.close_connection();
        self.nicknames.remove(&client.nickname);
        self.clients.remove(&client.id());
    }
    
    /// Handles the MODE command
    fn handle_mode(&mut self, origin: SharedClient, message: RawMessage) {
        let params = message.params();
        if params.len() > 0 {
            match verify_receiver(params[0]) {
                ChannelName(ref name) => {
                    match self.channels.find_mut(&name.to_string()) {
                        Some(channel) => channel.handle_mode(origin.borrow().id(), message.clone()),
                        None => origin
                            .borrow_mut().send_response(ERR_NOSUCHCHANNEL,
                                Some(*name), Some("No such channel"))
                            
                            
                    }
                },
                _ => error!("user modes not supported yet")
            }
        } else {
            origin.borrow_mut().send_response(ERR_NEEDMOREPARAMS,
                Some(message.command().to_string().as_slice()),
                Some("no receiver given")
            )
        }
    }
    
    /// Handles the JOIN command
    ///    Command: JOIN
    /// Parameters: <channel>{,<channel>} [<key>{,<key>}]
    fn handle_join(&mut self, origin: SharedClient, message: RawMessage) {
        let params = message.params();
        let host = self.host.clone();
        if params.len() > 0 {
            let passwords: Vec<&[u8]> = if params.len() > 1 {
                params[1].as_slice().split(|c| *c == b',').collect()
            } else {
                Vec::new()
            };
            for (i, channel_name) in params[0].as_slice().split(|c| *c == b',').enumerate() {
                match verify_channel(channel_name) {
                    Some(channel) => {
                        self.channels.find_or_insert_with(channel.to_string(), |key| {
                            Channel::new(key.clone(), host.clone())
                        }).handle_join(
                            Member::new(
                                origin.borrow().id(),
                                origin.borrow().nickname.clone(),
                                self.host.clone(),
                                origin.borrow().tx()
                            ), 
                            passwords.as_slice().get(i).map(|v| v.to_owned())
                        )
                    },
                    None => origin.borrow_mut().send_response(ERR_NOSUCHCHANNEL,
                        Some(String::from_utf8_lossy(channel_name.as_slice()).as_slice()),
                        Some("Invalid channel name.")
                    )
                }
            }
        } else {
            origin.borrow_mut().send_response(ERR_NEEDMOREPARAMS,
                Some(message.command().to_string().as_slice()),
                Some("no params given")
            )
        }
    }
}