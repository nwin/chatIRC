use cmd::{REPLY, UNKNOWN, ResponseCode};
use cmd;

use server::{Server, ChannelProxy};
use client::SharedClient;
use channel;

use super::{RawMessage};
use super::util;

mod join;

macro_rules! handle {
    {$(
        $command:ident with $handler:path;
    )*} => {
/// Temporary dispatcher
pub fn get_handler(message: RawMessage) -> Result<Box<MessageHandler + Send>, RawMessage> {
    match message.command() {
        $(cmd::$command => {
            let t: Result<Box<$handler>, RawMessage> = MessageHandler::from_message(message);
            t.map(|v| v as Box<MessageHandler + Send>)
        },)*
        _ => fail!("get_handler, not handled yet")
    }
}
}}

handle!{
    JOIN with self::join::JoinHandler;
}

///// Temporary dispatcher
//pub fn get_handler(message: RawMessage) -> Result<Box<MessageHandler + Send>, RawMessage> {
//    let t: Result<Box<self::join::JoinHandler>, RawMessage> = MessageHandler::from_message(message);
//    t.map(|v| v as Box<MessageHandler + Send>)
//}

/// Trait for the various message handlers
///
/// The general template for the implementation of new messages is:
///
/// ```no_run
/// struct XXHandler {
///     raw: RawMessage,
/// }
/// impl XXHandler {
///     fn handle_XX() {
///     }
/// }
/// impl MessageHandler for XXHandler {
///     fn from_message(message: RawMessage) -> Result<Box<Self>, RawMessage> {
///     }
///     fn invoke(self, server: &mut Server, origin: SharedClient) {
///     }
///     fn raw_message(&self) -> &RawMessage {
///         self.raw
///     }
/// }
/// ```
pub trait MessageHandler {
    /// Tries to parse the raw message and return handler for the message.
    /// Returns a error message if something goes wrong
    fn from_message(message: RawMessage) -> Result<Box<Self>, RawMessage>;
    /// Invokes the message handler. Since this usually happens on the main event loop,
    /// the method should avoid time-consuming operations such that the main thread
    /// is not blocked for an extended time period.
    fn invoke(self, server: &mut Server, origin: SharedClient);
    /// Returns the raw message the handler is bases on
    fn raw_message(&self) -> &RawMessage;
    
}