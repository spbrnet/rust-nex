use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
    time::{Duration, Instant},
};

use rnex_util::PID;
use sqlx::{PgPool, Row};
use tokio::sync::Mutex;
use uuid::Uuid;

const REQUIRED_AGREEING_REPORTS: i64 = 4;

#[derive(Debug, Clone)]
pub struct ResultsStore {
    pub pool: PgPool,
    pub active_fest_id: u32,
    pending_children: Arc<Mutex<HashMap<u32, PendingChild>>>,
}

#[derive(Debug)]
struct PendingChild {
    expected_roster: HashSet<PID>,
    mode: u32,
    created_at: Instant,
}

#[derive(Debug, Clone, Copy)]
pub struct ResultReport {
    pub fest_id: u32,
    pub pid: PID,
    pub score: u32,
    pub team_id: u8,
    pub team_win: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitOutcome {
    Stored,
    Duplicate,
    Rejected,
}

impl ResultsStore {
    pub async fn connect() -> anyhow::Result<Self> {
        let url = env::var("RNEX_SPLATOON_RESULTS_DB")?;
        let active_fest_id: u32 = env::var("RNEX_SPLATOON_RESULTS_ACTIVE_SPLATFEST_ID")?.parse()?;

        anyhow::ensure!(active_fest_id != 0, "active fest id must be nonzero");

        let pool = PgPool::connect(&url).await?;
        Self::migrate(&pool).await?;

        Ok(Self::from_pool(pool, active_fest_id))
    }

    async fn migrate(pool: &PgPool) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        sqlx::query("SELECT pg_advisory_xact_lock(67395271168216649::bigint)")
            .fetch_one(&mut *tx)
            .await?;

        sqlx::raw_sql(include_str!("../migrations/splatfest.sql"))
            .execute(&mut *tx)
            .await?;

        tx.commit().await
    }

    pub fn from_pool(pool: PgPool, active_fest_id: u32) -> Self {
        Self {
            pool,
            active_fest_id,
            pending_children: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn record_automatic_match(
        &self,
        gid: u32,
        mode: u32,
        existing: HashSet<PID>,
        joining: HashSet<PID>,
        final_roster: HashSet<PID>,
    ) -> Result<bool, sqlx::Error> {
        if gid == 0
            || existing.is_empty()
            || joining.is_empty()
            || !existing.is_disjoint(&joining)
            || final_roster.len() != 8
            || existing.union(&joining).copied().collect::<HashSet<_>>() != final_roster
        {
            return Ok(false);
        }

        self.insert_match(gid, mode, final_roster).await
    }

    pub async fn observe_child_created(
        &self,
        gid: u32,
        mode: u32,
        expected_roster: HashSet<PID>,
        actual_roster: HashSet<PID>,
    ) -> Result<bool, sqlx::Error> {
        if gid == 0
            || expected_roster.len() != 8
            || actual_roster.is_empty()
            || !actual_roster.is_subset(&expected_roster)
        {
            return Ok(false);
        }

        if actual_roster == expected_roster {
            return self.insert_match(gid, mode, actual_roster).await;
        }

        let mut pending = self.pending_children.lock().await;
        pending.retain(|_, child| child.created_at.elapsed() < Duration::from_secs(45));
        pending.insert(
            gid,
            PendingChild {
                expected_roster,
                mode,
                created_at: Instant::now(),
            },
        );

        Ok(false)
    }

    pub async fn observe_child_join(
        &self,
        gid: u32,
        actual_roster: HashSet<PID>,
    ) -> Result<bool, sqlx::Error> {
        let mut pending = self.pending_children.lock().await;
        let Some(child) = pending.get(&gid) else {
            return Ok(false);
        };

        if child.created_at.elapsed() >= Duration::from_secs(45)
            || !actual_roster.is_subset(&child.expected_roster)
        {
            pending.remove(&gid);
            return Ok(false);
        }

        if actual_roster != child.expected_roster {
            return Ok(false);
        }

        let mode = child.mode;

        pending.remove(&gid);
        drop(pending);

        self.insert_match(gid, mode, actual_roster).await
    }

    async fn insert_match(
        &self,
        gid: u32,
        mode: u32,
        final_roster: HashSet<PID>,
    ) -> Result<bool, sqlx::Error> {
        let mut roster: Vec<i64> = final_roster.into_iter().map(i64::from).collect();

        roster.sort_unstable();

        let mut tx = self.pool.begin().await?;

        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(i64::from(self.active_fest_id))
            .fetch_one(&mut *tx)
            .await?;

        let recently_recorded: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM splatfest.matches \
             WHERE fest_id = $1 AND roster = $2 \
               AND started_at > now() - interval '90 seconds')",
        )
        .bind(i64::from(self.active_fest_id))
        .bind(&roster)
        .fetch_one(&mut *tx)
        .await?;

        if recently_recorded {
            return Ok(false);
        }

        let inserted = sqlx::query(
            "INSERT INTO splatfest.matches (id, fest_id, gid, game_mode, roster) \
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (fest_id, gid) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(i64::from(self.active_fest_id))
        .bind(i64::from(gid))
        .bind(i64::from(mode))
        .bind(roster)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(inserted.rows_affected() == 1)
    }

    pub async fn submit(&self, report: ResultReport) -> Result<SubmitOutcome, sqlx::Error> {
        if report.fest_id != self.active_fest_id || report.team_id > 1 {
            return Ok(SubmitOutcome::Rejected);
        }

        let mut tx = self.pool.begin().await?;

        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(i64::from(report.pid))
            .fetch_one(&mut *tx)
            .await?;

        let match_row = sqlx::query(
            "SELECT id, status FROM splatfest.matches \
             WHERE fest_id = $1 AND roster @> ARRAY[$2::bigint] \
               AND started_at <= now() - interval '30 seconds' \
               AND expires_at > now() \
             ORDER BY started_at DESC LIMIT 1 FOR UPDATE",
        )
        .bind(i64::from(report.fest_id))
        .bind(i64::from(report.pid))
        .fetch_optional(&mut *tx)
        .await?;

        let Some(match_row) = match_row else {
            return Ok(SubmitOutcome::Rejected);
        };
        let match_id: Uuid = match_row.try_get("id")?;
        let status: String = match_row.try_get("status")?;
        if status == "conflicted" {
            return Ok(SubmitOutcome::Rejected);
        }

        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM splatfest.submissions WHERE match_id = $1 AND pid = $2",
        )
        .bind(match_id)
        .bind(i64::from(report.pid))
        .fetch_one(&mut *tx)
        .await?;

        if existing != 0 {
            return Ok(SubmitOutcome::Duplicate);
        }

        let cooling_down = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM splatfest.submissions s \
             JOIN splatfest.matches m ON m.id = s.match_id \
             WHERE s.pid = $1 AND m.fest_id = $2 \
               AND s.submitted_at > now() - interval '3 minutes')",
        )
        .bind(i64::from(report.pid))
        .bind(i64::from(report.fest_id))
        .fetch_one(&mut *tx)
        .await?;
        if cooling_down {
            return Ok(SubmitOutcome::Rejected);
        }

        sqlx::query(
            "INSERT INTO splatfest.players (fest_id, pid, team_id) VALUES ($1, $2, $3) \
             ON CONFLICT (fest_id, pid) DO NOTHING",
        )
        .bind(i64::from(report.fest_id))
        .bind(i64::from(report.pid))
        .bind(i16::from(report.team_id))
        .execute(&mut *tx)
        .await?;

        let recorded_team: i16 = sqlx::query_scalar(
            "SELECT team_id FROM splatfest.players WHERE fest_id = $1 AND pid = $2",
        )
        .bind(i64::from(report.fest_id))
        .bind(i64::from(report.pid))
        .fetch_one(&mut *tx)
        .await?;
        if recorded_team != i16::from(report.team_id) {
            return Ok(SubmitOutcome::Rejected);
        }

        let winner_team = i16::from(report.team_id) ^ i16::from(!report.team_win);
        sqlx::query(
            "INSERT INTO splatfest.submissions \
             (match_id, pid, score, team_id, team_win, winner_team) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(match_id)
        .bind(i64::from(report.pid))
        .bind(i64::from(report.score))
        .bind(i16::from(report.team_id))
        .bind(report.team_win)
        .bind(winner_team)
        .execute(&mut *tx)
        .await?;

        // this code is a mess.
        if status == "pending" {
            let counts = sqlx::query(
                "SELECT winner_team, count(*) AS votes FROM splatfest.submissions \
                 WHERE match_id = $1 GROUP BY winner_team ORDER BY votes DESC, winner_team",
            )
            .bind(match_id)
            .fetch_all(&mut *tx)
            .await?;
            if let Some(first) = counts.first() {
                let votes: i64 = first.try_get("votes")?;
                if votes >= REQUIRED_AGREEING_REPORTS {
                    let winner: i16 = first.try_get("winner_team")?;
                    sqlx::query(
                        "UPDATE splatfest.matches SET status = 'confirmed', winner_team = $2 \
                         WHERE id = $1",
                    )
                    .bind(match_id)
                    .bind(winner)
                    .execute(&mut *tx)
                    .await?;
                } else {
                    let total: i64 = counts.iter().map(|row| row.get::<i64, _>("votes")).sum();
                    if total == 8 {
                        sqlx::query(
                            "UPDATE splatfest.matches SET status = 'conflicted' WHERE id = $1",
                        )
                        .bind(match_id)
                        .execute(&mut *tx)
                        .await?;
                    }
                }
            }
        }
        tx.commit().await?;
        Ok(SubmitOutcome::Stored)
    }

    pub async fn team_votes(&self, fest_id: u32) -> Result<Vec<u32>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT team_id, count(*) AS votes FROM splatfest.players \
             WHERE fest_id = $1 GROUP BY team_id",
        )
        .bind(i64::from(fest_id))
        .fetch_all(&self.pool)
        .await?;

        let mut votes = vec![0, 0];
        for row in rows {
            let team: i16 = row.try_get("team_id")?;
            let count: i64 = row.try_get("votes")?;
            if let Some(slot) = votes.get_mut(team as usize) {
                *slot = count.min(i64::from(u32::MAX)) as u32;
            }
        }

        Ok(votes)
    }

    pub async fn team_wins(&self, fest_id: u32) -> Result<Vec<u32>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT winner_team, count(*) AS wins FROM splatfest.matches \
             WHERE fest_id = $1 AND status = 'confirmed' GROUP BY winner_team",
        )
        .bind(i64::from(fest_id))
        .fetch_all(&self.pool)
        .await?;

        let mut wins = vec![0, 0];
        for row in rows {
            let team: i16 = row.try_get("winner_team")?;
            let count: i64 = row.try_get("wins")?;
            if let Some(slot) = wins.get_mut(team as usize) {
                *slot = count.min(i64::from(u32::MAX)) as u32;
            }
        }

        Ok(wins)
    }

    pub async fn scores(
        &self,
        fest_id: u32,
        offset: u32,
        size: u32,
    ) -> Result<Vec<(PID, u32)>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT s.pid, s.score FROM splatfest.submissions s \
             JOIN splatfest.matches m ON m.id = s.match_id \
             WHERE m.fest_id = $1 AND m.status = 'confirmed' \
             ORDER BY s.score DESC, s.submitted_at, s.pid LIMIT $2 OFFSET $3",
        )
        .bind(i64::from(fest_id))
        .bind(i64::from(size.min(100)))
        .bind(i64::from(offset))
        .fetch_all(&self.pool)
        .await?;
    
        rows.into_iter()
            .map(|row| {
                let pid: i64 = row.try_get("pid")?;
                let score: i64 = row.try_get("score")?;
                Ok((pid as PID, score as u32))
            })
            .collect()
    }
}