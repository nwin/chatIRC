#![crate_name = "rircd"]
#![experimental]
#![comment = "IRC daemon written in pure rust, doomed to be fast"]
#![license = "MIT/ASL2"]
#![crate_type = "bin"]
#![feature(globs)]
#![feature(phase)]
#![feature(macro_rules)]

#[phase(plugin, link)] extern crate log;
extern crate collections;
extern crate libc;

use server::{run_server};

// pub only for documentation purposes
pub mod message;
pub mod client;
pub mod server;
#[allow(dead_code)]
pub mod channel;
pub mod cmd;
pub mod util;

fn main() {
    match run_server("localhost") {
        Ok(_) => {},
        Err(err) => error!("{}", err)
    }
}
