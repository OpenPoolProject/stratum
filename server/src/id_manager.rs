use crate::{Error, Result, SessionID};
use bit_set::BitSet;
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::{info, warn};

const MAX_SESSION_INDEX_SERVER: u32 = 0x00FF_FFFE;

#[derive(Clone)]
pub struct IDManager {
    inner: Arc<Inner>,
}

struct Inner {
    server_id: u8,
    //State can mutate
    state: Mutex<SessionIDInfo>,
}

#[derive(Debug)]
struct SessionIDInfo {
    count: u32,
    idx: u32,
    session_ids: BitSet,
}

impl IDManager {
    pub fn new(server_id: u8) -> Self {
        IDManager {
            inner: Arc::new(Inner {
                server_id,
                state: Mutex::new(SessionIDInfo {
                    count: 0,
                    idx: 0,
                    session_ids: BitSet::new(),
                }),
            }),
        }
    }

    fn is_full(&self) -> bool {
        self.inner.state.lock().count > MAX_SESSION_INDEX_SERVER
    }

    pub fn allocate_session_id(&self) -> Result<SessionID> {
        if self.is_full() {
            return Err(Error::SessionIDsExhausted);
        }

        let mut state = self.inner.state.lock();

        let mut idx = state.idx;
        while state.session_ids.contains(idx as usize) {
            idx += 1;

            if idx > MAX_SESSION_INDEX_SERVER {
                idx = 0;
            }
        }

        state.session_ids.insert(idx as usize);
        state.count += 1;
        state.idx = idx;

        let count = state.count;
        drop(state);

        if count % 250 == 0 {
            info!("Currently allocated {count}/{MAX_SESSION_INDEX_SERVER} session IDs.");
        }

        if (f64::from(count) / f64::from(MAX_SESSION_INDEX_SERVER)) > 0.7 {
            warn!(
                "More than 70% of Session IDs allocated. Only {} remaining",
                MAX_SESSION_INDEX_SERVER - count
            );
        }

        let session_id: u32 = (u32::from(self.inner.server_id) << 24) | idx;

        Ok(session_id.into())
    }

    pub fn remove_session_id(&self, session_id: SessionID) {
        let idx = session_id.as_u32() & 0x00FF_FFFF;

        let mut state = self.inner.state.lock();

        state.session_ids.remove(idx as usize);
        state.count -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::assert_ok;

    #[cfg_attr(coverage_nightly, no_coverage)]
    #[tokio::test]
    async fn idx_allocations() {
        let id_manager = IDManager::new(0);

        let id = assert_ok!(id_manager.allocate_session_id());

        assert_eq!(id, SessionID::from(0));

        for _ in 0..MAX_SESSION_INDEX_SERVER - 1 {
            assert_ok!(id_manager.allocate_session_id());
        }

        let last_id = assert_ok!(id_manager.allocate_session_id());

        assert_eq!(last_id, SessionID::from(MAX_SESSION_INDEX_SERVER));

        id_manager.remove_session_id(SessionID::from(0));

        let rolled_id = assert_ok!(id_manager.allocate_session_id());

        assert_eq!(rolled_id, SessionID::from(0));
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    #[tokio::test]
    async fn basic_idx_allocations_with_prefix() {
        let id_manager = IDManager::new(9);

        let id = assert_ok!(id_manager.allocate_session_id());

        assert_eq!(id, SessionID::from(0x0900_0000));

        let id = assert_ok!(id_manager.allocate_session_id());

        assert_eq!(id, SessionID::from(0x0900_0001));
    }
}
