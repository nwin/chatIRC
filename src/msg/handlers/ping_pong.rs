use msg::RawMessage;
use server::{Server};
use con::Peer;

#[allow(dead_code)] 
pub struct Ping {
 raw: RawMessage,
 payload: Option<String>
}

impl super::MessageHandler for Ping {
    fn from_message(message: RawMessage) -> Result<Box<Ping>, RawMessage> {
       let payload = message.params().as_slice().get(0).map(
           |&v| String::from_utf8_lossy(v).to_string());
       Ok(box Ping {
           raw: message, payload: payload
       })
    }
    fn invoke(self, _: &mut Server, _: Peer) {
        // ignore for now
    }
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}

#[allow(dead_code)] 
pub struct Pong {
 raw: RawMessage,
 payload: Option<String>
}

impl super::MessageHandler for Pong {
    fn from_message(message: RawMessage) -> Result<Box<Pong>, RawMessage> { 
       let payload = message.params().as_slice().get(0).map(
           |&v| String::from_utf8_lossy(v).to_string());
       Ok(box Pong {
           raw: message, payload: payload
       })
    }
    fn invoke(self, _: &mut Server, _: Peer) {
        // ignore for now
    }
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}
