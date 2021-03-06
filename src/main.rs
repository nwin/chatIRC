#![crate_name = "chätd"]
#![experimental]
#![comment = "IRC daemon written in pure rust, doomed to be fast"]
#![license = "MIT/ASL2"]
#![crate_type = "bin"]
#![feature(globs)]
#![feature(phase)]
#![feature(macro_rules)]
#![feature(unsafe_destructor)]
#![feature(if_let)]
#[phase(plugin, link)] extern crate log;
extern crate collections;
extern crate libc;

#[cfg(not(test))]
use server::{run_server};

// pub only for documentation purposes
pub mod con;
pub mod server;
pub mod channel;
pub mod msg;
pub mod cmd;
pub mod util;


#[cfg(not(test))]
fn main() {
    match run_server("localhost") {
        Ok(_) => {},
        Err(err) => error!("{}", err)
    }
}
