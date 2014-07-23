#![crate_name = "rusty-irc"]
#![unstable]
#![comment = "IRC daemon written in pure rust, doomed to be fast"]
#![license = "MIT/ASL2"]
#![crate_type = "bin"]
#![feature(globs)]
#![feature(phase)]
#![feature(macro_rules)]

#[phase(plugin, link)] extern crate log;
extern crate collections;
extern crate libc;

// pub only for documentation purposes
pub mod message;
pub mod client;
pub mod server;
#[allow(dead_code)]
pub mod channel;
pub mod cmd;
pub mod util;

fn main() {
    match server::IrcServer::new("127.0.0.1", 6667).serve_forever() {
        Ok(_) => {},
        Err(err) => error!("{}", err)
    }
}
