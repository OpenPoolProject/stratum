pub mod error;
pub mod miner;
pub mod params;
pub mod stratum_error;
pub mod traits;
use crate::params::{Params, Results};
pub use crate::stratum_error::StratumError;
use crate::traits::{PoolParams, StratumParams};
pub use error::Error;
pub use miner::{MinerAuth, MinerInfo, MinerJobStats};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::marker::PhantomData;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum StratumPacket<PP, SP>
where
    PP: PoolParams,
    SP: StratumParams,
{
    Request(Request<PP, SP>),
    Response(Response<PP>),
}

#[derive(Serialize, Deserialize)]
pub struct Response<PP>
where
    PP: PoolParams,
{
    pub id: ID,
    #[serde(skip_serializing_if = "StratumMethod::is_classic")]
    pub method: StratumMethod,
    pub result: Option<Results<PP>>,
    pub error: Option<StratumError>,
}

// #[derive(Serialize, Deserialize)]
#[derive(Serialize)]
pub struct Request<PP, SP>
where
    PP: PoolParams,
    SP: StratumParams,
{
    pub id: ID,
    pub method: StratumMethod,
    pub params: Params<PP, SP>,
}

impl<'de, PP, SP> Deserialize<'de> for Request<PP, SP>
where
    PP: PoolParams + Deserialize<'de>,
    SP: StratumParams + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Id,
            Method,
            Params,
            JsonRPC,
        };

        // This part could also be generated independently by:
        //
        //    #[derive(Deserialize)]
        //    #[serde(field_identifier, rename_all = "lowercase")]
        //    enum Field { Secs, Nanos }
        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`id` or `method` or params")
                    }

                    fn visit_str<E>(self, value: &str) -> std::result::Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "id" => Ok(Field::Id),
                            "method" => Ok(Field::Method),
                            "params" => Ok(Field::Params),
                            "jsonrpc" => Ok(Field::JsonRPC),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        // struct RequestVisitor;
        struct RequestVisitor<SP, PP>
        where
            PP: PoolParams,
            SP: StratumParams,
        {
            marker: PhantomData<fn() -> Request<PP, SP>>,
        };

        impl<SP, PP> RequestVisitor<SP, PP>
        where
            PP: PoolParams,
            SP: StratumParams,
        {
            fn new() -> Self {
                RequestVisitor {
                    marker: PhantomData,
                }
            }
        }

        impl<'de, SP, PP> Visitor<'de> for RequestVisitor<SP, PP>
        where
            PP: PoolParams + Deserialize<'de>,
            SP: StratumParams + Deserialize<'de>,
        {
            type Value = Request<PP, SP>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Request")
            }

            //     fn visit_seq<V>(self, mut seq: V) -> Result<Request, V::Error>
            //     where
            //         V: SeqAccess<'de>,
            //     {
            //         let secs = seq
            //             .next_element()?
            //             .ok_or_else(|| de::Error::invalid_length(0, &self))?;
            //         let nanos = seq
            //             .next_element()?
            //             .ok_or_else(|| de::Error::invalid_length(1, &self))?;
            //         Ok(Duration::new(secs, nanos))
            //     }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Request<PP, SP>, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut id = None;
                let mut method = None;
                // let mut jsonrpc = None;
                let mut params = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Id => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        Field::Method => {
                            if method.is_some() {
                                return Err(de::Error::duplicate_field("method"));
                            }
                            method = Some(map.next_value()?);
                        }
                        Field::JsonRPC => {
                            //Do nothing but don't error out.
                        }
                        Field::Params => {
                            if params.is_some() {
                                return Err(de::Error::duplicate_field("params"));
                            }
                            if let Some(temp_method) = &method {
                                if &StratumMethod::ClassicSubscribe == temp_method
                                    || &StratumMethod::Subscribe == temp_method
                                {
                                    let temp: PP::Subscribe = map.next_value()?;
                                    params = Some(Params::Subscribe(temp));
                                // params: PP::Subscribe = Some(map.next_value()?);
                                } else {
                                    params = Some(map.next_value()?);
                                }
                            } else {
                                params = Some(map.next_value()?);
                            }
                        }
                    }
                }

                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
                let method = method.ok_or_else(|| de::Error::missing_field("method"))?;
                let params = params.ok_or_else(|| de::Error::missing_field("params"))?;

                Ok(Request { id, method, params })
            }
        }

        const FIELDS: &'static [&'static str] = &["id", "method", "params", "jsonrpc"];
        deserializer.deserialize_struct("Request", FIELDS, RequestVisitor::new())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ID {
    Num(u64),
    Str(String),
    Null(serde_json::Value),
}

// impl Serialize for ID {
//     fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let id = match *self {
//             ID::Num(num) => num.to_string(),
//             ID::Str(ref string) => string.clone(),
//         };

//         serializer.serialize_str(&id)
//     }
// }

impl std::fmt::Display for ID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ID::Num(ref e) => write!(f, "{}", e),
            ID::Str(ref e) => write!(f, "{}", e),
            ID::Null(ref _e) => write!(f, "null"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum StratumMethod {
    //Sending and receiving
    Authorize,
    ClassicAuthorize,
    Submit,
    ClassicSubmit,
    Subscribe,
    ClassicSubscribe,
    Notify,
    ClassicNotify,
    SetDifficulty,
    ClassicSetDifficulty,

    //Future methods potentially not implemented yet.
    Unknown(String),
}

impl StratumMethod {
    pub fn is_classic(&self) -> bool {
        match self {
            StratumMethod::ClassicAuthorize => true,
            StratumMethod::ClassicSubmit => true,
            StratumMethod::ClassicNotify => true,
            StratumMethod::ClassicSetDifficulty => true,
            _ => false,
        }
    }
}

impl Serialize for StratumMethod {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match *self {
            StratumMethod::Authorize => "authorize",
            StratumMethod::ClassicAuthorize => "mining.authorize",
            StratumMethod::Submit => "submit",
            StratumMethod::ClassicSubmit => "mining.submit",
            StratumMethod::Subscribe => "subscribe",
            StratumMethod::ClassicSubscribe => "mining.subscribe",
            StratumMethod::Notify => "notify",
            StratumMethod::ClassicNotify => "mining.notify",
            StratumMethod::SetDifficulty => "set_difficulty",
            StratumMethod::ClassicSetDifficulty => "mining.set_difficulty",
            StratumMethod::Unknown(ref s) => s,
        })
    }
}

impl<'de> Deserialize<'de> for StratumMethod {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "authorize" => StratumMethod::Authorize,
            "mining.authorize" => StratumMethod::ClassicAuthorize,
            "submit" => StratumMethod::Submit,
            "mining.submit" => StratumMethod::ClassicSubmit,
            "subscribe" => StratumMethod::Subscribe,
            "mining.subscribe" => StratumMethod::ClassicSubscribe,
            "notify" => StratumMethod::Notify,
            "mining.notify" => StratumMethod::ClassicNotify,
            "set_difficulty" => StratumMethod::SetDifficulty,
            "mining.set_difficulty" => StratumMethod::ClassicSetDifficulty,
            _ => StratumMethod::Unknown(s),
        })
    }
}
