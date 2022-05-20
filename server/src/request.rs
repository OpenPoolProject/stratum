use crate::types::{ExMessageGeneric, GlobalVars, MessageValue, ID};

pub struct StratumRequest<State> {
    pub(crate) state: State,
    pub(crate) values: MessageValue,
    pub(crate) global_vars: GlobalVars,
}

impl<State> StratumRequest<State> {
    ///  Access application scoped state.
    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn global_vars(&self) -> &GlobalVars {
        &self.global_vars
    }

    pub fn get_ex_message(&self) -> Option<ExMessageGeneric> {
        match &self.values {
            MessageValue::StratumV1(_) => None,
            MessageValue::ExMessage(msg) => Some(msg.clone()),
        }
    }

    pub fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        name: &str,
    ) -> Result<T, serde_json::Error> {
        match &self.values {
            MessageValue::StratumV1(params) => {
                let params = params
                    .get(name)
                    .ok_or_else(|| serde::de::Error::custom(format!("expected {}", name)))?
                    .clone();

                Ok(serde_json::from_value(params)?)
            }
            MessageValue::ExMessage(_) => Err(serde::de::Error::custom(
                "wrong message type request".to_string(),
            )),
        }
    }

    pub fn get_id(&self) -> Result<ID, serde_json::Error> {
        match &self.values {
            MessageValue::StratumV1(params) => {
                let params = params
                    .get("id")
                    .ok_or_else(|| serde::de::Error::custom(format!("expected {}", "id")))?
                    .clone();

                Ok(serde_json::from_value(params)?)
            }
            MessageValue::ExMessage(_) => Err(serde::de::Error::custom(
                "wrong message type request".to_string(),
            )),
        }
    }
}
