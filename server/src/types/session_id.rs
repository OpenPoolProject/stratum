use std::fmt::{self, Display};

#[derive(Clone, PartialEq, Eq, Hash, Debug, Copy, Default)]
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

//@todo needs testing
impl Display for SessionID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:x}{:x}{:x}{:x} ({})",
            self.0[0],
            self.0[1],
            self.0[2],
            self.0[3],
            self.as_u32()
        )
    }
}

//@todo we need lots of testing here.
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_session_id_u32_conversion() {
        let id: u32 = 280;

        let session_id = SessionID::from(id);

        assert_eq!(id, session_id.as_u32());
    }
}
