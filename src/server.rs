
use std::io::{TcpListener};
use std::io::{Listener, Acceptor};
use std::io::{IoResult};
use std::collections::hashmap::{HashMap};

use message::{Message};
use channel::{Channel};
use client::{SharedClient, Client, ClientId};
use util::{verify_nick, verify_channel, verify_receiver};

use cmd::*;

pub struct IrcServer {
    host: String,
    port: u16, 
    clients: HashMap<ClientId, SharedClient>,
    nicknames: HashMap<String, SharedClient>,
    channels: HashMap<String, Channel>
}

pub enum Event {
    Message(ClientId, Message),
    ClientConnected(Client)
}

impl IrcServer {
    /// Creates a new IRC server instance.
    pub fn new(host: &str, port: u16) -> IrcServer {
        IrcServer {
            host: String::from_str(host),
            port: port,
            clients: HashMap::new(),
            nicknames: HashMap::new(),
            channels: HashMap::new()
        }
    }
    
    /// Starts the main loop and listens on the specified host and port.
    pub fn serve_forever(mut self) -> IoResult<IrcServer> {
        // todo change this to a more general event dispatching loop
        let message_rx = try!(self.start_listening());
        for event in message_rx.iter() {
            match event {
                Message(client_id, message) => {
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
        let listener = TcpListener::bind(self.host.as_slice(), self.port);
        let acceptor = try!(listener.listen());
        let (tx, rx) = channel();
        spawn(proc() {
            let mut a = acceptor; // https://github.com/rust-lang/rust/issues/11958
            for maybe_stream in a.incoming() {
                match maybe_stream {
                    Err(err) => { error!("{}", err) }
                    Ok(stream) => {
                        match Client::listen(stream, tx.clone()) {
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
    fn dispatch(&mut self, origin: SharedClient, message: Message) {
        // TODO: wrap this in a proc?
        match message.command() {
            PRIVMSG => self.handle_msg(origin, message),
            JOIN => self.handle_join(origin, message),
            NICK => self.handle_nick(origin, message),
            USER => self.handle_user(origin, message),
            QUIT => self.handle_quit(origin, message),
            REPLY(_) => {}, // should not come from a client, ignore
            UNKNOWN(_) => 
                error!(
                    "Handling of message {} not implemented yet.",
                    message.command().to_string())
        }
    }
    
    /// Sends a welcome message to a newly registered client
    fn welcome_user(&self, client: SharedClient) {
        let mut c = client;
        c.borrow_mut().send_response(RPL_WELCOME, None, None)
    }

    fn handle_msg(&mut self, origin: SharedClient, message: Message) {
        match message.params() {
            Some(ref params) if params.len() > 1 => {
                for receiver in params[0].as_slice().split(|&v| v == b',' )
                                         .filter_map(|v| verify_receiver(v)) {
                    if receiver.starts_with("#") {
                        match self.channels.find_mut(&receiver.to_string()) {
                            Some(channel) => 
                                channel.handle_msg(origin.clone(), message.clone()),
                            None => {}
                        }
                    }
                }
            },
            _ => origin.borrow_mut().send_response(ERR_NEEDMOREPARAMS,
                Some(message.command().to_string().as_slice()),
                Some("not enought params given")
            )
        }
    }
    
    /// Handles the nick command
    ///    Command: NICK
    /// Parameters: <nickname> [ <hopcount> ]
    fn handle_nick(&self, mut origin: SharedClient, message: Message) {
        let mut client = origin.borrow_mut();
        match message.params() {
            Some(params) => {
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
            },
            None => {
                client.send_response(ERR_NONICKNAMEGIVEN, None,
                    Some("no nickname given")
                )
            }
        }
    }
    
    /// Handles the USER command
    fn handle_user(&mut self, mut origin: SharedClient, message: Message) {
        match message.params() {
            Some(params) => if params.len() >= 4 {
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
                    self.welcome_user(origin);
                }
            } else {
                origin.borrow_mut().send_response(ERR_NEEDMOREPARAMS,
                    Some(message.command().to_string().as_slice()),
                    Some("not enought params given")
                )
            },
            None => {
                origin.borrow_mut().send_response(ERR_NEEDMOREPARAMS,
                    Some(message.command().to_string().as_slice()),
                    Some("no params given")
                )
            }
        }
    }
    
    /// Handles the QUIT command
    fn handle_quit(&mut self, origin: SharedClient, _: Message) {
        // TODO communicate this to other users
        let mut client = origin.borrow_mut();
        client.close_connection();
        self.nicknames.remove(&client.nickname);
        self.clients.remove(&client.id());
    }
    
    /// Handles the JOIN command
    ///    Command: JOIN
    /// Parameters: <channel>{,<channel>} [<key>{,<key>}]
    fn handle_join(&mut self, origin: SharedClient, message: Message) {
        match message.params() {
            Some(ref params) if params.len() > 0 => {
                let passwords: Vec<&[u8]> = if params.len() > 1 {
                    params[1].as_slice().split(|c| *c == b',').collect()
                } else {
                    Vec::new()
                };
                for (i, channel_name) in params[0].as_slice().split(|c| *c == b',').enumerate() {
                    match verify_channel(channel_name) {
                        Some(channel) => {
                            self.channels.find_or_insert_with(channel.to_string(), |key| {
                                Channel::new(key.clone())
                            }).handle_join(
                                origin.clone(), 
                                passwords.as_slice().get(i).map(|v| *v)
                            )
                        },
                        None => origin.borrow_mut().send_response(ERR_NOSUCHCHANNEL,
                            Some(String::from_utf8_lossy(channel_name.as_slice()).as_slice()),
                            Some("Invalid channel name.")
                        )
                    }
                }
            },
            _ => origin.borrow_mut().send_response(ERR_NEEDMOREPARAMS,
                     Some(message.command().to_string().as_slice()),
                     Some("no params given")
                 )
        }
    }
}