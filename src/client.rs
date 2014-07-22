use std::io::{TcpStream, BufferedReader, BufferedWriter};
use std::io::{IoResult};
use std::io::net::ip::{Ipv4Addr, Ipv6Addr};
use std::rc::{Rc, Weak};
//use std::sync::{Arc, Weak};
use std::cell::{RefCell};
use std::rand::{random};
use std::fmt::{Show, Formatter, FormatError};

use message::{Message};
use cmd::{Command, REPLY, ResponseCode};

use server::{Event, ClientConnected, Message};

pub type SharedClient = Rc<RefCell<Client>>;
pub type WeaklySharedClient = Weak<RefCell<Client>>;

pub struct Client {
    id: ClientId,
    msg_tx: Sender<Message>,
    stream: TcpStream,
    pub ip: String,
    pub nickname: String,
    pub username: String,
    pub realname: String,
}

/// Unique client id
#[deriving(Hash)]
pub struct ClientId {
    id: [u64, ..2]
}

impl ClientId {
    /// The client id is losely inspired by SILC but the silc
    /// method of using the nickname is not applicable to IRC
    fn new(stream: &mut TcpStream) -> ClientId {
        ClientId { 
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

impl PartialEq for ClientId {
    #[inline]
    fn eq(&self, other: &ClientId) -> bool {
        self.id == other.id
    }
}

impl Eq for ClientId {}

impl Show for ClientId {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), FormatError> {
        write!(fmt, "{:x}{:x}", self.id[0], self.id[1])
    }
}

impl Client {
    /// Spawns two threads for communication with the client
    /// Returns a SharedClient instance.
    /// TODO handle failures
    pub fn listen(mut stream: TcpStream, 
                         tx: Sender<Event>) -> IoResult<()> {
        let (msg_tx, rx) = channel();
        let this = Client {
            id: ClientId::new(&mut stream),
            msg_tx: msg_tx,
            stream: stream.clone(),
            ip: format!("{}", stream.peer_name().unwrap().ip),
            nickname: "".to_string(),
            username: "".to_string(),
            realname: "".to_string()
        };
        let receiving_stream = stream.clone();
        let id = this.id;
        // this has to be sended first otherwise we have a nice race conditions
        tx.send(ClientConnected(this));
        spawn(proc() {
            // TODO: write a proper 510 char line iterator
            for line in BufferedReader::new(receiving_stream).lines() {
                let message = Message::parse(line.unwrap().as_slice().trim_right().as_bytes()).unwrap();
                debug!("Received message {}", message.to_string());
                tx.send(Message(id, message))
            }
        });
        spawn(proc() {
            let mut output_stream = BufferedWriter::new(stream);
            for message in rx.iter() {
                debug!("Sending message {}", message.to_string());
                output_stream.write(message.as_slice()).unwrap();
                output_stream.write(b"\r\n").unwrap();
                output_stream.flush().unwrap();
            }
        });
        Ok(())
    }
    
    /// Converts the Client to a shared client
    pub fn as_shared(self) -> SharedClient {
        Rc::new(RefCell::new(self))
    }
    
    /// Sends a numeric response code to the client.
    /// Returns immediately.
    pub fn send_response<'a>(&'a self, response: ResponseCode, 
                         origin: Option<&'a str>, reason: Option<&'a str>) {
        let mut params: Vec<&'a str> = Vec::with_capacity(2);
        if self.nickname.len() == 0 {
            // TODO follow rust-dev
            // this static lifetime made all lifetime annotations necessary
            params.push("*")
        } else {
            params.push(self.nickname.as_slice())
        }
        if origin.is_some() {
            params.push(origin.unwrap())
        }
        if reason.is_some() {
            params.push(reason.unwrap())
        }
        self.send_msg(Message::new(REPLY(response), 
            params.as_slice(), Some("localhost")))
    }
    
    /// Sends constructs a message and sends it the client
    pub fn send(&self, command: Command, params: &[&str], prefix: Option<&str>) {
        // Note mem::transmute is safe in this case, since &[&str] is just &[&[u8].
        self.send_msg(Message::new(
            command, params, prefix
        ))
    }
    
    /// Sends a message to the client.
    /// Returns immediately.
    pub fn send_msg(&self, message: Message) {
        self.msg_tx.send(message)
    }
    
    /// Closes the connection to the client
    pub fn close_connection(&mut self) {
        let _ = self.stream.close_read();
        let _ = self.stream.close_write();
    }
    
    /// Accessor for the client id
    #[inline]
    pub fn id(&self) -> ClientId {
        self.id
    }
}


impl PartialEq for Client {
    #[inline]
    fn eq(&self, other: &Client) -> bool {
        self.id == other.id
    }
}

impl Eq for Client {}