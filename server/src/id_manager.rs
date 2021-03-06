use async_std::sync::Mutex;
use bit_set::BitSet;

const MAX_SESSION_INDEX_SERVER: u32 = 0x00FFFFFE;

pub struct IDManager {
    pub server_id: u8,
    session_id_info: Mutex<SessionIDInfo>,
}

struct SessionIDInfo {
    count: u32,
    idx: u32,
    session_ids: BitSet,
}

impl IDManager {
    pub fn new(server_id: u8) -> Self {
        IDManager {
            server_id,
            session_id_info: Mutex::new(SessionIDInfo {
                count: 0,
                idx: 0,
                session_ids: BitSet::new(),
            }),
        }
    }

    async fn is_full(&self) -> bool {
        self.session_id_info.lock().await.count > MAX_SESSION_INDEX_SERVER
    }

    pub async fn allocate_session_id(&self) -> Option<u32> {
        if self.is_full().await {
            return None;
        }

        let mut info = self.session_id_info.lock().await;

        while info.session_ids.contains(info.idx as usize) {
            info.idx += 1;

            if info.idx > MAX_SESSION_INDEX_SERVER {
                info.idx = 0;
            }
        }

        let idx = info.idx;

        info.session_ids.insert(idx as usize);
        info.count += 1;

        let session_id: u32 = ((self.server_id as u32) << 24) | info.idx;

        Some(session_id)
    }

    pub async fn remove_session_id(&self, session_id: u32) {
        let idx = session_id & 0x00FFFFFF;

        let mut info = self.session_id_info.lock().await;

        info.session_ids.remove(idx as usize);
        info.count -= 1;
    }
}
