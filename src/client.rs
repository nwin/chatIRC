use std::io::{TcpStream, BufferedReader, BufferedWriter};
use std::io::{IoResult};
use std::io::net::ip::{Ipv4Addr, Ipv6Addr};
use std::rc::{Rc, Weak};
//use std::sync::{Arc, Weak};
use std::cell::{RefCell};
use std::rand::{random};
use std::fmt::{Show, Formatter, FormatError};

use msg::{RawMessage, Message};
use cmd::{Command, REPLY, ResponseCode};

use server::{Event, ClientConnected, MessageReceived};

pub type SharedClient = Rc<RefCell<Client>>;
pub type WeaklySharedClient = Weak<RefCell<Client>>;

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

impl Clone for ClientId {
    fn clone(&self) -> ClientId {
        ClientId { id: [ self.id[0], self.id[1] ] }
    }
}

/// Proxy that forwards a message to a client
#[deriving(Clone)]
pub struct ClientProxy {
    id: ClientId,
    nick: String,
    tx: Sender<RawMessage>
}

impl ClientProxy {
    fn new(id: ClientId, nick: String, tx: Sender<RawMessage>) -> ClientProxy {
        ClientProxy {
            id: id,
            nick: nick,
            tx: tx
        }
    }
    
    pub fn send_msg(&self, msg: RawMessage) {
        let _ = self.tx.send_opt(msg);
    }
    
    pub fn send_response<'a>(&'a self, command: ResponseCode, 
                         params: &[&'a str], origin: &str) {
        let msg = RawMessage::new(
            REPLY(command), 
            (vec![self.nick.as_slice()] + params).as_slice(), 
            Some(origin)
        );
        let _ = self.tx.send_opt(msg);
    }
    pub fn id(&self) -> ClientId {
        self.id
    }
    pub fn update_nick(&mut self, nick: String) {
        self.nick = nick
    }
}


pub struct Client {
    id: ClientId,
    msg_tx: Sender<RawMessage>,
    stream: TcpStream,
    server_host: String,
    hostname: String,
    pub ip: String,
    pub nickname: String,
    pub username: String,
    pub realname: String,
}


impl Client {
    /// Spawns two threads for communication with the client
    /// Returns a SharedClient instance.
    /// TODO handle failures
    pub fn listen(host: String, mut stream: TcpStream, 
                         tx: Sender<Event>) -> IoResult<()> {
        let (msg_tx, rx) = channel();
        let err_tx = msg_tx.clone();
        let peer_name = stream.peer_name().unwrap();
        let this = Client {
            id: ClientId::new(&mut stream),
            msg_tx: msg_tx,
            stream: stream.clone(),
            server_host: host,
            hostname: self::net::get_nameinfo(peer_name),
            ip: format!("{}", peer_name.ip),
            nickname: "".to_string(),
            username: "".to_string(),
            realname: "".to_string()
        };
        debug!("hostname of client is {}", this.hostname)
        let receiving_stream = stream.clone();
        let id = this.id;
        // this has to be sended first otherwise we have a nice race conditions
        tx.send(ClientConnected(this));
        spawn(proc() {
            // TODO: write a proper 510 char line iterator
            // as it is now it is probably very slow
            // TODO handle failures properly, send QUIT
            for line in BufferedReader::new(receiving_stream).lines() {
                match RawMessage::parse(line.unwrap().as_slice()
                .trim_right().as_bytes()) {
                    Ok(raw) => {
                        debug!("received message {}", raw.to_string());
                        match Message::from_raw_message(raw) {
                            Ok(message) => tx.send(MessageReceived(id, message)),
                            Err(err_msg) => {
                                // TODO inject proper receiver
                                err_tx.send(err_msg)
                            }
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
    
    /// Returns the mask
    pub fn real_mask(&self) -> String {
        format!("{}!{}@{}", self.nickname, self.username, self.hostname)
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
        self.send_msg(RawMessage::new(REPLY(response), 
            params.as_slice(), Some(self.server_host.as_slice())))
    }
    
    /// Sends constructs a message and sends it the client
    pub fn send(&self, command: Command, params: &[&str], prefix: Option<&str>) {
        // Note mem::transmute is safe in this case, since &[&str] is just &[&[u8].
        self.send_msg(RawMessage::new(
            command, params, prefix
        ))
    }
    
    /// Sends a message to the client.
    /// Returns immediately.
    pub fn send_msg(&self, message: RawMessage) {
        let _ = self.msg_tx.send(message);
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
    
    /// Returns a proxy to the current client
    pub fn proxy(&self) -> ClientProxy {
        ClientProxy::new(self.id(), self.nickname.clone(), self.msg_tx.clone())
    }
}


impl PartialEq for Client {
    #[inline]
    fn eq(&self, other: &Client) -> bool {
        self.id == other.id
    }
}

impl Eq for Client {}
    
mod net {

    use std::io::net::ip::{SocketAddr, Ipv4Addr, Ipv6Addr};
    use libc::{malloc, sockaddr, sockaddr_in, sockaddr_in6, in_addr, in6_addr, c_int, c_char, socklen_t, AF_INET, AF_INET6};
    use std::mem::{size_of, transmute};
    use std::string;

    /*
     const char *
         inet_ntop(int af, const void * restrict src, char * restrict dst,
             socklen_t size);
    */
    extern {
        fn getnameinfo(sa: *const sockaddr, salen: socklen_t, 
                       host: *mut c_char, hostlen: socklen_t, 
                       serv: *mut c_char, servlen: socklen_t, 
                       flags: c_int) -> c_int;
    }

    //static NI_NUMERICHOST: c_int = 0x00000002;
    //static NI_NAMEREQD: c_int = 0x00000004;

    /// Returns the hostname for an ip address
    /// TODO: make this safe, see manpage
    pub fn get_nameinfo(peer_socket: SocketAddr) -> String {
        let SocketAddr { ip: ip, port: port } = peer_socket;
        let buf: *mut i8;
        let _ = unsafe {
            let hostlen = 80;
            buf = transmute(malloc(hostlen as u64 + 1));
            match ip {
                Ipv4Addr(a, b, c, d) => {
                    let addr = in_addr {
                        s_addr: a as u32 << 24 
                              | b as u32 << 16 
                              | c as u32 << 8 
                              | d as u32
                    };
                    let sockaddr = sockaddr_in {
                        sin_len: size_of::<sockaddr_in>() as u8,
                        sin_family: AF_INET as u8,
                        sin_port: port,
                        sin_addr: addr,
                        sin_zero: [0, ..8]
                    };
                    getnameinfo(transmute(&sockaddr), size_of::<sockaddr_in>() as i32, 
                                buf, hostlen, transmute(0u), 0, 0)
                },
                Ipv6Addr(a, b, c, d, e, f, g, h) => {
                    let sockaddr = sockaddr_in6 {
                        sin6_len: size_of::<sockaddr_in6>() as u8,
                        sin6_family: AF_INET6 as u8,
                        sin6_port: port,
                        sin6_flowinfo: 0,
                        sin6_addr: in6_addr {
                            s6_addr: [a, b, c, d, e, f, g, h]
                        },
                        sin6_scope_id: 0,
                    };
                    getnameinfo(transmute(&sockaddr), size_of::<sockaddr_in6>() as i32, 
                                buf, hostlen, transmute(0u), 0, 0)
                },
            }
       
        };
        unsafe {string::raw::from_buf(transmute(buf))}
    
    }
}