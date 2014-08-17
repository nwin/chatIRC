use cmd::{REPLY, UNKNOWN};
use cmd;

use con::{Peer, Connection};

use server::{Server};
use super::{RawMessage};

mod registration;
//mod msg;
//mod join;
//mod part;
//mod mode;
//mod lists;
mod simple;
mod ping_pong;

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
        REPLY(_) => {
            let t: Result<Box<Reply>, RawMessage> = MessageHandler::from_message(message);
            t.map(|v| v as Box<MessageHandler + Send>)
        },
        UNKNOWN(_) => {
            let t: Result<Box<ExtensionHandler>, RawMessage> = MessageHandler::from_message(message);
            t.map(|v| v as Box<MessageHandler + Send>)
        },
        _ => fail!()
    }
}
}}

handle!{
    //PRIVMSG with self::msg::Privmsg;
    //NAMES with self::lists::Names;
    //WHO with self::lists::Who;
    //MODE with self::mode::Mode;
    //JOIN with self::join::Join;
    TOPIC with self::simple::Topic;
    //PART with self::part::Part;
    //QUIT with self::part::Quit;
    NICK with self::registration::Nick;
    USER with self::registration::User;
    PING with self::ping_pong::Ping;
    PONG with self::ping_pong::Pong;
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
/// pub struct Handler {
///     raw: RawMessage,
/// }
/// impl Handler {
///     fn handle_XX() {
///     }
/// }
/// impl super::MessageHandler for Handler {
///     fn from_message(message: RawMessage) -> Result<Box<Handler>, RawMessage> {
///     }
///     fn invoke(self, server: &mut Server, origin: SharedClient) {
///     }
///     fn raw_message(&self) -> &RawMessage {
///         &self.raw
///     }
/// }
/// ```
pub trait MessageHandler {
    /// Tries to parse the raw message.
    ///
    /// Returns the handler for the message or an error message
    /// if something goes wrong
    fn from_message(message: RawMessage) -> Result<Box<Self>, RawMessage>;
    /// Invokes the message handler. 
    ///
    /// Since this usually happens on the main event loop,
    /// the method should avoid time-consuming operations such that the main thread
    /// is not blocked for an extended time period.
    fn invoke(self, server: &mut Server, origin: Peer);
    /// Invoke the handler for a connection.
    ///
    /// This only happens if the client is not registered. The default implementation
    /// does nothing. Overwrite to influence the registration process.
    fn invoke_con(self, _: &mut Server, _: Connection) {}
    /// Returns the raw message the handler is bases on
    fn raw_message(&self) -> &RawMessage;
    
}

/// Handles (ignores) reply codes from clients
struct Reply {
    raw: RawMessage,
}
impl MessageHandler for Reply {
    fn from_message(message: RawMessage) -> Result<Box<Reply>, RawMessage> {
        Ok(box Reply { raw: message })
    }
    fn invoke(self, _: &mut Server, _: Peer) {
        // Ingore reply codes from clients they are not allowed to send any
    }
    fn raw_message(&self) -> &RawMessage { &self.raw }
}

/// Handles unknown messages. Could be used as an entry point for plugins
pub struct ExtensionHandler {
    raw: RawMessage,
}
impl MessageHandler for ExtensionHandler {
    fn from_message(message: RawMessage) -> Result<Box<ExtensionHandler>, RawMessage> {
        Ok(box ExtensionHandler { raw: message })
    }
    fn invoke(self, _: &mut Server, _: Peer) {
        error!("Handling of message {} not implemented yet", self.raw.command().to_string())
    }
    fn raw_message(&self) -> &RawMessage { &self.raw }
}