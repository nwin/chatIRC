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



#[cfg(target_os = "linux")]
#[cfg(target_os = "android")]
fn new_sockaddr_in(port: u16, addr: in_addr) -> sockaddr_in {
    sockaddr_in {
        sin_family: AF_INET as u16,
        sin_port: port,
        sin_addr: addr,
        sin_zero: [0, ..8]
    }
}
#[cfg(target_os = "macos")]
fn new_sockaddr_in(port: u16, addr: in_addr) -> sockaddr_in {
    sockaddr_in {
        sin_len: size_of::<sockaddr_in>() as u8,
        sin_family: AF_INET as u8,
        sin_port: port,
        sin_addr: addr,
        sin_zero: [0, ..8]
    }
}


#[cfg(target_os = "linux")]
#[cfg(target_os = "android")]
fn new_sockaddr_in6(port: u16, addr: in6_addr) -> sockaddr_in6 {
    sockaddr_in6 {
        sin6_family: AF_INET6 as u16,
        sin6_port: port,
        sin6_flowinfo: 0,
        sin6_addr: addr,
        sin6_scope_id: 0,
    }
}
#[cfg(target_os = "macos")]
fn new_sockaddr_in6(port: u16, addr: in6_addr) -> sockaddr_in6 {
    sockaddr_in6 {
        sin6_len: size_of::<sockaddr_in6>() as u8,
        sin6_family: AF_INET6 as u8,
        sin6_port: port,
        sin6_flowinfo: 0,
        sin6_addr: addr,
        sin6_scope_id: 0,
    }
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
                let sockaddr = new_sockaddr_in(port, addr);
                getnameinfo(transmute(&sockaddr), size_of::<sockaddr_in>() as socklen_t, 
                            buf, hostlen, transmute(0u), 0, 0)
            },
            Ipv6Addr(a, b, c, d, e, f, g, h) => {
                let addr = in6_addr {
                    s6_addr: [a, b, c, d, e, f, g, h]
                };
                let sockaddr = new_sockaddr_in6(port, addr);
                getnameinfo(transmute(&sockaddr), size_of::<sockaddr_in6>() as socklen_t, 
                            buf, hostlen, transmute(0u), 0, 0)
            },
        }
   
    };
    unsafe {string::raw::from_buf(transmute(buf))}

}
