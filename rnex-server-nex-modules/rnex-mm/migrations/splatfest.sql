CREATE SCHEMA IF NOT EXISTS splatfest;

CREATE TABLE IF NOT EXISTS splatfest.matches (
    id uuid PRIMARY KEY,
    fest_id bigint NOT NULL,
    gid bigint NOT NULL,
    game_mode bigint NOT NULL,
    roster bigint[] NOT NULL,
    started_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL DEFAULT (now() + interval '10 minutes'),
    status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'confirmed', 'conflicted')),
    winner_team smallint CHECK (winner_team IN (0, 1)),
    CHECK (cardinality(roster) = 8)
);

CREATE INDEX IF NOT EXISTS splatfest_matches_fest_status_idx
    ON splatfest.matches (fest_id, status, started_at DESC);
CREATE INDEX IF NOT EXISTS splatfest_matches_roster_idx
    ON splatfest.matches USING gin (roster);
CREATE UNIQUE INDEX IF NOT EXISTS splatfest_matches_fest_gid_idx
    ON splatfest.matches (fest_id, gid);

CREATE TABLE IF NOT EXISTS splatfest.submissions (
    match_id uuid NOT NULL REFERENCES splatfest.matches(id),
    pid bigint NOT NULL,
    score bigint NOT NULL,
    team_id smallint NOT NULL CHECK (team_id IN (0, 1)),
    team_win boolean NOT NULL,
    winner_team smallint NOT NULL CHECK (winner_team IN (0, 1)),
    submitted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (match_id, pid)
);
CREATE INDEX IF NOT EXISTS splatfest_submissions_pid_idx
    ON splatfest.submissions (pid, submitted_at DESC);

CREATE TABLE IF NOT EXISTS splatfest.players (
    fest_id bigint NOT NULL,
    pid bigint NOT NULL,
    team_id smallint NOT NULL CHECK (team_id IN (0, 1)),
    first_played_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (fest_id, pid)
);
