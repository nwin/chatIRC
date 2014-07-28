use collections::str::{from_utf8};

/// Checks if the nickname is valid
pub fn valid_nick(nick: &str) -> bool {
    // <nick>       ::= <letter> { <letter> | <number> | <special> }
    //<special>    ::= '-' | '[' | ']' | '\' | '`' | '^' | '{' | '}'
    // 
    // As of http://tools.ietf.org/html/rfc2812#section-2.3.1
    // nickname   =  ( letter / special ) *8( letter / digit / special / "-" )
    // special    =  %x5B-60 / %x7B-7D
    for (i, char) in nick.chars().enumerate() {
        if i == 9 {
            return false
        }
        match char {
            'a'..'z' | 'A'..'Z' | '\x5B'..'\x60' | '\x7B'..'\x7D'
                if i == 0 => {},
            'a'..'z' | 'A'..'Z' | '0'..'9' | '\x5B'..'\x60' | '\x7B'..'\x7D' | '-' 
                if i != 0 => {},
            _ => return false
        }
    }
    true
}

/// Validates the raw nickname and converts it into a string. 
pub fn verify_nick<'a>(nick: &'a [u8]) -> Option<&'a str> {
    match from_utf8(nick) {
        None => None,
        Some(nick) => if valid_nick(nick) { Some(nick) } else { None }
    }
}

pub fn valid_channel(channel: &str) -> bool {
    for (i, char) in channel.chars().enumerate() {
        match char {
            '#' | '&' if i == 0 => {},
            _ if i == 0 => { return false },
            ' ' | '\x07' | ',' => { return false }
            _ => {}
        }
    }
    true
}

/// Validates the raw channel name and converts it into a string. 
pub fn verify_channel<'a>(channel: &'a [u8]) -> Option<&'a str> {
    match from_utf8(channel) {
        None => None,
        Some(channel) => 
            if valid_channel(channel) {
                Some(channel) 
            } else { None }
    }
}

pub enum Receiver<'a> {
    ChannelName(&'a str),
    NickName(&'a str),
    InvalidReceiver(&'a [u8])
}

/// Validates the raw channel name and converts it into a string. 
pub fn verify_receiver<'a>(recv: &'a [u8]) -> Receiver<'a> {
    match from_utf8(recv) {
        None => InvalidReceiver(recv),
        Some(name) => 
            if valid_channel(name) {
                ChannelName(name)
            } else if valid_nick(name) {
                NickName(name)
            } else { InvalidReceiver(recv) }
    }
}


#[deriving(Hash, PartialEq, Eq)]
pub struct HostMask {
    mask: String
}

impl HostMask {
    pub fn new(mask: String) -> HostMask {
        HostMask {
            mask: mask
        }
    }
    fn matches(&self, hostname: &str) -> bool {
        let mut name_chars = hostname.chars().peekable();
        let mask = self.mask.as_slice();
        for (c, next) in mask.chars().zip({let mut n = mask.chars(); n.next(); n}) {
            match c {
                '*' => { while match name_chars.peek() {
                    Some(&name_cha) => name_cha != next,
                    None => false } { let _ = name_chars.next(); }
                },
                cha => match name_chars.next() {
                    None => return false,
                    Some(name_cha) => if cha != name_cha { return false }
                }
            }
        }
        true
    }
}


#[cfg(test)]
mod tests {
	use super::{valid_nick, valid_channel, HostMask};
	#[test]
	/// Test the nickname validation function
	fn test_nickname_validation() {
		assert!(valid_nick("FooBar123"))
		assert_eq!(valid_nick("FooBar1234"), false)
		assert_eq!(valid_nick("1FooBar12"), false)
	}
	#[test]
	/// Test the nickname validation function
	fn test_channel_name_validation() {
		assert!(valid_channel("#Foobar"))
		assert_eq!(valid_channel("Foobar"), false)
		assert_eq!(valid_channel("#Foo,bar"), false)
		assert_eq!(valid_channel("Foo bar"), false)
	}
    
    #[test]
    /// Test the hostname masks
	fn test_masks() {
		assert!(HostMask::new("*!*@*.com".to_string()).matches("a!b@example.com"))
		assert!(!HostMask::new("*!*@*.com".to_string()).matches("*!*@*.edu"))
		assert!(HostMask::new("*!*@example.com".to_string()).matches("a!b@example.com"))
		assert!(HostMask::new("foo!*@*.com".to_string()).matches("foo!bar@example.com"))
		assert!(!HostMask::new("foo!*@*.com".to_string()).matches("baz!bar@example.com"))
		assert!(HostMask::new("*!bar@*.com".to_string()).matches("foo!bar@example.com"))
		assert!(!HostMask::new("*!bar@*.com".to_string()).matches("foo!baz@example.com"))
	}
    
}
