
use std::io::{TcpListener};
use std::io::{Listener, Acceptor};
use std::io::{IoResult};
use std::io::net;
use std::io;
use std::collections::hashmap::{HashMap};

use client::{SharedClient, Client, ClientId};
use msg::{MessageHandler};

use cmd;
use channel;

pub struct Server {
    host: String,
    ip: String,
    port: u16, 
    tx: Option<Sender<Event>>,
    clients: HashMap<ClientId, SharedClient>,
    pub registered: HashMap<String, SharedClient>,
    pub channels: HashMap<String, channel::Proxy>
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
        client.borrow().send_response(cmd::RPL_WELCOME, None, None)
    }
}