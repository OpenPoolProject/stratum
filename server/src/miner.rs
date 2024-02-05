use crate::{
    types::{ConnectionID, Difficulties, Difficulty, DifficultySettings, VarDiffBuffer},
    utils, ConfigManager, SessionID,
};
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

//A miner is essentially an individual worker unit. There can be multiple Miners on a single
//connection which is why we needed to break it into these primitives.
#[derive(Debug, Clone)]
pub struct Miner {
    config_manager: ConfigManager,
    shared: Arc<Shared>,
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub(crate) struct Inner {
    pub(crate) worker_id: Uuid,
    pub(crate) sid: SessionID,
    pub(crate) connection_id: ConnectionID,
    pub(crate) client: Option<String>,
    pub(crate) name: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Shared {
    difficulties: Mutex<Difficulties>,
    needs_ban: Mutex<bool>,
    stats: Mutex<MinerStats>,
    var_diff_stats: Mutex<VarDiffStats>,
    difficulty_settings: Mutex<DifficultySettings>,
}

impl Miner {
    #[must_use]
    pub fn new(
        connection_id: ConnectionID,
        worker_id: Uuid,
        sid: SessionID,
        client: Option<String>,
        name: Option<String>,
        config_manager: ConfigManager,
        difficulty: DifficultySettings,
    ) -> Self {
        let now = utils::now();

        let shared = Shared {
            difficulties: Mutex::new(Difficulties::new_only_current(difficulty.default)),
            needs_ban: Mutex::new(false),
            stats: Mutex::new(MinerStats {
                accepted: 0,
                stale: 0,
                rejected: 0,
                last_active: now,
            }),
            var_diff_stats: Mutex::new(VarDiffStats {
                last_timestamp: now,
                last_retarget: config_manager
                    .difficulty_config()
                    .initial_retarget_time(now),
                vardiff_buf: VarDiffBuffer::new(),
                last_retarget_share: 0,
            }),
            difficulty_settings: Mutex::new(difficulty),
        };

        let inner = Inner {
            worker_id,
            sid,
            connection_id,
            client,
            name,
        };

        Miner {
            config_manager,
            shared: Arc::new(shared),
            inner: Arc::new(inner),
        }
    }

    // pub(crate) fn id(&self) -> Uuid {
    //     self.inner.id
    // }

    pub(crate) fn ban(&self) {
        *self.shared.needs_ban.lock() = true;

        //@todo I think we need to disconnect here as well.
    }

    pub fn consider_ban(&self) {
        let stats = self.shared.stats.lock();

        //@note this could possibly possibly possibly overflow - let's just think about that as we
        //move forward.
        let total = stats.accepted + stats.stale + stats.rejected;

        let config = &self.config_manager.current_config().connection;

        if total >= config.check_threshold {
            let percent_bad: f64 = ((stats.stale + stats.rejected) as f64 / total as f64) * 100.0;

            if percent_bad < config.invalid_percent {
                //@todo do we want to reset though?
                //Although if we don't, then this will trigger on every new share after 500.
                //So we could switch it to modulo 500 == 0
                //@todo make this possible. Reset stats to 0.
                // self.stats.lock().await = MinerStats::default();
            } else {
                warn!(
                    id = ?self.inner.connection_id,
                    worker_id = ?self.inner.worker_id,
                    worker = ?self.inner.name,
                    client = ?self.inner.client,
                    "Miner banned. {} out of the last {} shares were invalid",
                    stats.stale + stats.rejected,
                    total
                );
                self.ban();
            }
        }
    }

    #[must_use]
    pub fn difficulties(&self) -> Difficulties {
        self.shared.difficulties.lock().clone()
    }

    //@todo in the future have this accept difficulty, and then we could calculate hashrate here.
    pub fn valid_share(&self) {
        let mut stats = self.shared.stats.lock();
        stats.accepted += 1;
        stats.last_active = utils::now();

        drop(stats);

        self.consider_ban();

        self.retarget();
    }

    pub fn stale_share(&self) {
        let mut stats = self.shared.stats.lock();

        stats.stale += 1;
        stats.last_active = utils::now();

        drop(stats);

        self.consider_ban();

        //@todo see below
        //I don't think we want to retarget on invalid shares, but let's double check later.
        // self.retarget().await;
    }

    pub fn rejected_share(&self) {
        let mut stats = self.shared.stats.lock();

        stats.rejected += 1;
        stats.last_active = utils::now();

        drop(stats);

        self.consider_ban();

        //@todo see below
        //I don't think we want to retarget on invalid shares, but let's double check later.
        // self.retarget().await;
    }

    //@todo note, this only can be sent over ExMessage when it's hit a certain threshold.
    //@todo self.set_difficulty
    //@todo self.set_next_difficulty
    //@todo does this need to return a result? Ideally not, but if we send difficulty, then maybe.
    //@todo see if we can solve a lot of these recasting issues.
    //@todo wrap u64 with a custom difficulty type.
    fn retarget(&self) {
        //This is in milliseconds
        let now = utils::now();
        // let retarget_time = self.config_manager.difficulty_config().retarget_time() * 1000.0;

        let mut difficulties = self.shared.difficulties.lock();
        let mut var_diff_stats = self.shared.var_diff_stats.lock();
        let stats = self.shared.stats.lock();

        let since_last = now - var_diff_stats.last_timestamp;

        var_diff_stats.vardiff_buf.append(since_last);
        var_diff_stats.last_timestamp = now;

        //@todo review this code, see if we can make this easier.
        if !(((stats.accepted - var_diff_stats.last_retarget_share)
            >= self
                .config_manager
                .difficulty_config()
                .retarget_share_amount)
            || (now - var_diff_stats.last_retarget)
                >= (self.config_manager.difficulty_config().retarget_time as u128 * 1000))
        {
            return;
        }

        // dbg!(stats.accepted);
        //
        // dbg!(var_diff_stats.last_retarget_share);
        // dbg!(
        //     self.config_manager
        //         .difficulty_config()
        //         .retarget_share_amount
        // );

        var_diff_stats.last_retarget = now;
        var_diff_stats.last_retarget_share = stats.accepted;

        // let variance = self.options.target_time * (self.options.variance_percent as f64 / 100.0);
        // let time_min = self.options.target_time as f64 * 0.40;
        // let time_max = self.options.target_time as f64 * 1.40;

        //This average is in milliseconds
        let avg = var_diff_stats.vardiff_buf.avg();

        // debug!(average = ?avg);
        // dbg!(avg);

        if avg <= 0.0 {
            return;
        }

        let mut new_diff;

        //@todo figure out what else needs to come from config here, and comment out this function.
        let target_time = self.config_manager.difficulty_config().target_time as f64 * 1000.0;

        if avg > target_time {
            //@todo this needs to just be target_time since we multiplied it above.
            if (avg / self.config_manager.difficulty_config().target_time as f64) <= 1.5 {
                return;
            }
            new_diff = difficulties.current().as_u64() / 2;
        } else if (avg / target_time) >= 0.7 {
            return;
        } else {
            new_diff = difficulties.current().as_u64() * 2;
        }

        new_diff = new_diff.clamp(
            self.shared.difficulty_settings.lock().minimum.as_u64(),
            self.config_manager.difficulty_config().maximum_difficulty,
        );

        if new_diff != difficulties.current().as_u64() {
            difficulties.update_next(Difficulty::from(new_diff));
            var_diff_stats.vardiff_buf.reset();
        }
    }

    #[must_use]
    pub fn update_difficulty(&self) -> Option<Difficulty> {
        let mut difficulties = self.shared.difficulties.lock();

        difficulties.shift()
    }

    pub fn set_difficulty(&self, difficulty: Difficulty) {
        let mut difficulties = self.shared.difficulties.lock();

        difficulties.set_and_shift(difficulty);
    }

    #[must_use]
    pub fn connection_id(&self) -> ConnectionID {
        self.inner.connection_id.clone()
    }

    #[must_use]
    pub fn worker_id(&self) -> Uuid {
        self.inner.worker_id
    }

    #[must_use]
    pub fn session_id(&self) -> SessionID {
        self.inner.sid
    }
}

//@todo either wrap miner in a folder, or move these both to types
#[derive(Debug, Clone)]
pub struct MinerStats {
    accepted: u64,
    stale: u64,
    rejected: u64,
    last_active: u128,
}

#[derive(Debug)]
pub struct VarDiffStats {
    last_timestamp: u128,
    last_retarget_share: u64,
    last_retarget: u128,
    vardiff_buf: VarDiffBuffer,
}

#[cfg(test)]
mod test {
    use std::thread::sleep;

    use super::*;
    use crate::Config;

    #[test]
    fn test_valid_share() {
        let connection_id = ConnectionID::new();
        let worker_id = Uuid::new_v4();
        let session_id = SessionID::from(1);

        let config = Config::default();
        let config_manager = ConfigManager::new(config.clone());

        let diff_settings = DifficultySettings {
            default: Difficulty::from(config.difficulty.initial_difficulty),
            minimum: Difficulty::from(config.difficulty.minimum_difficulty),
        };
        let miner = Miner::new(
            connection_id,
            worker_id,
            session_id,
            None,
            None,
            config_manager,
            diff_settings,
        );

        miner.valid_share();

        for _ in 0..100 {
            miner.valid_share();
            sleep(std::time::Duration::from_millis(50));
        }

        let new_diff = miner.update_difficulty();
        assert!(new_diff.is_some());

        for _ in 0..100 {
            miner.valid_share();
        }

        let new_diff = miner.update_difficulty();
        assert!(new_diff.is_some());

        //@todo we need some actual result here lol
    }

    #[test]
    fn test_retarget() {
        let connection_id = ConnectionID::new();
        let worker_id = Uuid::new_v4();
        let session_id = SessionID::from(1);

        let config = Config::default();
        let config_manager = ConfigManager::new(config.clone());

        let diff_settings = DifficultySettings {
            default: Difficulty::from(config.difficulty.initial_difficulty),
            minimum: Difficulty::from(config.difficulty.minimum_difficulty),
        };

        let miner = Miner::new(
            connection_id,
            worker_id,
            session_id,
            None,
            None,
            config_manager,
            diff_settings,
        );

        // OK what do we need to test here....
        // 1. We need to solve the issue with why difficulty is flucuating so much with the single
        //    miner that is on the pool right now from CLSK.
        //
        //    Scenario:
        //    The current miner should be at roughly 120 TH/s
        //    Which would equate to 120000000000000 (hashrate) = 4294967296 (multiplier) * effort /
        //    time
        //
        //    For 1m (retarget interval) 120000000000000 * 60 / 4294967296 = effort for the whole
        //    minute. So 1,676,380.6343078613. Cleaning that up, it's 1676360 If we divide that
        //    across our various difficulty levels:
        //
        //    1676360 / 16384 ~= 102 shares per minute
        //    1676360 / 32768 ~= 51 shares per minute
        //    1676360 / 65536 ~= 25 shares per minute
        //    1676360 / 131072  ~= 12.7  shares per minute
        //    1676360 / 262144 ~=  6.4 shares per minute
        //    1676360 / 524288 ~=  3.2 shares per minute
        //

        dbg!(miner.difficulties());

        miner.valid_share();

        for _ in 0..100 {
            miner.valid_share();
            sleep(std::time::Duration::from_millis(50));
        }

        dbg!(miner.difficulties());

        let new_diff = miner.update_difficulty();
        assert!(new_diff.is_some());

        dbg!(miner.difficulties());

        for _ in 0..100 {
            miner.valid_share();
        }

        dbg!(miner.difficulties());

        let new_diff = miner.update_difficulty();
        assert!(new_diff.is_some());

        dbg!(miner.difficulties());
    }
}
