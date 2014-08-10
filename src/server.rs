
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
    pub registered: HashMap<String, SharedClient>,
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
            registered: HashMap::new(),
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
    
    /// Getter for hostname
    pub fn host(&self) -> &str {
        self.host.as_slice()
    }
    
    /// Getter for hostname
    pub fn tx(&self) -> Option<Sender<Event>> {
        self.tx.clone()
    }
    
    pub fn remove_client(&mut self, client: &SharedClient) {
        let client = client.borrow();
        self.registered.remove(&client.nickname);
        self.clients.remove(&client.id());
    }
    
    /// Sends a welcome message to a newly registered client
    pub fn send_welcome_msg(&self, client: &SharedClient) {
        client.borrow().send_response(RPL_WELCOME, None, None)
    }

    //fn handle_privmsg(&mut self, origin: SharedClient, mut message: msg::PrivMessage) {
    //    message.raw.set_prefix(origin.borrow().nickname.as_slice());
    //    for receiver in message.receiver.move_iter() {
    //        match receiver {
    //            ChannelName(name) => match self.channels.find_mut(&name.to_string()) {
    //                Some(channel) => 
    //                    channel.send(channel::Message(
    //                        channel::PRIVMSG,
    //                        origin.borrow().id(),
    //                        message.raw.clone(),
    //                    )),
    //                None => {}
    //            },
    //            NickName(nick) => match self.registered.find_mut(&nick.to_string()) {
    //                Some(client) => {
    //                    client.borrow_mut().send_msg(message.raw.clone());
    //                },
    //                None => {}
    //            },
    //            _ => {}
    //        }
    //    }
    //}
}