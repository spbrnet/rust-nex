use std::{env, fs, net::SocketAddr};

use rnex_mm::results::ResultsStore;
use rnex_server_api::splatfest_results::{
    FestivalRequest, FestivalSummary, ListMatchesRequest, ListMatchesResponse, ListReportsResponse,
    MatchRecord, MatchRequest, ReportRecord,
    splatfest_results_read_service_server::{
        SplatfestResultsReadService, SplatfestResultsReadServiceServer,
    },
};
use sqlx::Row;
use subtle::ConstantTimeEq;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{
    Request, Response, Status,
    transport::{Certificate, Identity, Server, ServerTlsConfig},
};

#[derive(Clone)]
struct ReadService {
    store: ResultsStore,
    token: Vec<u8>,
}

impl ReadService {
    fn authorize<T>(&self, request: &Request<T>) -> Result<(), Status> {
        let supplied = request
            .metadata()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("missing read credential"))?;
        if supplied.len() != self.token.len() || !bool::from(supplied.as_bytes().ct_eq(&self.token))
        {
            return Err(Status::unauthenticated("invalid read credential"));
        }
        Ok(())
    }
}

#[tonic::async_trait]
impl SplatfestResultsReadService for ReadService {
    async fn get_festival_summary(
        &self,
        request: Request<FestivalRequest>,
    ) -> Result<Response<FestivalSummary>, Status> {
        self.authorize(&request)?;
        let fest_id = request.into_inner().fest_id;
        let wins = self.store.team_wins(fest_id).await.map_err(db_error)?;
        let votes = self.store.team_votes(fest_id).await.map_err(db_error)?;
        let statuses = sqlx::query(
            "SELECT status, count(*) AS total FROM splatfest.matches \
             WHERE fest_id = $1 GROUP BY status",
        )
        .bind(i64::from(fest_id))
        .fetch_all(&self.store.pool)
        .await
        .map_err(db_error)?;
        let mut pending = 0;
        let mut conflicted = 0;
        for row in statuses {
            let status: String = row.try_get("status").map_err(db_error)?;
            let count: i64 = row.try_get("total").map_err(db_error)?;
            match status.as_str() {
                "pending" => pending = count as u64,
                "conflicted" => conflicted = count as u64,
                _ => {}
            }
        }
        Ok(Response::new(FestivalSummary {
            fest_id,
            team_wins: wins.into_iter().map(u64::from).collect(),
            team_votes: votes.into_iter().map(u64::from).collect(),
            pending_matches: pending,
            conflicted_matches: conflicted,
        }))
    }

    async fn list_matches(
        &self,
        request: Request<ListMatchesRequest>,
    ) -> Result<Response<ListMatchesResponse>, Status> {
        self.authorize(&request)?;
        let request = request.into_inner();
        let rows = sqlx::query(
            "SELECT m.id, m.fest_id, m.gid, m.game_mode, m.roster, m.status, \
                    m.winner_team, EXTRACT(EPOCH FROM m.started_at)::bigint AS started_at_unix, \
                    count(s.pid) AS report_count \
             FROM splatfest.matches m \
             LEFT JOIN splatfest.submissions s ON s.match_id = m.id \
             WHERE m.fest_id = $1 \
             GROUP BY m.id ORDER BY m.started_at DESC, m.id \
             LIMIT $2 OFFSET $3",
        )
        .bind(i64::from(request.fest_id))
        .bind(i64::from(request.limit.min(100)))
        .bind(i64::from(request.offset))
        .fetch_all(&self.store.pool)
        .await
        .map_err(db_error)?;
        let mut matches = Vec::with_capacity(rows.len());
        for row in rows {
            let id: uuid::Uuid = row.try_get("id").map_err(db_error)?;
            let winner_team: Option<i16> = row.try_get("winner_team").map_err(db_error)?;
            matches.push(MatchRecord {
                id: id.to_string(),
                fest_id: row.get::<i64, _>("fest_id") as u32,
                gid: row.get::<i64, _>("gid") as u32,
                game_mode: row.get::<i64, _>("game_mode") as u32,
                roster: row.try_get("roster").map_err(db_error)?,
                status: row.try_get("status").map_err(db_error)?,
                winner_team: winner_team.map(|team| team as u32),
                started_at_unix: row.try_get("started_at_unix").map_err(db_error)?,
                report_count: row.get::<i64, _>("report_count") as u32,
            });
        }
        Ok(Response::new(ListMatchesResponse { matches }))
    }

    async fn list_reports(
        &self,
        request: Request<MatchRequest>,
    ) -> Result<Response<ListReportsResponse>, Status> {
        self.authorize(&request)?;
        let id = request
            .into_inner()
            .id
            .parse::<uuid::Uuid>()
            .map_err(|_| Status::invalid_argument("invalid match ID"))?;
        let rows = sqlx::query(
            "SELECT pid, score, team_id, team_win, \
                    EXTRACT(EPOCH FROM submitted_at)::bigint AS submitted_at_unix \
             FROM splatfest.submissions WHERE match_id = $1 ORDER BY pid",
        )
        .bind(id)
        .fetch_all(&self.store.pool)
        .await
        .map_err(db_error)?;
        let reports = rows
            .into_iter()
            .map(|row| {
                Ok(ReportRecord {
                    pid: row.try_get("pid")?,
                    score: row.get::<i64, _>("score") as u32,
                    team_id: row.get::<i16, _>("team_id") as u32,
                    team_win: row.try_get("team_win")?,
                    submitted_at_unix: row.try_get("submitted_at_unix")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(db_error)?;
        Ok(Response::new(ListReportsResponse { reports }))
    }
}

fn db_error(error: sqlx::Error) -> Status {
    tracing::error!(?error, "splatfest read query failed");
    Status::internal("results are temporarily unavailable")
}

pub async fn start(store: ResultsStore) -> anyhow::Result<()> {
    let addr: SocketAddr = env::var("RNEX_SPLATOON_RESULTS_READ_ADDR")?.parse()?;
    let cert = fs::read(env::var("RNEX_SPLATOON_RESULTS_TLS_CERT")?)?;
    let key = fs::read(env::var("RNEX_SPLATOON_RESULTS_TLS_KEY")?)?;
    let client_ca = fs::read(env::var("RNEX_SPLATOON_RESULTS_CLIENT_CA")?)?;
    let token = env::var("RNEX_SPLATOON_RESULTS_READ_TOKEN")?.into_bytes();
    anyhow::ensure!(
        token.len() >= 32,
        "splatfest read token must be at least 32 bytes"
    );
    let tls = ServerTlsConfig::new()
        .identity(Identity::from_pem(cert, key))
        .client_ca_root(Certificate::from_pem(client_ca));
    let service = ReadService { store, token };
    let mut server = Server::builder().tls_config(tls)?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tokio::spawn(async move {
        if let Err(error) = server
            .add_service(SplatfestResultsReadServiceServer::new(service))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
        {
            tracing::error!(?error, "splatfest read service stopped");
        }
    });
    Ok(())
}
