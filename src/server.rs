
use std::io::{TcpListener};
use std::io::{Listener, Acceptor};
use std::io::{IoResult};
use std::io::net;
use std::io;
use std::collections::{HashMap};

use msg::{MessageHandler};

use cmd;
use con::{Peer, PeerId, Connection};
use channel;

pub use self::Event::*;

pub struct Server {
    host: String,
    ip: String,
    port: u16, 
    tx: Option<Sender<Event>>,
    connections: HashMap<PeerId, Connection>,
    pub users: HashMap<PeerId, Peer>,
    pub nicks: HashMap<String, PeerId>,
    pub channels: HashMap<String, channel::Proxy>
}

/// Enumeration of the events the server can receive
pub enum Event {
    /// Message received from a client
    MessageReceived(PeerId, Box<MessageHandler + Send>),
    /// Connection to a peer established
    Connected(Connection),
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
        debug!("addresses found: {}", addresses)
        // Listen only on ipv4 for now…
        let ip = match addresses.iter().filter(
            |&v| match v { &net::ip::Ipv4Addr(_, _, _, _) => true, _ => false }
        ).nth(0) {
            Some(ip) => ip,
            None => return Err(io::IoError {
                kind: io::OtherIoError,
                desc: "cannot get host address",
                detail: None
            })
        };
        Ok(Server {
            host: host.to_string(),
            ip: format!("{}", ip),
            port: 6667,
            tx: None,
            connections: HashMap::new(),
            users: HashMap::new(),
            nicks: HashMap::new(),
            channels: HashMap::new()
        })
    }
    
    /// Starts the main loop and listens on the specified host and port.
    pub fn serve_forever(mut self) -> IoResult<Server> {
        // todo change this to a more general event dispatching loop
        for event in try!(self.start_listening()).iter() {
            match event {
                MessageReceived(client_id, handler) => {
                    let client = match self.users.get(&client_id) {
                        Some(client) => Some(client.clone()),
                        None => None
                    };
                    match client {
                        Some(client) => handler.invoke(&mut self, client),
                        None => {
                            let con = match self.connections.get(&client_id) {
                                Some(con) => Some(con.clone()),
                                None => None
                            };
                            match con {
                                Some(con) => handler.invoke_con(&mut self, con),
                                None => 
                                    error!(
                                        "Client {} not found when sending message.",
                                        client_id
                                    ) // The user is not registered yet
                            }
                        }
                    }
                },
                Connected(mut con) => { 
                    let id = con.id();
                    if self.connections.get(&id).is_some() {
                        // Duplicate client id.
                        con.close();
                    }
                    self.connections.insert(id, con); 
                },
                ChannelLost(name) => {
                    // TODO kick all users from this channel
                    // can be implemented when channel names are cached on all users
                    self.channels.remove(&name);
                }
            }
        }
        Ok(self)
    }
    
    fn start_listening(&mut self) -> IoResult<Receiver<(Event)>>  {
        let listener = TcpListener::bind(format!("{}:{}", self.ip, self.port).as_slice());
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
                        match Connection::listen(host.clone(), stream, tx.clone()) {
                            Ok(()) => {},
                            Err(err) => error!("{}", err)
                        }
                    }
                }
            }
        });
        Ok(rx)
    }
    
    /// Checks if the nickname is valid
    pub fn valid_nick(&self, nick: &str) -> bool {
        nick.len() > 1
    }
    
    /// Getter for hostname
    pub fn host(&self) -> &str {
        self.host.as_slice()
    }
    
    /// Finds a peer
    pub fn get_peer(&self, nick: &String) -> Option<&Peer> {
        self.nicks.get(nick).and_then(|id| self.users.get(id))
    }
    
    /// Getter for hostname
    pub fn tx(&self) -> Option<Sender<Event>> {
        self.tx.clone()
    }
    
    pub fn close_connection(&mut self, client: &Peer) {
        //self.nicks.remove(client.nick());
        let id = &client.id();
        self.connections.get_mut(id).map(|c| c.close());
        self.users.remove(id);
        self.connections.remove(id);
        error!("TODO: clean up nicknames in Server::close_connection")
    }
    
    pub fn add_user(&mut self, client: Peer) {
        self.nicks.insert(client.info().read().nick().to_string(), client.id());
        self.users.insert(client.id(), client);
    }
    
    /// Sends a welcome message to a newly registered client
    pub fn send_welcome_msg(&self, client: &Peer) {
        client.send_response(cmd::RPL_WELCOME, &["Welcome the {} IRC network"], self.host.as_slice())
    }
}