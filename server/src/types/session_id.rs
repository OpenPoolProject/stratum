use std::fmt::{self, Debug, Display};

#[derive(Clone, PartialEq, Eq, Hash, Copy, Default)]
pub struct SessionID([u8; 4]);

impl SessionID {
    #[must_use]
    pub fn as_u32(&self) -> u32 {
        u32::from_le_bytes(self.0)
    }
}

impl From<u32> for SessionID {
    fn from(value: u32) -> Self {
        SessionID(value.to_le_bytes())
    }
}

impl Display for SessionID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}",
            self.0[3], self.0[2], self.0[1], self.0[0],
        )
    }
}

impl Debug for SessionID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:#04x}{:02x}{:02x}{:02x} ({})",
            self.0[3],
            self.0[2],
            self.0[1],
            self.0[0],
            self.as_u32()
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_session_id_u32_conversion() {
        let id: u32 = 280;

        let session_id = SessionID::from(id);

        assert_eq!(id, session_id.as_u32());
    }

    #[test]
    fn test_session_id_display() {
        assert_eq!(format!("{}", SessionID::from(200)), "000000c8");
        assert_eq!(format!("{}", SessionID::from(1)), "00000001");
        assert_eq!(format!("{}", SessionID::from(2)), "00000002");
        assert_eq!(format!("{}", SessionID::from(256)), "00000100");
        assert_eq!(format!("{}", SessionID::from(65536)), "00010000");
        assert_eq!(format!("{}", SessionID::from(16_777_216)), "01000000");
        assert_eq!(format!("{}", SessionID::from(16_777_217)), "01000001");
    }

    #[test]
    fn test_session_id_debug() {
        assert_eq!(format!("{:?}", SessionID::from(200)), "0x000000c8 (200)");
        assert_eq!(format!("{:?}", SessionID::from(1)), "0x00000001 (1)");
        assert_eq!(format!("{:?}", SessionID::from(2)), "0x00000002 (2)");
        assert_eq!(format!("{:?}", SessionID::from(256)), "0x00000100 (256)");
        assert_eq!(
            format!("{:?}", SessionID::from(65536)),
            "0x00010000 (65536)"
        );
        assert_eq!(
            format!("{:?}", SessionID::from(16_777_216)),
            "0x01000000 (16777216)"
        );
        assert_eq!(
            format!("{:?}", SessionID::from(16_777_217)),
            "0x01000001 (16777217)"
        );
    }
}
