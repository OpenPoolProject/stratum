use crate::{
    types::{Difficulties, VarDiffBuffer},
    utils, ConfigManager,
};
use extended_primitives::Buffer;
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
    pub(crate) id: Uuid,
    pub(crate) _sid: Buffer,
    pub(crate) _client: Option<String>,
    pub(crate) _name: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Shared {
    difficulties: Mutex<Difficulties>,
    needs_ban: Mutex<bool>,
    stats: Mutex<MinerStats>,
    var_diff_stats: Mutex<VarDiffStats>,
}

impl Miner {
    #[must_use]
    pub fn new(
        id: Uuid,
        client: Option<String>,
        name: Option<String>,
        sid: Buffer,
        config_manager: ConfigManager,
        difficulty: u64,
    ) -> Self {
        let now = utils::now();

        let shared = Shared {
            difficulties: Mutex::new(Difficulties::new(difficulty, 0, 0)),
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
        };

        let inner = Inner {
            id,
            _sid: sid,
            _client: client,
            _name: name,
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
                    "Miner: {} banned. {} out of the last {} shares were invalid",
                    self.inner.id,
                    stats.stale + stats.rejected,
                    total
                );
                self.ban();
            }
        }
    }

    pub(crate) fn difficulties(&self) -> Difficulties {
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
        let now = utils::now();

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
                >= self.config_manager.difficulty_config().retarget_time as u128)
        {
            return;
        }

        var_diff_stats.last_retarget = now;
        var_diff_stats.last_retarget_share = stats.accepted;

        // let variance = self.options.target_time * (self.options.variance_percent as f64 / 100.0);
        // let time_min = self.options.target_time as f64 * 0.40;
        // let time_max = self.options.target_time as f64 * 1.40;

        let avg = var_diff_stats.vardiff_buf.avg();

        if avg <= 0.0 {
            return;
        }

        let mut new_diff;

        //@todo figure out what else needs to come from config here, and comment out this function.
        let target_time = self.config_manager.difficulty_config().target_time as f64;
        if avg > target_time {
            if (avg / self.config_manager.difficulty_config().target_time as f64) <= 1.5 {
                return;
            }
            new_diff = difficulties.current() / 2;
        } else if (avg / target_time) >= 0.7 {
            return;
        } else {
            new_diff = difficulties.current() * 2;
        }

        new_diff = new_diff.clamp(
            self.config_manager.difficulty_config().minimum_difficulty,
            self.config_manager.difficulty_config().maximum_difficulty,
        );

        if new_diff != difficulties.current() {
            difficulties.update_next(new_diff);
            var_diff_stats.vardiff_buf.reset();
        }
    }

    #[must_use]
    pub fn update_difficulty(&self) -> Option<u64> {
        let mut difficulties = self.shared.difficulties.lock();

        difficulties.shift()
    }

    pub fn set_difficulty(&self, difficulty: u64) {
        let mut difficulties = self.shared.difficulties.lock();

        difficulties.set_and_shift(difficulty);
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
