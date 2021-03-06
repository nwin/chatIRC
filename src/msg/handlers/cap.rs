use cmd;
use msg::RawMessage;

use server::{Server};
use con::{Peer, Connection};

/// Handles the CAP command.
#[deriving(Clone)]
pub struct Cap {
    raw: RawMessage,
    subcmd: String,
    params: Vec<String>,
}

impl super::MessageHandler for Cap {
    fn from_message(message: RawMessage) -> Result<Box<Cap>, Option<RawMessage>> { 
        let params = message.params();
        let mut params = params.iter().map(|&p| 
            String::from_utf8_lossy(p).to_string()
        );
        let subcmd = if params.len() > 0 {
            params.nth(0).unwrap()
        } else { return Err(None) };
        Ok(box Cap {
            raw: message.clone(), subcmd: subcmd, params: params.collect()
        })
    }
    
    fn invoke(&self, server: &mut Server, peer: Peer) {
        let server_name = server.host().to_string();
        info!("cap:invoke")
        let this = (*self).clone();
        spawn(proc() {
            let info = peer.info().read();
            let nick = info.nick().as_slice();
            match this.subcmd.as_slice() {
                "LS" => {
                    peer.send_msg(RawMessage::new(cmd::CAP, &[
                        nick, "LS", ""//, "multi-prefix sasl"
                    ], Some(server_name.as_slice())))
                },
                "REQ" => {
                    peer.send_msg(RawMessage::new(cmd::CAP, &[
                        nick, "NAQ", this.params.connect(" ").as_slice()
                    ], Some(server_name.as_slice())))
                },
                _ => {}
            }
        })
    }
    
    fn invoke_con(&self, server: &mut Server, origin: Connection) {
        self.invoke(server, origin.peer())
    }
    
    fn raw_message(&self) -> &RawMessage {
        &self.raw
    }
}