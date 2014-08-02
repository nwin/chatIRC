
use std::io::{TcpListener};
use std::io::{Listener, Acceptor};
use std::io::{IoResult};
use std::io::net;
use std::io;
use std::collections::hashmap::{HashMap};

use msg::{RawMessage};
use channel::{Member, Channel, ChannelEvent};
use channel;
use client::{SharedClient, Client, ClientId};
use msg::util::{ChannelName, NickName, verify_nick, verify_channel, verify_receiver};
use msg::util;
use msg;

use cmd::*;


/// Forwards the message to a channel
struct ChannelProxy {
    name: String,
    tx: Sender<ChannelEvent>,
    server_tx: Sender<Event>
}

impl ChannelProxy {
    fn new(name: String,
           tx: Sender<ChannelEvent>, 
           server_tx: Sender<Event>) -> ChannelProxy {
        ChannelProxy {
            name: name,
            tx: tx,
            server_tx: server_tx
        }
    }
    fn send(&self, event: ChannelEvent) {
        match self.tx.send_opt(event) {
            Ok(_) => {},
            Err(_) => {
                let _ = self.server_tx.send_opt(ChannelLost(self.name.clone()));
            }
        }
    }
}

pub struct IrcServer {
    host: String,
    ip: String,
    port: u16, 
    tx: Option<Sender<Event>>,
    // TODO put unregisterd clients in a staging Map
    clients: HashMap<ClientId, SharedClient>,
    nicknames: HashMap<String, SharedClient>,
    channels: HashMap<String, ChannelProxy>
}

/// Enumeration of the events the server can receive
pub enum Event {
    /// Message received from a client
    MessageReceived(ClientId, RawMessage),
    /// Connection to a client established
    ClientConnected(Client),
    /// The task of Channel(name) failed
    ChannelLost(String),
}

/// Convenience function to run the server
pub fn run_server(host: &str) -> IoResult<IrcServer> {
    let server = try!(IrcServer::new(host));
    server.serve_forever()
}

/// Irc server
impl IrcServer {
    /// Creates a new IRC server instance.
    pub fn new(host: &str) -> IoResult<IrcServer> {
        let addresses = try!(net::get_host_addresses(host));
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
            tx: None,
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
                ChannelLost(name) => {
                    self.channels.remove(&name);
                }
            }
        }
        Ok(self)
    }
    
    fn start_listening(&mut self) -> IoResult<Receiver<(Event)>>  {
        let listener = TcpListener::bind(self.ip.as_slice(), self.port);
        info!("started listening on {}:{} ({})", self.ip, self.port, self.host);
        let acceptor = try!(listener.listen());
        let (tx, rx) = channel();
        self.tx = Some(tx.clone());
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

    fn handle_privmsg(&mut self, origin: SharedClient, message: RawMessage) {
        let mut message = msg::PrivMessage::from_raw_message(message).unwrap();
        message.raw.set_prefix(origin.borrow().nickname.as_slice());
        for receiver in message.receiver.move_iter() {
            match receiver {
                ChannelName(name) => match self.channels.find_mut(&name.to_string()) {
                    Some(channel) => 
                        channel.send(channel::Message(
                            channel::PRIVMSG,
                            origin.borrow().id(),
                            message.raw.clone(),
                        )),
                    None => {}
                },
                NickName(nick) => match self.nicknames.find_mut(&nick.to_string()) {
                    Some(client) => {
                        client.borrow_mut().send_msg(message.raw.clone());
                    },
                    None => {}
                },
                _ => {}
            }
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
                        Some(channel) => {
                            channel.send(channel::Reply(
                                channel::NAMES,
                                origin.borrow().id(),
                                origin.borrow().proxy()
                            ))
                        } 
                        None => origin
                            .borrow_mut().send_response(ERR_NOSUCHCHANNEL,
                                Some(name.as_slice()), Some("No such channel"))
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
        let message = msg::UserMessage::from_raw_message(message).unwrap();
        origin.borrow_mut().username = message.username;
        origin.borrow_mut().realname = message.realname;
        let nick = origin.borrow_mut().nickname.clone();
        if self.nicknames.contains_key(&nick) {
            origin.borrow_mut().send_response(ERR_ALREADYREGISTRED, None,
                Some("somebody already registered with the same nickname")
            );
        } else {
            self.nicknames.insert(nick, origin.clone());
            self.send_welcome_msg(origin);
        }
    }
    
    /// Handles the QUIT command
    fn handle_quit(&mut self, origin: SharedClient, _: RawMessage) {
        // let message = msg::QuitMessage::from_raw_message(message).unwrap();
        // TODO communicate this to other users
        let mut client = origin.borrow_mut();
        client.close_connection();
        self.nicknames.remove(&client.nickname);
        self.clients.remove(&client.id());
    }
    
    /// Handles the MODE command
    fn handle_mode(&mut self, origin: SharedClient, message: RawMessage) {
        let message = msg::ModeMessage::from_raw_message(message).unwrap();
        match message.receiver {
            ChannelName(ref name) => {
                match self.channels.find_mut(&name.to_string()) {
                    Some(channel) => channel.send(channel::Message(
                            channel::MODE,
                            origin.borrow().id(), 
                            message.raw
                    )),
                    None => origin
                        .borrow_mut().send_response(ERR_NOSUCHCHANNEL,
                            Some(name.as_slice()), Some("No such channel"))
                        
                        
                }
            },
            _ => error!("user modes not supported yet")
        }
    }
    
    /// Handles the JOIN command
    ///    Command: JOIN
    /// Parameters: <channel>{,<channel>} [<key>{,<key>}]
    fn handle_join(&mut self, origin: SharedClient, message: RawMessage) {
        let message = msg::JoinMessage::from_raw_message(message).unwrap();
        let host = self.host.clone();
        for (channel, password) in message.targets.move_iter()
        .zip(message.passwords.move_iter()) {
            let tx = self.tx.clone().unwrap();
            self.channels.find_or_insert_with(channel.to_string(), |key| {
                ChannelProxy::new(
                    key.clone(),
                    Channel::new(key.clone(), host.clone()).listen(),
                    // this should exist by now
                    tx.clone()
                )
            }).send(
                channel::Join(
                    Member::new(
                        origin.borrow().id(),
                        origin.borrow().nickname.clone(),
                        util::HostMask::new(origin.borrow().real_mask()),
                        self.host.clone(),
                        origin.borrow().proxy()
                    ),
                    password
                )
            )
        }
    }
}