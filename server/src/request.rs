use crate::{
    types::{GlobalVars, ID},
    Frame,
};

pub struct StratumRequest<State> {
    pub(crate) state: State,
    pub(crate) values: Frame,
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

    // pub fn get_ex_message(&self) -> Option<ExMessageGeneric> {
    //     match &self.values {
    //         MessageValue::StratumV1(_) => None,
    //         MessageValue::ExMessage(msg) => Some(msg.clone()),
    //     }
    // }

    pub fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        _name: &str,
    ) -> Result<T, serde_json::Error> {
        match &self.values {
            Frame::V1(request) => Ok(serde_json::from_value(request.params.clone())?),
            // MessageValue::ExMessage(_) => Err(serde::de::Error::custom(
            //     "wrong message type request".to_string(),
            // )),
        }
    }

    pub fn get_id(&self) -> Result<ID, serde_json::Error> {
        match &self.values {
            Frame::V1(request) => Ok(request.id.clone()),
            // MessageValue::ExMessage(_) => Err(serde::de::Error::custom(
            //     "wrong message type request".to_string(),
            // )),
        }
    }
}
