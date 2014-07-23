use cmd::{Command};

#[deriving(Show, Clone)]
// Helper struct to efficently adress the different parts
// of the message.
struct ASlice {
    start: uint,
    end: uint
}

impl ASlice {
    #[inline] 
    fn slice_vec<'a, T>(&self, vec: &'a Vec<T>) -> &'a[T] {
        vec.slice(self.start, self.end)
    }
}

/// IRC message
// TODO: do not use vecs for that, better [u8, ..510] and slices to that
// or just safe offsets in the message parts fields…
#[deriving(Show, Clone)]
pub struct RawMessage {
    raw_message: Vec<u8>,
    prefix: Option<ASlice>,
    command: ASlice, 
    params: Vec<ASlice>
}

/// Searches a slice for the first occurence of needle
fn position<T: PartialEq>(this: &[T], needle: &[T]) -> Option<uint> {
    for i in range(0, this.len()) {
        if this.slice_from(i).starts_with(needle) {
            return Some(i)
        }
    }
    None
}

impl RawMessage {
    /// Creates a new message
    pub fn new(command: Command, 
                params: &[&str], 
                prefix: Option<&str>) -> RawMessage {
        let mut raw_message = Vec::with_capacity(20);
        let msg_prefix = prefix.map(|p| {
            raw_message.push(b':');
            raw_message.push_all(p.as_bytes());
            raw_message.push(b' ');
            ASlice { start: 1, end: p.as_bytes().len() + 1 }
        });
        let cmd = command.to_bytes();
        let cmd_bytes = cmd.as_slice();
        let msg_command = ASlice{ start: raw_message.len(),
                                    end: raw_message.len() + cmd_bytes.len() };
        raw_message.push_all(cmd_bytes);
        let mut start = msg_command.end;
        let msg_params = params.iter().map( |&param| {
            let bytes = param.as_bytes();
            raw_message.push(b' ');
            raw_message.push_all(bytes);
            let end = start + 1 + bytes.len();
            let slice = ASlice { start: start + 1, end: end };
            start = end;
            slice
        }).collect();
        RawMessage {
            raw_message: raw_message,
            prefix: msg_prefix,
            command: msg_command,
            params: msg_params
        }
    }
    
    /// Parses a message. Extracts the prefix, command and the params
    pub fn parse(mut message: &[u8]) -> Result<RawMessage, &'static str> {
        // Check for message prefix (starts with : and ends with space)
        let raw_message = Vec::from_slice(message);
        let prefix = if message.starts_with([b':']) {
            let prefix_end = match message.position_elem(&b' ') { 
                Some(v) => v, 
                None => return Err("RawMessage does not contain a command.") 
            };
            message = message.slice_from(prefix_end + 1);
            Some(ASlice{ start: 1, end: prefix_end })
        } else {
            None
        };
        let cmd_start = prefix.map(|v| v.end + 1).unwrap_or(0);
        let trailing = match position(message, " :".as_bytes()) {
            Some(trailing_pos) => {
                message = message.slice_to(trailing_pos);
                Some(ASlice { 
                    start: cmd_start + trailing_pos + 2, 
                    end: raw_message.len() })
            },
            None => None
        };
        let mut middle_iter = message.split(|&x| x == b' ');
        let command = match middle_iter.next() {
            Some(cmd) =>
                ASlice { start: cmd_start, 
                           end: cmd_start + cmd.len() },
            None => return Err("RawMessage does not contain a command.") 
        };
        let mut start = command.end + 1;
        let mut params: Vec<ASlice> = middle_iter.map(|p| {
            let slice = ASlice { start: start, end: start + p.len() };
            start = slice.end + 1;
            slice
        }).collect();
        if trailing.is_some() {
            params.push(trailing.unwrap())
        }
        Ok(RawMessage {
            raw_message: raw_message,
            prefix: prefix,
            command: command,
            params: params
        })
    }
    
    /// Returns the message prefix
    /// It might contain non-utf8 chars and thus only bytes are returned.
    pub fn prefix<'a>(&'a self) -> Option<&'a[u8]> {
        self.prefix.map(|p| p.slice_vec(&self.raw_message))
    }
    
    /// Sets the message prefix
    /// For all purposes of this library &str should be fine since only
    /// ASCII chars will be used for the prefix.
    pub fn set_prefix(&mut self, prefix: &str) {
        let bytes = prefix.as_bytes();
        let offset = match self.prefix {
            Some(ref mut old_prefix) => {
                self.raw_message = Vec::from_slice(b":")
                    .append(bytes)
                    .append(self.raw_message.slice_from(old_prefix.end));
                let offset = prefix.len() - old_prefix.end + 1;
                old_prefix.end += offset;
                offset
            },
            None => {
                self.raw_message = Vec::from_slice(b":")
                    .append(bytes)
                    .append(b" ")
                    .append(self.raw_message.as_slice());
                prefix.len() + 2
            }
        };
        self.command.start += offset;
        self.command.end += offset;
        for param in self.params.mut_iter() {
            param.start += offset;
            param.end += offset;
        }
    }

    /// Returns the message command
    pub fn command<'a>(&'a self) -> Command<'a> {
        Command::from_bytes(self.command.slice_vec(&self.raw_message))
    }
    
    /// Returns the parameters of the command
    /// *Note* since the IRC protocol does not define any encoding
    /// raw bytes are returned.
    pub fn params<'a>(&'a self) -> Vec<&'a[u8]> {
        self.params.iter().map(
            |slice| slice.slice_vec(&self.raw_message)
        ).collect()
    }

    /// Returns the raw message
    pub fn as_slice<'a>(&'a self) -> &'a[u8] {
        self.raw_message.as_slice()
    }
    
    /// Returns the message as an string for debugging/logging
    pub fn to_string(&self) -> String {
        String::from_utf8_lossy(self.raw_message.as_slice()).into_string()
    }
}

#[cfg(test)]
mod tests {
	use super::{RawMessage};
	use cmd::{JOIN};
	/// Test the nickname validation function
	#[test]
	fn test_message_parser() {
        let m = RawMessage::parse(":prefix JOIN #channel".as_bytes()).unwrap();
        assert_eq!(m.prefix().unwrap(), b"prefix")
        assert!(match m.command() {JOIN => true, _ => false})
        assert_eq!(m.params().unwrap()[0], b"#channel")
	}
	/// Test the prefix setter
	#[test]
	fn test_prefix_setter() {
        let mut m = RawMessage::parse(":prefix JOIN #channel".as_bytes()).unwrap();
        m.set_prefix("new prefix");
        assert_eq!(String::from_utf8_lossy(m.prefix().unwrap()).to_owned(),
                   String::from_str("new prefix").to_owned())
        assert!(match m.command() {JOIN => true, _ => false})
        assert_eq!(m.params().unwrap()[0], b"#channel")
        assert_eq!(m.as_slice(), b":new prefix JOIN #channel")
	}
	/// Test message creation
	#[test]
	fn test_msg_new() {
        let m = RawMessage::new(JOIN, Some(&["#channel"]), Some("prefix"));
        assert_eq!(m.prefix().unwrap(), b"prefix")
        assert!(match m.command() {JOIN => true, _ => false})
        assert_eq!(m.params().unwrap()[0], b"#channel")
        assert_eq!(m.as_slice(), b":prefix JOIN #channel")
	}
}