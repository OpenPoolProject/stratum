use crate::{
    types::{
        BanStats, ConnectionID, Difficulties, Difficulty, DifficultySettings, MinerStats,
        VarDiffBuffer, VarDiffStats,
    },
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

//@todo random reminder for myself here -> Would it be more efficient to wrap this entire struct in
//a Mutex? My guess is no... But let's jsut re-review.
#[derive(Debug)]
pub(crate) struct Shared {
    difficulties: Mutex<Difficulties>,
    ban_stats: Mutex<BanStats>,
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
            ban_stats: Mutex::new(BanStats {
                last_ban_check_share: 0,
                needs_ban: false,
            }),
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

    pub(crate) fn ban(&self) {
        //@todo I now set needs ban in consider ban so that I don't have to drop the Mutex. In
        //here, we just really want to either contact the session, or disconnect the miner
        // let mut ban_stats = self.shared.ban_stats.lock();
        // ban_stats.needs_ban = true;

        //@todo I think we need to disconnect here as well.
        //@todo ok as far as I can tell, a ban will *not* lead to a miner being disconnected from
        //the pool right now.
        //
        //Couple of thoughts.
        //
        //1. Let's go with the most complex scenario. Single connection with multiple miners. We
        //   want to disconnect this miner ONLY - How we do that will be complex because we will
        //   want the other miners in this connection to still work.
        //
        //2. If it is a single miner and single connection, I think this becomes a bit easier. We
        //   probably need a check in the tcp loop where we see if there are still remaining miners
        //   on a connection? Or we just send this individual miner a ban signal (tbd)
    }

    pub fn needs_ban(&self) -> bool {
        self.shared.ban_stats.lock().needs_ban
    }

    pub fn consider_ban(&self) {
        let stats = self.shared.stats.lock();
        let mut ban_stats = self.shared.ban_stats.lock();

        //@note this could possibly possibly possibly overflow - let's just think about that as we
        //move forward.
        //@todo I think this needs to be moved to like how retarget it used. Last ban check etc.
        let total = stats.accepted + stats.stale + stats.rejected;

        let config = &self.config_manager.connection_config();

        if total - ban_stats.last_ban_check_share >= config.check_threshold {
            let percent_bad: f64 = ((stats.stale + stats.rejected) as f64 / total as f64) * 100.0;

            ban_stats.last_ban_check_share = total;

            if percent_bad < config.invalid_percent {
                //Does not need a ban
                //@todo not sure if this is a good idea. Basically what we are saying is if the
                //miner doesn't get banned in time, they can redeem themselves.
                ban_stats.needs_ban = false;
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
                ban_stats.needs_ban = true;

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

        self.retarget();
    }

    pub fn rejected_share(&self) {
        let mut stats = self.shared.stats.lock();

        stats.rejected += 1;
        stats.last_active = utils::now();

        drop(stats);

        self.consider_ban();

        self.retarget();
    }

    fn retarget(&self) {
        //This is in milliseconds
        let now = utils::now();
        let difficulty_config = self.config_manager.difficulty_config();

        //@todo why not just store this as u128... Let's do that now.
        let retarget_time = difficulty_config.retarget_time as u128 * 1000;
        let retarget_share_amount = difficulty_config.retarget_share_amount;
        //@todo see above, should we just store this as f64 * 1000.0?

        let mut difficulties = self.shared.difficulties.lock();
        let mut var_diff_stats = self.shared.var_diff_stats.lock();
        let stats = self.shared.stats.lock();

        let since_last = now - var_diff_stats.last_timestamp;

        var_diff_stats.vardiff_buf.append(since_last);
        var_diff_stats.last_timestamp = now;

        //@todo add this as a function on miner stats please.
        let total = stats.accepted + stats.rejected + stats.stale;

        //This is the amoutn of shares we've added since the last retarget
        let share_difference = total - var_diff_stats.last_retarget_share;
        let time_difference = now - var_diff_stats.last_retarget;

        if !((share_difference >= retarget_share_amount) || time_difference >= retarget_time) {
            return;
        }

        var_diff_stats.last_retarget = now;
        var_diff_stats.last_retarget_share = stats.accepted;

        //This average is in milliseconds
        let avg = var_diff_stats.vardiff_buf.avg();

        if avg <= 0.0 {
            return;
        }

        let mut new_diff;

        let target_time = difficulty_config.target_time as f64 * 1000.0;

        //@todo these variances should probs come from config.
        if avg > target_time {
            //@todo this needs to just be target_time since we multiplied it above.
            if (avg / target_time) <= 1.5 {
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
            difficulty_config.maximum_difficulty,
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
    fn test_ban() {
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

        //Note Check threshold for miner bans is 500.
        for _ in 0..500 {
            miner.stale_share();
        }

        assert!(miner.needs_ban());
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
        //    miner that is on the pool right now.
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
