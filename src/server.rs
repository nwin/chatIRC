
use std::io::{TcpListener};
use std::io::{Listener, Acceptor};
use std::io::{IoResult};
use std::io::net;
use std::io;
use std::collections::hashmap::{HashMap};

use channel::{ChannelEvent};
use channel;
use client::{SharedClient, Client, ClientId};
use msg::util::{ChannelName, NickName};
use msg::util;
use msg::{MessageHandler};
use msg;

use cmd::*;


/// Forwards the message to a channel
pub struct ChannelProxy {
    name: String,
    tx: Sender<ChannelEvent>,
    server_tx: Sender<Event>
}

impl ChannelProxy {
    pub fn new(name: String,
           tx: Sender<ChannelEvent>, 
           server_tx: Sender<Event>) -> ChannelProxy {
        ChannelProxy {
            name: name,
            tx: tx,
            server_tx: server_tx
        }
    }
    pub fn send(&self, event: ChannelEvent) {
        match self.tx.send_opt(event) {
            Ok(_) => {},
            Err(_) => {
                let _ = self.server_tx.send_opt(ChannelLost(self.name.clone()));
            }
        }
    }
}

pub struct Server {
    host: String,
    ip: String,
    port: u16, 
    tx: Option<Sender<Event>>,
    // TODO put unregisterd clients in a staging Map
    clients: HashMap<ClientId, SharedClient>,
    nicknames: HashMap<String, SharedClient>,
    pub channels: HashMap<String, ChannelProxy>
}

/// Enumeration of the events the server can receive
pub enum Event {
    /// Message received from a client
    MessageReceived(ClientId, Box<MessageHandler + Send>),
    /// Connection to a client established
    ClientConnected(Client),
    /// The task of Channel(name) failed
    ChannelLost(String),
}

/// Convenience function to run the server
pub fn run_server(host: &str) -> IoResult<Server> {
    let server = try!(Server::new(host));
    server.serve_forever()
}

/// Irc server
impl Server {
    /// Creates a new IRC server instance.
    pub fn new(host: &str) -> IoResult<Server> {
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
        Ok(Server {
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
    pub fn serve_forever(mut self) -> IoResult<Server> {
        // todo change this to a more general event dispatching loop
        let message_rx = try!(self.start_listening());
        for event in message_rx.iter() {
            match event {
                MessageReceived(client_id, handler) => {
                    let client = match self.clients.find(&client_id) {
                        Some(client) => Some(client.clone()),
                        None => None
                    };
                    match client {
                        Some(client) => handler.invoke(&mut self, client),
                        None => error!(
                            "Client {} not found when sending message.",
                            client_id
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
    //fn dispatch(&mut self, origin: SharedClient, message: Message) {
    //    // TODO: wrap this in a proc?
    //    match message {
    //        msg::Priv(msg) => self.handle_privmsg(origin, msg),
    //        msg::Mode(msg) => self.handle_mode(origin, msg),
    //        // ignoring PONG, this is basically handled
    //        // by the socket timeout
    //        msg::Pong(_) => {},
    //        msg::Join(msg) => self.handle_join(origin, msg),
    //        msg::Part(msg) => self.handle_part(origin, msg),
    //        msg::Who(msg) => self.handle_who(origin, msg),
    //        msg::Names(msg) => self.handle_names(origin, msg),
    //        msg::Nick(msg) => self.handle_nick(origin, msg),
    //        msg::User(msg) => self.handle_user(origin, msg),
    //        msg::Ping(_) => {}, // ignoring this message, I am a server
    //        msg::Quit(msg) => self.handle_quit(origin, msg),
    //        msg::Reply(_) => {}, // should not come from a client, ignore
    //        msg::Unknown(cmd) => 
    //            error!(
    //                "Handling of message {} not implemented yet.",
    //                String::from_utf8_lossy(cmd.as_slice()))
    //    }
    //}
    
    /// Getter for hostname
    pub fn host(&self) -> &str {
        self.host.as_slice()
    }
    
    /// Getter for hostname
    pub fn tx(&self) -> Option<Sender<Event>> {
        self.tx.clone()
    }
    
    
    /// Sends a welcome message to a newly registered client
    fn send_welcome_msg(&self, client: &SharedClient) {
        client.borrow().send_response(RPL_WELCOME, None, None)
    }

    fn handle_privmsg(&mut self, origin: SharedClient, mut message: msg::PrivMessage) {
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
    
    /// Handles the NAMES command
    fn handle_names(&mut self, origin: SharedClient, message: msg::NamesMessage) {
        for &recv in message.receivers.iter() {
            match recv {
                ChannelName(ref name) => {
                    match self.channels.find_mut(&name.to_string()) {
                        Some(channel) => {
                            channel.send(channel::Reply(
                                channel::NAMES,
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
            
        }
    }
    
    /// Handles the NAMES command
    fn handle_who(&mut self, origin: SharedClient, message: msg::WhoMessage) {
        match self.channels.find(&message.mask) {
            Some(channel) => channel.send(channel::Who(
                origin.borrow().proxy(),
                message,
            )),
            None => {} // handle later
        }
    }
    
    /// Handles the nick command
    ///    Command: NICK
    /// Parameters: <nickname> [ <hopcount> ]
    fn handle_nick(&mut self, origin: SharedClient, message: msg::NickMessage) {
        if self.nicknames.contains_key(&message.nick) {
            origin.borrow().send_response(
                ERR_NICKNAMEINUSE,
                Some(message.nick.as_slice()), 
                Some("nickname in use")
            );
        } else {
            origin.borrow_mut().nickname = message.nick;
        }
        if origin.borrow().username.len() > 0 && !self.nicknames.contains_key(&origin.borrow().nickname){
            // user message already send but not yet registered
            self.try_register(&origin)
        }
    }
    
    /// Handles the USER command
    fn handle_user(&mut self, origin: SharedClient, message: msg::UserMessage) {
        origin.borrow_mut().username = message.username;
        origin.borrow_mut().realname = message.realname;
        self.try_register(&origin)
    }
    
    fn try_register(&mut self, origin: &SharedClient) {
        let nick = origin.borrow().nickname.clone();
        if nick.len() > 0 && self.nicknames.contains_key(&nick) {
            origin.borrow().send_response(ERR_ALREADYREGISTRED, None,
                Some("somebody already registered with the same nickname")
            );
        } else {
            self.nicknames.insert(nick, origin.clone());
            self.send_welcome_msg(origin);
        }
    }
    
    /// Handles the QUIT command
    fn handle_quit(&mut self, origin: SharedClient, message: msg::QuitMessage) {
        let mut client = origin.borrow_mut();
        client.close_connection();
        self.nicknames.remove(&client.nickname);
        self.clients.remove(&client.id());
        let proxy = client.proxy();
        for (_, channel) in self.channels.iter() {
            // TODO make this more performant, cache channels in user?
            channel.send(channel::Quit(proxy.clone(), message.clone()))
        }
    }
    
    /// Handles the MODE command
    fn handle_mode(&mut self, origin: SharedClient, message: msg::ModeMessage) {
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
    
    /// Handles the PART command
    fn handle_part(&mut self, origin: SharedClient, message: msg::PartMessage) {
        for channel_name in message.channels.iter() {
            match self.channels.find_mut(channel_name) {
                Some(channel) => channel.send(channel::Part(
                        origin.borrow().proxy(), 
                        message.clone()
                )),
                None => origin
                    .borrow_mut().send_response(ERR_NOSUCHCHANNEL,
                        Some(channel_name.as_slice()), Some("No such channel"))
                    
                    
            }
        }
    }
}