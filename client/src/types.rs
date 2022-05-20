use stratum_types::traits::StratumParams;

#[derive(PartialEq, Debug)]
pub enum State {
    Connected,
    Disconnect,
}

#[derive(PartialEq, Debug)]
pub enum Event {
    NewWork,
    ShareAccepted,
    ShareRejected,
}

//@todo this might be able to be private
#[derive(PartialEq, Debug)]
pub enum Message<SP: StratumParams> {
    Submit(SP::Submit),
}

//@todo stats
//How many accepted shares, rejected, hashrate etc.
//
//Stratum User {
// credentials:
// authorized:
//
//}
//
//.is_authroized, etc.
