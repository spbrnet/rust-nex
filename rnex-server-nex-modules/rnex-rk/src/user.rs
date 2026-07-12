#[derive(Serialize, Deserialize)]
pub struct CompetitionPostResults {
    pub splatfest_id: u32,
    pub score: u32,
    pub team_id: u8,
    pub team_win: u8,
    pub user: PID,
}

// Seperate function because I cannot give a fuck right now
async fn fetch_team_votes(fest_id: u32) -> Result<Vec<u32>, ErrorCode> {
    let endpoint_votes = env::var("RNEX_SPLATOON_RESULTS_VOTES_GET").map_err(|_| {
        error!("RNEX_SPLATOON_RESULTS_VOTES_GET not set");
        ErrorCode::RendezVous_InvalidConfiguration
    })?;

    let url_votes = format!("{}?splatfest_id={}", endpoint_votes, fest_id);
    let mut response = tokio::task::spawn_blocking(|| {
        ureq::get(&url_votes).call().map_err(|e| {
            error!("GET for votes failed: {:?}", e);
            ErrorCode::RendezVous_InvalidConfiguration
        })
    })
    .await?;

    let body = response.body_mut().read_to_string().map_err(|e| {
        error!("failed to read votes body: {:?}", e);
        ErrorCode::RendezVous_InvalidConfiguration
    })?;

    let body = body.trim().trim_start_matches('[').trim_end_matches(']');
    let votes: Result<Vec<u32>, _> = body.split(',').map(|s| u32::from_str(s.trim())).collect();

    votes.map_err(|e| {
        error!("failed to parse votes: {:?}", e);
        ErrorCode::RendezVous_InvalidConfiguration
    })
}

impl Ranking for User {
    async fn competition_ranking_get_param(
        &self,
        param: CompetitionRankingGetParam,
    ) -> Result<Vec<CompetitionRankingScoreInfo>, ErrorCode> {
        let fest_id = param.festival_ids.get(0).copied().unwrap_or(0);

        let endpoint_results = env::var("RNEX_SPLATOON_RESULTS_GET").map_err(|_| {
            error!("RNEX_SPLATOON_RESULTS_GET not set");
            ErrorCode::RendezVous_InvalidConfiguration
        })?;

        let url_results = format!("{}?splatfest_id={}", endpoint_results, fest_id);
        let response_results = ureq::get(&url_results).call();

        let results: Vec<CompetitionPostResults> = match response_results {
            Ok(mut res) => res.body_mut().read_json().map_err(|e| {
                error!("failed to parse JSON: {:?}", e);
                ErrorCode::RendezVous_InvalidConfiguration
            })?,
            Err(e) => {
                error!("GET failed: {:?}", e);
                return Err(ErrorCode::RendezVous_InvalidConfiguration);
            }
        };

        let offset = param.range.offset as usize;
        let size = param.range.size as usize;

        let start = offset.min(results.len());
        let end = (start + size).min(results.len());

        let team_votes = fetch_team_votes(fest_id)?;
        let mut wins = vec![0u32, 0u32];
        for r in &results {
            let won_team = (r.team_id ^ (!r.team_win)) & 1;
            if let Some(team) = wins.get_mut(won_team as usize) {
                *team += 1
            };
        }

        let score_data: Vec<CompetitionRankingScoreData> = results[start..end]
            .iter()
            .map(|r| CompetitionRankingScoreData {
                unk: 1,
                pid: r.user,
                score: r.score,
                modified: KerberosDateTime::now(),
                unk2: 1,
                appdata: QBuffer(vec![]),
            })
            .collect();

        let info = CompetitionRankingScoreInfo {
            fest_id,
            score_data,
            unk: 0,
            team_wins: wins,
            team_votes,
        };

        println!("range: {:?}", param.range);

        Ok(vec![info])
    }

    async fn upload_competition_ranking_score(
        &self,
        param: UploadCompetitionData,
    ) -> Result<bool, ErrorCode> {
        info!("fest results for user {:?}:", self.pid);
        info!("fest id: {:?}", param.splatfest_id);
        info!("score: {:?}", param.score);
        info!("team id: {:?}", param.team_id);
        info!("did current team win: {:?}", param.team_win);

        let endpoint = match env::var("RNEX_SPLATOON_RESULTS_POST") {
            Ok(url) => url,
            Err(_) => {
                error!("RNEX_SPLATOON_RESULTS_POST not set");
                return Ok(false);
            }
        };

        let payload = CompetitionPostResults {
            splatfest_id: param.splatfest_id,
            score: param.score,
            team_id: param.team_id,
            team_win: param.team_win,
            user: self.pid,
        };

        let json_body = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => {
                error!("error making json_body: {:?}", e);
                return Ok(false);
            }
        };

        let response = ureq::post(&endpoint)
            .header("Content-Type", "application/json")
            .send(json_body);

        match response {
            Ok(res) => {
                info!("POST worked: {}", res.status());
            }
            Err(e) => {
                error!("POST borked: {:?}", e);
            }
        }

        Ok(true)
    }
}
