use serde::{ser::SerializeTuple, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug)]
pub enum StratumError {
    Unknown(i32, String),
    StaleShare,
    DuplicateShare,
    LowDifficultyShare,
    Unauthorized,
    NotSubscribed,
    NodeSyncing,
    Internal,
    InvalidHeaderData,
    TimeOutOfRange,
    InvalidExtraNonce,
}

impl StratumError {
    pub fn get_error_code(&self) -> i32 {
        match *self {
            StratumError::Unknown(code, _) => code,
            StratumError::StaleShare => 21,
            StratumError::DuplicateShare => 22,
            StratumError::LowDifficultyShare => 23,
            StratumError::Unauthorized => 24,
            StratumError::NotSubscribed => 25,
            StratumError::NodeSyncing => 26,
            StratumError::Internal => 27,
            StratumError::InvalidHeaderData => 28,
            StratumError::TimeOutOfRange => 29,
            StratumError::InvalidExtraNonce => 30,
        }
    }
}

impl fmt::Display for StratumError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StratumError::Unknown(_, ref e) => write!(f, "Error: {}", e),
            StratumError::StaleShare => write!(f, "Share submitted for old job"),
            StratumError::DuplicateShare => write!(f, "Share already submitted"),
            StratumError::LowDifficultyShare => write!(f, "Share too low difficulty"),
            StratumError::Unauthorized => write!(f, "Miner is not authorized"),
            StratumError::NotSubscribed => write!(f, "Miner is not subscribed"),
            StratumError::NodeSyncing => write!(f, "Node is currently syncing. Please wait."),
            StratumError::Internal => write!(f, "Internal Error. Please wait."),
            StratumError::InvalidHeaderData => write!(f, "Header data in share is invalid."),
            StratumError::TimeOutOfRange => {
                write!(f, "Timestamp is outside of the acceptable range.")
            }
            StratumError::InvalidExtraNonce => write!(f, "Extranonce does not match provided."),
        }
    }
}

impl Serialize for StratumError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tup = serializer.serialize_tuple(2)?;
        tup.serialize_element(&self.get_error_code())?;
        tup.serialize_element(&self.to_string())?;
        tup.end()
    }
}

impl<'de> Deserialize<'de> for StratumError {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|(c, n)| match c {
            21 => StratumError::StaleShare,
            22 => StratumError::DuplicateShare,
            23 => StratumError::LowDifficultyShare,
            24 => StratumError::Unauthorized,
            25 => StratumError::NotSubscribed,
            26 => StratumError::NodeSyncing,
            27 => StratumError::Internal,
            28 => StratumError::InvalidHeaderData,
            29 => StratumError::TimeOutOfRange,
            30 => StratumError::InvalidExtraNonce,
            _ => StratumError::Unknown(c, n),
        })
    }
}
