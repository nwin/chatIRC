use std::io::{TcpStream, BufferedReader, BufferedWriter};
use std::io::{IoResult};
use std::io::net::ip::{Ipv4Addr, Ipv6Addr};
use std::rand::{random};
use std::fmt::{Show, Formatter, Error};

pub use self::client::{UserInfo, SharedInfo, Peer};
pub use self::client::flag as reg;

use msg::{RawMessage};
use msg;

//use cmd::{Command, REPLY, ResponseCode};
use server;

mod client;
mod net;

/// Unique client id
#[deriving(Hash)]
pub struct PeerId {
    id: [u64, ..2]
}

impl PeerId {
    /// The client id is losely inspired by SILC but the silc
    /// method of also using the nickname for this is not applicable to IRC
    fn new(stream: &mut TcpStream) -> PeerId {
        PeerId { 
            id: [
                match stream.socket_name().unwrap().ip {
                    Ipv4Addr(a, b, c, d) => a as u32 <<24 | b as u32<<16 | c as u32<<8 | d as u32,
                    Ipv6Addr(_, _, _, _, _, _, a, b) => a as u32 << 16 | b as u32 
                } as u64 << 32
                | match stream.peer_name().unwrap().ip {
                    Ipv4Addr(a, b, c, d) => a as u32 <<24 | b as u32 <<16 | c as u32 <<8 | d as u32,
                    Ipv6Addr(_, _, _, _, _, _, a, b) => a as u32  << 16 | b as u32  
                } as u64, 
                random()
            ]
        }
    }
}

impl PartialEq for PeerId {
    #[inline]
    fn eq(&self, other: &PeerId) -> bool {
        self.id == other.id
    }
}

impl Eq for PeerId {}

impl Show for PeerId {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        write!(fmt, "{:x}{:x}", self.id[0], self.id[1])
    }
}

impl Clone for PeerId {
    fn clone(&self) -> PeerId {
        PeerId { id: [ self.id[0], self.id[1] ] }
    }
}

#[deriving(Clone)]
pub struct Connection {
    id: PeerId,
    peer: Peer,
    stream: TcpStream,
}


impl Connection {
    /// Spawns two threads for communication with the client
    /// Returns a SharedClient instance.
    /// TODO handle failures
    pub fn listen(server_host: String, mut stream: TcpStream, 
                         tx: Sender<server::Event>) -> IoResult<()> {
        let (msg_tx, rx) = channel();
        let err_tx = msg_tx.clone();
        let peer_name = try!(stream.peer_name());
        let id = PeerId::new(&mut stream);
        let hostname = self::net::get_nameinfo(peer_name);
        debug!("hostname of client is {}", hostname.clone())
        let peer = Peer::new(
            UserInfo::new(id, server_host, hostname.clone()),
            msg_tx,
        );
        let receiving_stream = stream.clone();
        let id = peer.id();
        // this has to be sended first otherwise we have a nice race conditions
        tx.send(server::Connected(Connection {
            id: id,
            peer: peer.clone(),
            stream: stream.clone(),
            
        }));
        spawn(proc() {
            // TODO: write a proper 510 char line iterator
            // as it is now it is probably very slow
            // TODO handle failures properly, send QUIT
            for line in BufferedReader::new(receiving_stream).lines() {
                match RawMessage::parse(line.unwrap().as_slice()
                .trim_right().as_bytes()) {
                    Ok(raw) => {
                        debug!("received message {}", raw.to_string());
                        match msg::get_handler(raw) {
                            Ok(handler) => tx.send(server::MessageReceived(id, handler)),
                            Err(Some(mut err_msg)) => {
                                err_msg.set_prefix(hostname.as_slice());
                                err_tx.send(err_msg)
                            },
                            Err(None) => {} // Ingore error
                            
                        }
                    },
                    Err(_) => {}
                }
            }
        });
        spawn(proc() {
            // TODO: socket timeout
            // implement when pings are send out
            // TODO handle failures properly, send QUIT
            let mut output_stream = BufferedWriter::new(stream);
            for message in rx.iter() {
                debug!("sending message {}", message.to_string());
                output_stream.write(message.as_slice()).unwrap();
                output_stream.write(b"\r\n").unwrap();
                output_stream.flush().unwrap();
            }
        });
        Ok(())
    }
    
    /// Closes the connection to the client
    pub fn close(&mut self) {
        let _ = self.stream.close_read();
        let _ = self.stream.close_write();
    }
    
    /// Accessor for the client id
    #[inline]
    pub fn id(&self) -> PeerId {
        self.id
    }
    
    /// Returns a proxy to the current client
    pub fn peer(&self) -> Peer {
        self.peer.clone()
    }
}


impl PartialEq for Connection {
    #[inline]
    fn eq(&self, other: &Connection) -> bool {
        self.id == other.id
    }
}

impl Eq for Connection {}