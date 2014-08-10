use std::collections::{HashSet};


/// Enumeration of possible channel modes
/// as of http://tools.ietf.org/html/rfc2811#section-4
#[deriving(FromPrimitive, Show, Clone, Hash, PartialEq, Eq)]
pub enum ChannelMode {
    /// give "channel creator" status
    ChannelCreator = b'O' as int,
    /// give/take channel operator privilege
    OperatorPrivilege = b'o' as int,
    /// give/take the voice privilege
    VoicePrivilege = b'v' as int,
    /// toggle the anonymous channel flag
    AnonChannel = b'a' as int,
    /// toggle the invite-only channel flag
    InviteOnly = b'i' as int,
    /// toggle the moderated channel
    Moderated = b'm' as int,
    /// toggle the no messages to channel from clients on the
    /// outside
    MemberOnly = b'n' as int,
    /// toggle the quiet channel flag
    Quiet = b'q' as int,
    /// toggle the private channel flag
    Private = b'p' as int,
    /// toggle the secret channel flag
    Secret = b's' as int,
    /// toggle the server reop channel flag
    ReOpFlag = b'r' as int,
    /// toggle the topic settable by channel operator only flag
    TopicProtect = b't' as int,
    /// set/remove the channel key (password)
    ChannelKey = b'k' as int,
    /// set/remove the user limit to channel
    UserLimit = b'l' as int,
    /// set/remove ban mask to keep users out
    BanMask = b'b' as int,
    /// set/remove an exception mask to override a ban mask
    ExceptionMask = b'e' as int,
    /// set/remove an invitation mask to automatically override
    /// the invite-only flag
    InvitationMask = b'I' as int
}

// Actions which determine what to do with a mode
#[deriving(PartialEq, Eq, Show)]
pub enum Action {
    // Add a flag
    Add,
    // Remove a flag
    Remove,
    // Show the flag
    Show
}

impl ChannelMode {
    fn has_parameter(&self) -> bool {
        match *self {
            ChannelKey | UserLimit | BanMask
            | ExceptionMask | InvitationMask => true,
            _ => false
        }
    }
}

/// Parses the channel modes
///
/// According to [RFC 2812] (http://tools.ietf.org/html/rfc2812#section-3.2.3) the
/// syntax for setting modes is:
/// ```
///    Command: MODE
/// Parameters: <channel> *( ( "-" / "+" ) *<modes> *<modeparams> )
/// ```
///
/// Additionally an example is given
///
/// ```
/// MODE &oulu +b *!*@*.edu +e *!*@*.bu.edu
///                                 ; Command to prevent any user from a
///                                 hostname matching *.edu from joining,
///                                 except if matching *.bu.edu
/// 
/// MODE #bu +be *!*@*.edu *!*@*.bu.edu
///                                 ; Comment to prevent any user from a
///                                 hostname matching *.edu from joining,
///                                 except if matching *.bu.edu
/// ```
/// 
/// 
pub fn modes_do(slice: &[&[u8]], block: |Action, ChannelMode, Option<&[u8]>|) {
    let mut current = slice;
    loop {
        // Bug: no +/- asking for modes
        let (action, offset) = match current[0][0] {
            b'+' => (Add, 1),
            b'-' => (Remove, 1),
            _ => (Show, 0)
            
        };
        for mode in current[0].slice_from(offset).iter().filter_map( |&v| {
            let m: Option<ChannelMode> = FromPrimitive::from_u8(v); m
        }) {
            let param = if mode.has_parameter() && action != Show {
                let param = current.get(1).map(|v| *v);
                if current.len() > 1 {
                    current = current.slice_from(1);
                } else { current = &[]; }
                param
            } else {
                None
            };
            block(action, mode, param);
        }
        if current.len() > 1 {
            current = current.slice_from(1);
        } else { break }
    }
}

/// List of channel modes / member flags
pub type Flags = HashSet<ChannelMode>;


#[cfg(test)]
mod tests {
	use super::{modes_do, BanMask, ExceptionMask, Add, Show};
    use msg::{RawMessage};
	/// Tests the mode parser
    
    
    
	#[test]
	fn test_mode_parser() {
        let msgs = [
            b"MODE &oulu +b *!*@*.edu +e *!*@*.bu.edu",
            b"MODE #bu +be *!*@*.edu *!*@*.bu.edu",
            b"MODE #bu b",
            //b"MODE #bu /i", // Invalid mode should be skipped
            b"MODE #bu +g", // Invalid mode should be skipped
        ];
        let modes = [
            vec![(Add, BanMask, Some(b"*!*@*.edu")),
            (Add, ExceptionMask, Some(b"*!*@*.bu.edu"))],
            vec![(Add, BanMask, Some(b"*!*@*.edu")),
            (Add, ExceptionMask, Some(b"*!*@*.bu.edu"))],
            vec![(Show, BanMask, None)],
            //Vec::new(),
            Vec::new(),
        ];
        for (msg, modes) in msgs.iter().zip(modes.iter()) {
            let m = RawMessage::parse(*msg).unwrap();
            let mut mode_iter = modes.iter();
            modes_do(m.params().slice_from(1), |set, mode, parameter| {
                let (set_, mode_, parameter_) = *mode_iter.next().unwrap();
                assert_eq!(set_, set);
                assert_eq!(mode_, mode);
                assert_eq!(parameter_.to_string(), parameter.to_string());
            })
        }
	}
}