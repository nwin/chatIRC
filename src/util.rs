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

#[deriving(Clone)]
pub enum Receiver {
    ChannelName(String),
    NickName(String),
    InvalidReceiver(Vec<u8>)
}

/// Validates the raw channel name and converts it into a string. 
pub fn verify_receiver<'a>(recv: &'a [u8]) -> Receiver {
    match from_utf8(recv) {
        None => InvalidReceiver(recv.to_vec()),
        Some(name) => 
            if valid_channel(name) {
                ChannelName(name.to_string())
            } else if valid_nick(name) {
                NickName(name.to_string())
            } else { InvalidReceiver(recv.to_vec()) }
    }
}


#[deriving(Hash, PartialEq, Eq, Clone)]
/// A host mask in the form "*!*@*.*"
pub struct HostMask {
    mask: String
}

impl HostMask {
    pub fn new(mask: String) -> HostMask {
        HostMask {
            mask: mask
        }
    }
    pub fn from_parts(nick: &str, name: &str, host: &str) -> HostMask {
        HostMask {
            mask: format!("{}!{}@{}", nick, name, host)
        }
    }
    /// checks if the host mask matches another mask
    ///
    /// "*!*@*.com" would match "a!b@example.com"
    pub fn matches(&self, mask: &str) -> bool {
        let mut mask_chars = mask.chars().peekable();
        let mut chars = self.mask.as_slice().chars().peekable();
        for c in chars {
            match c {
                '*' => match chars.peek() {
                    // Consume all chars until next match is found
                    Some(next) => while match mask_chars.peek() {
                        Some(mask_cha) => mask_cha != next,
                        None => false } { let _ = mask_chars.next(); },
                    // * at end of the string matches the whole rest
                    None => return true
                },
                cha => match mask_chars.next() {
                    None => return false,
                    Some(mask_cha) => if cha != mask_cha { return false }
                }
            }
        }
        !mask_chars.next().is_some()
    }
    
    /// Returns the hostname
    pub fn host(&self) -> Option<&str> {
        self.mask.as_slice().split('@').last()
    }
    /// Returns the username
    pub fn user(&self) -> Option<&str> {
        self.mask.as_slice().split('@').nth(0).and_then(|v| 
            v.split('!').last()
        )
    }
    /// Returns the nickname
    pub fn nick(&self) -> Option<&str> {
        self.mask.as_slice().split('!').nth(0)
    }
    
    /// Returns a view into the mask
    pub fn as_str(&self) -> &str {
        return self.mask.as_slice()
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
		assert!(!HostMask::new("*!*@*.com".to_string()).matches("*!*@*.cop"))
		assert!(!HostMask::new("*!*@*.com".to_string()).matches("*!*@*.coma"))
		assert!(HostMask::new("*!*@example.com".to_string()).matches("a!b@example.com"))
		assert!(HostMask::new("foo!*@*.com".to_string()).matches("foo!bar@example.com"))
		assert!(!HostMask::new("foo!*@*.com".to_string()).matches("baz!bar@example.com"))
		assert!(HostMask::new("*!bar@*.com".to_string()).matches("foo!bar@example.com"))
		assert!(!HostMask::new("*!bar@*.com".to_string()).matches("foo!baz@example.com"))
	}
    
}
