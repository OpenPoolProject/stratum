use async_std::sync::{Arc, Mutex};
use extended_primitives::Buffer;
use log::warn;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{connection::MinerOptions, types::VarDiffBuffer};

//A miner is essentially an individual worker unit. There can be multiple Miners on a single
//connection which is why we needed to break it into these primitives.
#[derive(Debug, Clone)]
pub struct Miner {
    pub id: Uuid,
    pub sid: Buffer,
    pub client: Option<String>,
    pub name: Option<String>,
    //@todo one thing we could do here that I luike quite a bit is to just make this a tuple.
    //(old, new)
    pub difficulty: Arc<Mutex<u64>>,
    pub previous_difficulty: Arc<Mutex<u64>>,
    pub next_difficulty: Arc<Mutex<Option<u64>>>,
    pub stats: Arc<Mutex<MinerStats>>,
    pub job_stats: Arc<Mutex<JobStats>>,
    pub options: Arc<MinerOptions>,
    pub needs_ban: Arc<Mutex<bool>>,
}

impl Miner {
    pub fn new(
        id: Uuid,
        client: Option<String>,
        name: Option<String>,
        sid: Buffer,
        options: Arc<MinerOptions>,
        difficulty: u64,
    ) -> Self {
        Miner {
            id,
            sid,
            client,
            name,
            difficulty: Arc::new(Mutex::new(difficulty)),
            previous_difficulty: Arc::new(Mutex::new(difficulty)),
            next_difficulty: Arc::new(Mutex::new(None)),
            stats: Arc::new(Mutex::new(MinerStats {
                accepted_shares: 0,
                rejected_shares: 0,
                last_active: OffsetDateTime::now_utc(),
            })),
            job_stats: Arc::new(Mutex::new(JobStats {
                last_timestamp: OffsetDateTime::now_utc().unix_timestamp(),
                last_retarget: OffsetDateTime::now_utc().unix_timestamp()
                    - options.retarget_time as i64 / 2,
                vardiff_buf: VarDiffBuffer::new(),
                last_retarget_share: 0,
                current_difficulty: difficulty,
            })),
            options,
            needs_ban: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn ban(&self) {
        *self.needs_ban.lock().await = true;
        // self.disconnect().await;
    }

    pub async fn consider_ban(&self) {
        let accepted = self.stats.lock().await.accepted_shares;
        let rejected = self.stats.lock().await.rejected_shares;

        let total = accepted + rejected;

        //@todo come from options.
        let check_threshold = 500;
        let invalid_percent = 50.0;

        if total >= check_threshold {
            let percent_bad: f64 = (rejected as f64 / total as f64) * 100.0;

            if percent_bad < invalid_percent {
                //@todo make this possible. Reset stats to 0.
                // self.stats.lock().await = MinerStats::default();
            } else {
                warn!(
                    "Miner: {} banned. {} out of the last {} shares were invalid",
                    self.id, rejected, total
                );
                // self.ban().await; @todo
            }
        }
    }

    pub async fn current_difficulty(&self) -> u64 {
        *self.difficulty.lock().await
    }

    pub async fn previous_difficulty(&self) -> u64 {
        *self.previous_difficulty.lock().await
    }

    pub async fn valid_share(&self) {
        let mut stats = self.stats.lock().await;
        stats.accepted_shares += 1;
        stats.last_active = OffsetDateTime::now_utc();
        drop(stats);
        // self.consider_ban().await; @todo
        // @todo if we want to wrap this in an option, lets make it options.
        // @todo don't retarget until new job has been added.
        self.retarget().await;
    }

    pub async fn invalid_share(&self) {
        self.stats.lock().await.rejected_shares += 1;
        // self.consider_ban().await;
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
    async fn retarget(&self) {
        let now = OffsetDateTime::now_utc().unix_timestamp();

        let mut job_stats = self.job_stats.lock().await;

        let since_last = now - job_stats.last_timestamp;

        job_stats.vardiff_buf.append(since_last);
        job_stats.last_timestamp = now;

        //@todo set the retarget share amount in self.options as well.
        let stats = self.stats.lock().await;
        if !(((stats.accepted_shares - job_stats.last_retarget_share as u64) >= 30)
            || (now - job_stats.last_retarget) >= self.options.retarget_time as i64)
        {
            return;
        }

        job_stats.last_retarget = now;
        job_stats.last_retarget_share = stats.accepted_shares as i64;

        // let variance = self.options.target_time * (self.options.variance_percent as f64 / 100.0);
        // let time_min = self.options.target_time as f64 * 0.40;
        // let time_max = self.options.target_time as f64 * 1.40;

        let avg = job_stats.vardiff_buf.avg();

        if avg <= 0.0 {
            return;
        }
        // let mut d_dif = self.options.target_time as f64 / avg as f64;
        //
        let mut new_diff;

        if avg > self.options.target_time as f64 {
            if (avg / self.options.target_time as f64) <= 1.5 {
                return;
            } else {
                new_diff = job_stats.current_difficulty / 2;
            }
        } else {
            if (avg / self.options.target_time as f64) >= 0.7 {
                return;
            } else {
                new_diff = job_stats.current_difficulty * 2;
            }
        }

        if new_diff < self.options.min_diff {
            new_diff = self.options.min_diff;
        }

        if new_diff > self.options.max_diff {
            new_diff = self.options.max_diff;
        }

        if new_diff != job_stats.current_difficulty {
            *self.next_difficulty.lock().await = Some(new_diff);
            job_stats.vardiff_buf.reset();
        }

        // let mut new_difficulty = job_stats.current_difficulty.clone();
        // Too Fast
        // if (avg) < time_min {
        //     while (avg) < time_min && new_difficulty < self.options.max_diff {
        //         new_difficulty *= 2;
        //         avg *= 2.0;
        //     }
        //
        //     *self.next_difficulty.lock().await = Some(new_difficulty);
        // }

        // Too SLow
        // if (avg) > time_max && new_difficulty >= self.options.min_diff * 2 {
        //     while (avg) > time_max && new_difficulty >= self.options.min_diff * 2 {
        //         new_difficulty /= 2;
        //         avg /= 2.0;
        //     }
        // *self.next_difficulty.lock().await = Some(new_difficulty);

        // job_stats.times.clear();
    }

    pub async fn update_difficulty(&self) -> Option<u64> {
        let next_difficulty = *self.next_difficulty.lock().await;

        if let Some(next_difficulty) = next_difficulty {
            *self.difficulty.lock().await = next_difficulty;
            self.job_stats.lock().await.current_difficulty = next_difficulty;

            *self.next_difficulty.lock().await = None;

            Some(next_difficulty)
        } else {
            None
        }
    }

    pub async fn set_difficulty(&self, difficulty: u64) {
        let old_diff = self.difficulty.lock().await.clone();
        *self.difficulty.lock().await = difficulty;
        *self.previous_difficulty.lock().await = old_diff;
        self.job_stats.lock().await.current_difficulty = difficulty;
    }
}

#[derive(Debug, Clone)]
pub struct MinerStats {
    accepted_shares: u64,
    rejected_shares: u64,
    last_active: OffsetDateTime,
}

//@todo probably move these over to types.
//@todo maybe rename this as vardiff stats.
#[derive(Debug)]
pub struct JobStats {
    last_timestamp: i64,
    last_retarget_share: i64,
    last_retarget: i64,
    vardiff_buf: VarDiffBuffer,
    current_difficulty: u64,
}
