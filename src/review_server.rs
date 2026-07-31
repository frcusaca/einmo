//! `einmo-review-server` — the axum HTTP app for `EinmoReview` (EIMP-2 §3,
//! §3a, `docs/eimp/`). One process, one suite, one session (created once at
//! startup against itself — `POST /einmo/sessions`), session-scoped routes
//! shaped for `EIMP-1`'s eventual multi-session support (§2) even though
//! only one session exists in this EIMP.
//!
//! Typed extractors throughout: `Path<EinmoId>`/`Path<SessionId>` reject a
//! malformed segment before handler code runs (a `400`, via
//! `serde::Deserialize`); `Json<Decision>` rejects an invalid `"kind"` tag
//! the same way. A single [`ApiError`] maps every failure to its HTTP
//! status in one place (EIMP-2 §3a).

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, RwLock};

use axum::Router;
use axum::extract::{Json, Path, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use serde::{Deserialize, Serialize};
use tokio_stream::Stream;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

use crate::review::{Decision, EinmoReview};
use crate::stage::{EinmoId, Stage};

/// A session identifier — opaque from the client's perspective, minted by
/// [`AppState::create_session`]. `Display`s as a short hex string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct SessionId(u64);

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        u64::from_str_radix(&s, 16)
            .map(SessionId)
            .map_err(|_| serde::de::Error::custom(format!("invalid session id {s:?}")))
    }
}

/// Every failure this server's handlers can produce, mapped to one HTTP
/// status each (EIMP-2 §3a) — the single place that mapping happens.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// The path names a session id no [`AppState`] session matches.
    #[error("no such session: {0}")]
    UnknownSession(SessionId),
    /// The path names a case id `EinmoReview::items` doesn't know about.
    #[error("no such case: {0}")]
    UnknownCase(String),
    /// A request body or path segment failed to parse/validate.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// An underlying einmo operation failed (verification, I/O, config).
    #[error(transparent)]
    Einmo(#[from] crate::error::EinmoError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::UnknownSession(_) | ApiError::UnknownCase(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Einmo(crate::error::EinmoError::InvalidId(_)) => StatusCode::BAD_REQUEST,
            ApiError::Einmo(crate::error::EinmoError::NoKey(_)) => StatusCode::BAD_REQUEST,
            ApiError::Einmo(crate::error::EinmoError::Config(_)) => StatusCode::BAD_REQUEST,
            ApiError::Einmo(crate::error::EinmoError::Verification(_)) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            ApiError::Einmo(crate::error::EinmoError::Io { source, .. })
                if source.kind() == std::io::ErrorKind::NotFound =>
            {
                StatusCode::NOT_FOUND
            }
            ApiError::Einmo(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = serde_json::json!({ "error": self.to_string() });
        (status, Json(body)).into_response()
    }
}

/// One SSE event this server can push (`EIMP-1` §S.7's `GET
/// /api/review/events` row: "decision-made / item-changed / executed").
/// Broadcast to every subscriber of a session's event stream after the
/// mutating handler that caused it has already returned successfully
/// (never before — a failed decide/execute must not announce an event that
/// didn't actually happen).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "kebab-case")]
enum ReviewEvent {
    /// `PUT`/`DELETE … /decision` recorded, replaced, or cleared a
    /// decision.
    DecisionMade {
        /// The case affected.
        id: String,
    },
    /// A case's stage layout changed outside the decide/execute flow —
    /// today: `flag`, `retract`, and `claim` (each mutates or advertises
    /// state a listing view would want to refresh for).
    ItemChanged {
        /// The case affected.
        id: String,
    },
    /// `POST … /execute` completed one batch.
    Executed {
        /// Cases whose action was applied.
        executed: Vec<String>,
        /// Cases whose action was skipped (drifted since planning).
        skipped: Vec<String>,
    },
}

impl ReviewEvent {
    /// The SSE `event:` field — a stable name a browser's
    /// `EventSource.addEventListener` can key on, distinct from the JSON
    /// tag only in that it never changes shape even if the payload does.
    fn name(&self) -> &'static str {
        match self {
            ReviewEvent::DecisionMade { .. } => "decision-made",
            ReviewEvent::ItemChanged { .. } => "item-changed",
            ReviewEvent::Executed { .. } => "executed",
        }
    }
}

/// Capacity of each session's SSE broadcast channel: generous enough that a
/// burst of decisions between one subscriber's polls is never lost, small
/// enough that a subscriber that never reads just lags (drops the oldest)
/// rather than growing the channel unboundedly.
const EVENT_CHANNEL_CAPACITY: usize = 256;

/// One registered session: the review itself, plus the broadcast sender
/// every `GET … /events` subscriber of this session listens on. Grouped
/// together (rather than two parallel maps keyed by the same [`SessionId`])
/// so a session and its event channel can never desync — created together
/// in [`AppState::create_session`], looked up together in
/// [`AppState::get`]/[`AppState::events_sender`].
struct SessionEntry {
    review: Arc<EinmoReview>,
    events: tokio::sync::broadcast::Sender<ReviewEvent>,
}

/// Shared server state: every open session, keyed by [`SessionId`].
#[derive(Default)]
pub struct AppState {
    sessions: RwLock<HashMap<SessionId, SessionEntry>>,
    next_id: std::sync::atomic::AtomicU64,
}

impl AppState {
    /// Open a new session over `suite` and register it. Returns the id the
    /// client will address it by.
    pub fn create_session(&self, suite: impl Into<std::path::PathBuf>) -> SessionId {
        let id = SessionId(
            self.next_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        );
        let review = Arc::new(EinmoReview::open(suite));
        let (events, _rx) = tokio::sync::broadcast::channel(EVENT_CHANNEL_CAPACITY);
        self.sessions
            .write()
            .expect("sessions lock poisoned")
            .insert(id, SessionEntry { review, events });
        id
    }

    fn get(&self, id: SessionId) -> Result<Arc<EinmoReview>, ApiError> {
        self.sessions
            .read()
            .expect("sessions lock poisoned")
            .get(&id)
            .map(|entry| entry.review.clone())
            .ok_or(ApiError::UnknownSession(id))
    }

    /// Subscribe to `id`'s event stream (`EIMP-1` §S.7's `GET … /events`).
    fn subscribe(
        &self,
        id: SessionId,
    ) -> Result<tokio::sync::broadcast::Receiver<ReviewEvent>, ApiError> {
        self.sessions
            .read()
            .expect("sessions lock poisoned")
            .get(&id)
            .map(|entry| entry.events.subscribe())
            .ok_or(ApiError::UnknownSession(id))
    }

    /// Broadcast `event` to `id`'s subscribers, if any. A send error just
    /// means nobody is currently listening — not a failure of whatever
    /// mutation produced the event, which has already succeeded by the time
    /// this is called.
    fn publish(&self, id: SessionId, event: ReviewEvent) {
        if let Some(entry) = self
            .sessions
            .read()
            .expect("sessions lock poisoned")
            .get(&id)
        {
            let _ = entry.events.send(event);
        }
    }
}

/// `POST /einmo/sessions` request body.
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    /// The suite's work directory.
    pub suite: std::path::PathBuf,
}

/// `POST /einmo/sessions` response body.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    /// The new session's id.
    pub session: String,
}

async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let id = state.create_session(req.suite);
    Json(CreateSessionResponse {
        session: id.to_string(),
    })
}

/// One worklist row, as returned by `GET /einmo/<session>/cases`.
#[derive(Debug, Serialize, Deserialize)]
pub struct CaseSummary {
    /// The case identifier.
    pub id: String,
    /// `differing` per [`crate::review::ReviewItem`].
    pub differing: bool,
    /// `(stage name, status if present)` pairs.
    pub stages: Vec<(String, Option<String>)>,
    /// The reviewer's current decision, if any, rendered as a short tag.
    pub decision: Option<String>,
}

impl From<crate::review::ReviewItem> for CaseSummary {
    fn from(item: crate::review::ReviewItem) -> Self {
        CaseSummary {
            id: item.id.as_str().to_string(),
            differing: item.differing,
            stages: item
                .stages
                .into_iter()
                .map(|(s, st)| (s.dir_name().to_string(), st))
                .collect(),
            decision: item.decision.as_ref().map(decision_tag),
        }
    }
}

fn decision_tag(d: &Decision) -> String {
    match d {
        Decision::Promote { to } => format!("promote to {to}"),
        Decision::Retract { from } => format!("retract from {from}"),
        Decision::Flag { stage, reason } => format!("flag {stage}: {reason}"),
        Decision::Skip => "skip".to_string(),
    }
}

async fn list_cases(
    State(state): State<Arc<AppState>>,
    Path(session): Path<SessionId>,
) -> Result<Json<Vec<CaseSummary>>, ApiError> {
    let review = state.get(session)?;
    let items = review.items().map_err(ApiError::from)?;
    Ok(Json(items.into_iter().map(CaseSummary::from).collect()))
}

async fn case_detail(
    State(state): State<Arc<AppState>>,
    Path((session, id)): Path<(SessionId, EinmoId)>,
) -> Result<Json<CaseSummary>, ApiError> {
    let review = state.get(session)?;
    let items = review.items().map_err(ApiError::from)?;
    items
        .into_iter()
        .find(|i| i.id == id)
        .map(CaseSummary::from)
        .map(Json)
        .ok_or_else(|| ApiError::UnknownCase(id.as_str().to_string()))
}

/// `GET /einmo/<session>/cases/<id>/body/<stage>` response body.
#[derive(Debug, Serialize, Deserialize)]
pub struct BodyResponse {
    /// `(section name, section body)` pairs, STAMPS excluded.
    pub sections: Vec<(String, String)>,
}

async fn case_body(
    State(state): State<Arc<AppState>>,
    Path((session, id, stage)): Path<(SessionId, EinmoId, Stage)>,
) -> Result<Json<BodyResponse>, ApiError> {
    let review = state.get(session)?;
    let body = review.body(&id, stage).map_err(ApiError::from)?;
    Ok(Json(BodyResponse {
        sections: body.sections,
    }))
}

/// One line of a [`SectionDiffResponse`], tagged by `"tag"` — the wire form
/// of [`crate::review::DiffLine`].
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "lowercase")]
pub enum DiffLineResponse {
    /// Present, unchanged, on both sides.
    Equal {
        /// The line text.
        text: String,
    },
    /// Present on the `left` side, absent or different on `right`.
    Removed {
        /// The line text.
        text: String,
    },
    /// Present on the `right` side, absent or different on `left`.
    Added {
        /// The line text.
        text: String,
    },
}

impl From<crate::review::DiffLine> for DiffLineResponse {
    fn from(line: crate::review::DiffLine) -> Self {
        match line {
            crate::review::DiffLine::Equal(text) => DiffLineResponse::Equal { text },
            crate::review::DiffLine::Removed(text) => DiffLineResponse::Removed { text },
            crate::review::DiffLine::Added(text) => DiffLineResponse::Added { text },
        }
    }
}

/// One section's diff, as returned by `GET … /diff/<left>/<right>`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SectionDiffResponse {
    /// The section name.
    pub name: String,
    /// The section's line-level diff, `left` vs `right`.
    pub lines: Vec<DiffLineResponse>,
}

impl From<crate::review::SectionDiff> for SectionDiffResponse {
    fn from(section: crate::review::SectionDiff) -> Self {
        SectionDiffResponse {
            name: section.name,
            lines: section.lines.into_iter().map(Into::into).collect(),
        }
    }
}

/// `GET /einmo/<session>/cases/<id>/diff/<left>/<right>` response body
/// (`EIMP-1` §S.7).
#[derive(Debug, Serialize, Deserialize)]
pub struct DiffResponse {
    /// One entry per section present on either side (STAMPS excluded).
    pub sections: Vec<SectionDiffResponse>,
}

async fn case_diff(
    State(state): State<Arc<AppState>>,
    Path((session, id, left, right)): Path<(SessionId, EinmoId, Stage, Stage)>,
) -> Result<Json<DiffResponse>, ApiError> {
    let review = state.get(session)?;
    let hunks = review.diff(&id, left, right).map_err(ApiError::from)?;
    Ok(Json(DiffResponse {
        sections: hunks.sections.into_iter().map(Into::into).collect(),
    }))
}

/// A promote/retract destination or source restricted to the two stages a
/// human decision can name (`checked`/`verified`) — deliberately narrower
/// than [`Stage`] (which also has `output`/`flagged`, neither a legal
/// promote/retract target), so an invalid value is a `400` at the `Json`
/// extractor, before any handler code runs (EIMP-2 §3a).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecidableStage {
    /// The `checked` stage.
    Checked,
    /// The `verified` stage.
    Verified,
}

impl From<DecidableStage> for Stage {
    fn from(s: DecidableStage) -> Self {
        match s {
            DecidableStage::Checked => Stage::Checked,
            DecidableStage::Verified => Stage::Verified,
        }
    }
}

/// `PUT /einmo/<session>/cases/<id>/decision` request body — one of
/// [`Decision`]'s four variants, tagged by `"kind"` (EIMP-2 §3).
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum DecisionRequest {
    /// Record a pending promotion to `to`.
    Promote {
        /// The destination stage.
        to: DecidableStage,
    },
    /// Record a pending retraction from `from`.
    Retract {
        /// The stage to retract from.
        from: DecidableStage,
    },
    /// Record a pending flag at `stage` with `reason`. Normally the
    /// convenience `POST … /flag/<stage>` endpoint is used instead — this
    /// variant exists so `PUT … /decision` can express all four [`Decision`]
    /// kinds, matching EIMP-2 §3's table.
    Flag {
        /// The stage to flag from.
        stage: DecidableStage,
        /// The advisory reason.
        reason: String,
    },
    /// Record "looked, deliberately chose not to rule".
    Skip,
}

impl From<DecisionRequest> for Decision {
    fn from(req: DecisionRequest) -> Self {
        match req {
            DecisionRequest::Promote { to } => Decision::Promote { to: to.into() },
            DecisionRequest::Retract { from } => Decision::Retract { from: from.into() },
            DecisionRequest::Flag { stage, reason } => Decision::Flag {
                stage: stage.into(),
                reason,
            },
            DecisionRequest::Skip => Decision::Skip,
        }
    }
}

/// Record (or replace — replace-not-stack) `id`'s pending decision.
///
/// **No `If-Match`/409 (`EIMP-1` §S.7's table row, deliberately unimplemented
/// — recorded here, not silently dropped).** That row reads a request
/// `version` against an item's current `version` and 409s on mismatch. The
/// plan's own Phase 0 drift survey found the precondition this needs does
/// not exist: `EinmoReview` keeps **no cached worklist** (`items()` rescans
/// disk on every call) and **no `version` field** anywhere in
/// `review.rs` — there is nothing to compare a request's `If-Match` against.
/// Retrofitting a version counter solely to satisfy this one endpoint would
/// mean inventing a staleness signal `EinmoReview` was deliberately *not*
/// designed to carry (its actual staleness story is content-fingerprint
/// drift-checking at `decide`/`refresh`/`execute` time, §S.2's "Notable
/// drift" sub-tasks and the `refresh()` checkbox above). Building a real
/// `version` field is a design decision belonging to whichever future work
/// actually needs optimistic concurrency at the HTTP layer — out of scope
/// for this checkbox, which only asked to implement or explain, not force a
/// field into existence as a side effect.
async fn put_decision(
    State(state): State<Arc<AppState>>,
    Path((session, id)): Path<(SessionId, EinmoId)>,
    Json(req): Json<DecisionRequest>,
) -> Result<StatusCode, ApiError> {
    let review = state.get(session)?;
    review.decide(id.clone(), req.into());
    state.publish(
        session,
        ReviewEvent::DecisionMade {
            id: id.as_str().to_string(),
        },
    );
    Ok(StatusCode::OK)
}

/// Clear `id`'s pending decision back to "untouched" (a no-op, still `200`,
/// if it was already undecided).
async fn delete_decision(
    State(state): State<Arc<AppState>>,
    Path((session, id)): Path<(SessionId, EinmoId)>,
) -> Result<StatusCode, ApiError> {
    let review = state.get(session)?;
    review.undecide(&id);
    state.publish(
        session,
        ReviewEvent::DecisionMade {
            id: id.as_str().to_string(),
        },
    );
    Ok(StatusCode::OK)
}

/// `POST /einmo/<session>/cases/<id>/claim` request body — the claim TTL,
/// in seconds. Omit (or `0`) for [`EinmoReview::DEFAULT_CLAIM_TTL`].
#[derive(Debug, Default, Deserialize)]
pub struct ClaimRequest {
    /// Seconds the claim should last; `0`/absent means the library default
    /// (5 minutes, `EIMP-1` §S.5).
    #[serde(default)]
    pub ttl_secs: u64,
}

/// Advertise "I'm on this one" for `id` (`EIMP-1` §S.5): a soft, advisory
/// claim — never enforced, cannot wedge `decide`/`execute` for this or any
/// other case. Surfaced back to every reviewer via `GET … /plan`'s `claims`
/// field.
async fn claim_case(
    State(state): State<Arc<AppState>>,
    Path((session, id)): Path<(SessionId, EinmoId)>,
    Json(req): Json<ClaimRequest>,
) -> Result<StatusCode, ApiError> {
    let review = state.get(session)?;
    if req.ttl_secs == 0 {
        review.claim(&id);
    } else {
        review.claim_for(&id, std::time::Duration::from_secs(req.ttl_secs));
    }
    state.publish(
        session,
        ReviewEvent::ItemChanged {
            id: id.as_str().to_string(),
        },
    );
    Ok(StatusCode::OK)
}

/// `GET /einmo/<session>/events` — SSE stream of this session's
/// decision-made/item-changed/executed events (`EIMP-1` §S.7). Each pushed
/// event's `event:` field is one of those three names ([`ReviewEvent::name`]);
/// `data:` is the event's JSON payload. A keep-alive comment line is sent
/// periodically so a proxy or client library doesn't mistake an idle
/// session for a dead connection.
async fn session_events(
    State(state): State<Arc<AppState>>,
    Path(session): Path<SessionId>,
) -> Result<Sse<impl Stream<Item = std::result::Result<Event, Infallible>>>, ApiError> {
    let rx = state.subscribe(session)?;
    let stream = BroadcastStream::new(rx).filter_map(|msg| {
        msg.ok().map(|event| {
            let data = serde_json::to_string(&event).unwrap_or_default();
            Ok(Event::default().event(event.name()).data(data))
        })
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// One planned action, as returned by `GET /einmo/<session>/plan`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PlannedActionResponse {
    /// A pending promotion.
    Promote {
        /// The case.
        id: String,
        /// The destination stage.
        to: String,
    },
    /// A pending retraction.
    Retract {
        /// The case.
        id: String,
        /// The stage retracted from.
        from: String,
    },
    /// A pending flag.
    Flag {
        /// The case.
        id: String,
        /// The stage flagged from.
        stage: String,
        /// The advisory reason.
        reason: String,
    },
}

impl From<crate::review::PlannedAction> for PlannedActionResponse {
    fn from(action: crate::review::PlannedAction) -> Self {
        match action {
            crate::review::PlannedAction::Promote { id, to } => PlannedActionResponse::Promote {
                id: id.as_str().to_string(),
                to: to.dir_name().to_string(),
            },
            crate::review::PlannedAction::Retract { id, from } => PlannedActionResponse::Retract {
                id: id.as_str().to_string(),
                from: from.dir_name().to_string(),
            },
            crate::review::PlannedAction::Flag { id, stage, reason } => {
                PlannedActionResponse::Flag {
                    id: id.as_str().to_string(),
                    stage: stage.dir_name().to_string(),
                    reason,
                }
            }
        }
    }
}

/// One active soft claim (`EIMP-1` §S.5), as returned by
/// `GET /einmo/<session>/plan` — same shape the CLI's `cmd_plan`
/// (`src/bin/einmo_review_server.rs`) already prints.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimResponse {
    /// The claimed case.
    pub id: String,
    /// Seconds remaining before the claim auto-expires.
    pub remaining_secs: u64,
}

impl From<crate::review::ActiveClaim> for ClaimResponse {
    fn from(claim: crate::review::ActiveClaim) -> Self {
        ClaimResponse {
            id: claim.id.as_str().to_string(),
            remaining_secs: claim.remaining.as_secs(),
        }
    }
}

/// `GET /einmo/<session>/plan` response body — a preview of what
/// `POST … /execute` would do right now; also the end-of-pass summary the
/// client renders (EIMP-2 §5).
#[derive(Debug, Serialize, Deserialize)]
pub struct PlanResponse {
    /// The actions a call to execute would apply, in order.
    pub actions: Vec<PlannedActionResponse>,
    /// Every currently-active soft claim (`EIMP-1` §S.5) — advisory
    /// display data only, never consulted by `execute` itself.
    pub claims: Vec<ClaimResponse>,
}

async fn get_plan(
    State(state): State<Arc<AppState>>,
    Path(session): Path<SessionId>,
) -> Result<Json<PlanResponse>, ApiError> {
    let review = state.get(session)?;
    let plan = review.plan();
    Ok(Json(PlanResponse {
        actions: plan.actions.into_iter().map(Into::into).collect(),
        claims: plan.claims.into_iter().map(Into::into).collect(),
    }))
}

/// `POST /einmo/<session>/execute` request body. The passphrase, if
/// present, is used only long enough to build the `SignerSet` passed into
/// [`EinmoReview::execute`] — never stored on `AppState` or logged
/// (EIMP-2 §4).
#[derive(Deserialize)]
pub struct ExecuteRequest {
    /// Must be the literal string `"PROMOTE"` for any pending promotion to
    /// apply — a typo-resistant confirmation gate, not a security boundary.
    pub confirm: String,
    /// The `checked → verified` signing passphrase, if any pending decision
    /// promotes to `verified`. `output`/`checked → checked` promotions
    /// always use the computer/empty-passphrase key.
    #[serde(default)]
    pub passphrase: Option<String>,
}

impl std::fmt::Debug for ExecuteRequest {
    /// Never renders the raw passphrase — this doc comment's own "never
    /// logged" promise otherwise has nothing stopping the first
    /// `tracing::debug!("{:?}", req)` (or similar) from breaking it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecuteRequest")
            .field("confirm", &self.confirm)
            .field(
                "passphrase",
                &self.passphrase.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

/// One completed action, as returned by `POST … /execute`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutedResponse {
    /// The case id.
    pub id: String,
    /// What happened (e.g. `"promoted to checked"`).
    pub detail: String,
    /// See [`crate::review::Executed::non_human`] — `true` if this was a
    /// promotion to `verified` signed with a well-known computer key (a
    /// non-human attestation), never silently dropped on this path.
    pub non_human: bool,
}

/// The result of `POST … /execute`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteResponse {
    /// Actions that completed.
    pub executed: Vec<ExecutedResponse>,
    /// Case ids skipped because their source drifted since planning.
    pub skipped: Vec<String>,
}

async fn post_execute(
    State(state): State<Arc<AppState>>,
    Path(session): Path<SessionId>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Json<ExecuteResponse>, ApiError> {
    let review = state.get(session)?;
    let plan = review.plan();
    let has_promotion = plan
        .actions
        .iter()
        .any(|a| matches!(a, crate::review::PlannedAction::Promote { .. }));
    if has_promotion && req.confirm != "PROMOTE" {
        return Err(ApiError::BadRequest(
            "execute needs confirm: \"PROMOTE\" when any pending decision promotes".to_string(),
        ));
    }

    // The passphrase lives in this request's Json<ExecuteRequest> only —
    // built into a SignerSet right here, handed to execute() for the
    // duration of this one call, and dropped when this function returns.
    // Never stored on AppState, never logged (EIMP-2 §4).
    let keys = crate::review::SignerSet {
        to_checked: crate::config::KeySource::from_passphrase(""),
        to_verified: req
            .passphrase
            .map(crate::config::KeySource::from_passphrase),
    };
    let report = review.execute(&plan, &keys).map_err(ApiError::from)?;
    state.publish(
        session,
        ReviewEvent::Executed {
            executed: report
                .executed
                .iter()
                .map(|e| e.id.as_str().to_string())
                .collect(),
            skipped: report.skipped.iter().map(|id| id.to_string()).collect(),
        },
    );
    Ok(Json(ExecuteResponse {
        executed: report
            .executed
            .into_iter()
            .map(|e| ExecutedResponse {
                id: e.id.as_str().to_string(),
                detail: e.detail,
                non_human: e.non_human,
            })
            .collect(),
        skipped: report
            .skipped
            .into_iter()
            .map(|id| id.to_string())
            .collect(),
    }))
}

/// `POST /einmo/<session>/cases/<id>/flag/<stage>` request body.
#[derive(Debug, Deserialize)]
pub struct FlagRequest {
    /// The advisory reason recorded in the flagged file.
    pub reason: String,
}

/// Flag `id` at `stage` immediately — a single atomic call, unlike promote
/// (EIMP-2 §3, `EinmoReview::flag_now`).
async fn flag_case(
    State(state): State<Arc<AppState>>,
    Path((session, id, stage)): Path<(SessionId, EinmoId, Stage)>,
    Json(req): Json<FlagRequest>,
) -> Result<StatusCode, ApiError> {
    let review = state.get(session)?;
    review.flag_now(&id, stage, &req.reason)?;
    state.publish(
        session,
        ReviewEvent::ItemChanged {
            id: id.as_str().to_string(),
        },
    );
    Ok(StatusCode::OK)
}

/// Retract (demote) `id` from `stage` immediately — a single atomic call,
/// unlike promote (EIMP-2 §3, `EinmoReview::retract_now`). Cascades
/// `checked → verified`.
async fn retract_case(
    State(state): State<Arc<AppState>>,
    Path((session, id, stage)): Path<(SessionId, EinmoId, Stage)>,
) -> Result<StatusCode, ApiError> {
    let review = state.get(session)?;
    review.retract_now(&id, stage)?;
    state.publish(
        session,
        ReviewEvent::ItemChanged {
            id: id.as_str().to_string(),
        },
    );
    Ok(StatusCode::OK)
}

/// Build the router. `state` is shared across every route via axum's
/// `State` extractor.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/einmo/sessions", post(create_session))
        .route("/einmo/{session}/cases", get(list_cases))
        .route("/einmo/{session}/cases/{id}", get(case_detail))
        .route("/einmo/{session}/cases/{id}/body/{stage}", get(case_body))
        .route(
            "/einmo/{session}/cases/{id}/diff/{left}/{right}",
            get(case_diff),
        )
        .route("/einmo/{session}/cases/{id}/flag/{stage}", post(flag_case))
        .route(
            "/einmo/{session}/cases/{id}/retract/{stage}",
            post(retract_case),
        )
        .route(
            "/einmo/{session}/cases/{id}/decision",
            put(put_decision).delete(delete_decision),
        )
        .route("/einmo/{session}/cases/{id}/claim", post(claim_case))
        .route("/einmo/{session}/plan", get(get_plan))
        .route("/einmo/{session}/execute", post(post_execute))
        .route("/einmo/{session}/events", get(session_events))
        .route("/review/{session}", get(serve_review_dhtml))
        .route("/", get(serve_review_dhtml_root))
        .with_state(state)
}

/// Bearer-token + `execute`-passphrase guard applied only to the TCP router
/// (`EIMP-1` §S.7, §S.7's TCP row: "TCP on 127.0.0.1 with a bearer token
/// only when a browser needs it"). UDS is left completely untouched by this
/// — [`router`] carries no auth layer at all, matching the spec's own
/// framing that a local socket's directory permissions already are the
/// access control (`EIMP-2`'s established mode-700 discipline).
#[derive(Clone)]
struct TcpGuard {
    /// The bearer token every TCP request must present
    /// (`Authorization: Bearer <token>`).
    token: String,
    /// `EIMP-2`'s standing caveat, restated in `EIMP-1` §S.7: the
    /// `checked → verified` passphrase travels **plaintext** in the
    /// `execute` request body, which is materially riskier over TCP than
    /// over a UDS (a local-only socket). Resolving that risk for real (TLS,
    /// mTLS) is out of scope for this EIMP — see the plan's own recorded
    /// decision. So by default this guard hard-refuses any TCP `execute`
    /// request whose body carries a non-null `passphrase`; setting this
    /// `true` (an explicit, separate opt-in flag — never implied by merely
    /// asking for TCP) lifts the refusal, on the caller's own informed
    /// judgment that their network path is trusted.
    allow_insecure_verified_passphrase: bool,
}

/// `axum::middleware::from_fn_with_state` guard for the TCP router: rejects
/// any request missing/mismatching the bearer token, then — unless
/// [`TcpGuard::allow_insecure_verified_passphrase`] — inspects `POST
/// … /execute` bodies and refuses (403, not silently ignoring the field) any
/// that carry a non-null `passphrase`. Every other request (including a
/// passphrase-less `execute`, e.g. a `checked`-only promotion) passes
/// through unchanged.
async fn tcp_guard(
    State(guard): State<TcpGuard>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let authorized = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.strip_prefix("Bearer ")
                .is_some_and(|presented| presented == guard.token)
        });
    if !authorized {
        return unauthorized_response();
    }
    if guard.allow_insecure_verified_passphrase
        || !(req.method() == axum::http::Method::POST && req.uri().path().ends_with("/execute"))
    {
        return next.run(req).await;
    }
    guard_execute_passphrase(req, next).await
}

fn unauthorized_response() -> Response {
    let body = serde_json::json!({ "error": "missing or invalid bearer token" });
    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}

/// The body-buffering half of [`tcp_guard`]'s `execute` check, split out so
/// the common-case (non-`execute`, or `allow_insecure_verified_passphrase`)
/// path above never pays for buffering a body it doesn't need to inspect.
async fn guard_execute_passphrase(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let (parts, body) = req.into_parts();
    let bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => {
            let body = serde_json::json!({ "error": "could not read request body" });
            return (StatusCode::BAD_REQUEST, Json(body)).into_response();
        }
    };
    let has_passphrase = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|v| v.get("passphrase").cloned())
        .is_some_and(|v| !v.is_null());
    if has_passphrase {
        let body = serde_json::json!({
            "error": "execute with a verified passphrase is refused over TCP by default \
                      (EIMP-1 §S.7: the passphrase travels plaintext in the request body, \
                      materially riskier over TCP than over a unix-domain socket) -- \
                      re-run the server with the insecure opt-in flag if you understand \
                      and accept that risk on this network path"
        });
        return (StatusCode::FORBIDDEN, Json(body)).into_response();
    }
    let req = axum::extract::Request::from_parts(parts, axum::body::Body::from(bytes));
    next.run(req).await
}

/// Build the TCP-served variant of [`router`]: the identical route table,
/// wrapped in [`tcp_guard`] (`EIMP-1` §S.7). UDS callers never pay for or
/// see this layer — only [`serve_tcp`]'s caller opts into it.
pub fn router_tcp(
    state: Arc<AppState>,
    token: String,
    allow_insecure_verified_passphrase: bool,
) -> Router {
    let guard = TcpGuard {
        token,
        allow_insecure_verified_passphrase,
    };
    router(state).layer(axum::middleware::from_fn_with_state(guard, tcp_guard))
}

/// Serve `app` over a unix-domain socket at `socket_path` until `shutdown`
/// resolves (EIMP-2 §7, `EIMP-1` §S.7/§S.7a).
///
/// **Socket lifecycle**: refuses to start if a stale socket file already
/// exists at `socket_path` and cannot be connected to (a genuinely dead
/// socket from a crashed prior run is removed and rebound; a socket a live
/// process is listening on is left alone — starting would just fail to
/// bind anyway). The socket file is removed on the way out, success or
/// error, so a clean shutdown leaves the directory as it found it.
///
/// **axum 0.8** (`EIMP-1` §S.7a): `axum::serve` accepts anything
/// implementing `axum::serve::Listener`, which includes
/// `tokio::net::UnixListener` directly (confirmed by reading
/// `axum-0.8.9/src/serve/listener.rs`'s `#[cfg(unix)] impl Listener for
/// tokio::net::UnixListener` before deleting the hand-rolled accept loop
/// this function used to run under axum 0.7.9, whose `serve()` was
/// TCP-only). That whole glue — a manual `UnixListener::accept` loop
/// feeding each connection through `hyper`'s HTTP/1.1 connection builder
/// via `hyper-util`'s `TokioIo`/`TowerToHyperService` — is gone; this is
/// now the same four-line shape `EIMP-1` §S.7a's snippet illustrates, with
/// the stale-vs-live probe kept exactly as it was (the snippet's
/// unconditional `remove_file` is explicitly wrong per that section's own
/// implementation notes).
///
/// # Errors
///
/// Returns an error if the socket cannot be bound (including the
/// stale-socket-still-live case above).
pub async fn serve_uds(
    app: Router,
    socket_path: &std::path::Path,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    use tokio::net::UnixListener;

    if socket_path.exists() {
        // A live socket refuses a fresh bind on its own (AddrInUse); only a
        // genuinely stale one (no listener left) needs manual removal.
        match tokio::net::UnixStream::connect(socket_path).await {
            Ok(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    format!("a server is already listening at {}", socket_path.display()),
                ));
            }
            Err(_) => std::fs::remove_file(socket_path)?,
        }
    }
    let listener = UnixListener::bind(socket_path)?;

    let result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await;
    let _ = std::fs::remove_file(socket_path); // best-effort cleanup either way
    result
}

/// Serve `app` over TCP at `addr` until `shutdown` resolves (`EIMP-1` §S.7:
/// "TCP on 127.0.0.1 with a bearer token only when a browser needs it" —
/// UDS stays the default; this is the opt-in). `app` is expected to already
/// carry [`router_tcp`]'s auth/passphrase-guard layer — this function is
/// deliberately a thin bind-and-serve, the TCP analogue of [`serve_uds`],
/// with the transport-specific safety policy factored into the router
/// builder instead of duplicated here.
///
/// **Loopback-only, enforced, not just documented.** A bind address whose
/// IP is not loopback (`127.0.0.1`/`::1`) is refused before any socket is
/// opened — "TCP on 127.0.0.1" in `EIMP-1` §S.7 is a hard invariant, not a
/// suggestion a caller could override by passing `0.0.0.0`.
///
/// # Errors
///
/// Returns an error if `addr` is not a loopback address, or if the socket
/// cannot be bound.
pub async fn serve_tcp(
    app: Router,
    addr: std::net::SocketAddr,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    if !addr.ip().is_loopback() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "refusing to bind TCP at {addr}: EIMP-1 §S.7 requires a loopback address \
                 (127.0.0.1/::1), never a wider-reachable one"
            ),
        ));
    }
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
}

/// The environment variable overriding the private-socket scratch
/// directory's base (`EIMP-1` §S.7a) — same override-with-a-default shape
/// `journal.rs`'s `EINMO_JOURNAL_DIR` and `einmo_review_client.sh`'s
/// `EINMO_REVIEW_CLIENT_DIR` already establish.
const PRIVATE_SOCKET_BASE_ENV: &str = "EINMO_REVIEW_PRIVATE_DIR";

/// The base directory [`private_socket_path`] mints private per-session
/// socket directories under: `$EINMO_REVIEW_PRIVATE_DIR` if set, else a
/// fixed subdirectory of the system temp dir.
fn private_socket_base_dir() -> std::path::PathBuf {
    match std::env::var_os(PRIVATE_SOCKET_BASE_ENV) {
        Some(dir) => std::path::PathBuf::from(dir),
        None => std::env::temp_dir().join("einmo-review-private"),
    }
}

/// Mint a fresh, unpredictable socket path for the TUI-owned private server
/// mode (`EIMP-1` §S.7a): a caller wants "a private, unpredictable socket
/// inside a mode-700 hardened scratch directory" rather than a fixed,
/// world-known path like `.einmo-review.sock`. This is the server-side
/// capability Phase D's client script drives — it does not itself start a
/// server, only reserves the path a subsequent [`serve_uds`] call binds to.
///
/// **The directory's random name plus its 0700 permissions are the whole
/// access control** — exactly `einmo_review_client.sh`'s own
/// `mktemp -d`-then-`harden_dir` pattern (an unguessable directory, files
/// inside it named predictably), and `journal.rs`'s `harden_dir` for the
/// Rust-side equivalent this function reuses directly rather than
/// hand-rolling a second copy. Nothing else on the machine can discover or
/// guess the resulting path without already being able to read this
/// process's own memory or the hardened directory's listing.
///
/// # Errors
///
/// Returns an error if a fresh directory name could not be minted after
/// several attempts (astronomically unlikely with 128 bits of randomness —
/// this only guards against a corrupted RNG or a full filesystem), or if
/// the directory could not be created/hardened.
pub fn private_socket_path() -> std::io::Result<std::path::PathBuf> {
    use rand_core::{OsRng, RngCore};

    let base = private_socket_base_dir();
    // Harden the BASE dir too, not just the per-session leaf below --
    // otherwise, under a typical umask, `base` ends up world-readable/
    // executable and leaks every "unpredictable" session directory's name
    // to any local user who lists it (the leaf's own 0700 still blocks
    // traversal INTO it, but the name itself is no longer a secret).
    crate::journal::harden_dir(&base)?;
    for _ in 0..8 {
        let mut bytes = [0u8; 16];
        OsRng.fill_bytes(&mut bytes);
        let dir = base.join(format!("einmo-review-{}", hex::encode(bytes)));
        match std::fs::create_dir(&dir) {
            Ok(()) => {
                crate::journal::harden_dir(&dir)?;
                return Ok(dir.join("review.sock"));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not mint a fresh private socket directory after 8 attempts",
    ))
}

const REVIEW_DHTML: &str = include_str!("dhtml/review.html");

async fn serve_review_dhtml() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        REVIEW_DHTML,
    )
}

async fn serve_review_dhtml_root() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        REVIEW_DHTML,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    static JOURNAL_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct TestContext {
        _journal_guard: std::sync::MutexGuard<'static, ()>,
        _journal_tmp: tempfile::TempDir,
        suite: tempfile::TempDir,
    }

    impl TestContext {
        fn path(&self) -> &std::path::Path {
            self.suite.path()
        }
    }

    fn test_context() -> TestContext {
        let guard = JOURNAL_ENV_LOCK.lock().unwrap();
        let journal_tmp = tempfile::tempdir().unwrap();
        // SAFETY: serialized by `JOURNAL_ENV_LOCK`.
        unsafe {
            std::env::set_var("EINMO_JOURNAL_DIR", journal_tmp.path());
        }
        TestContext {
            _journal_guard: guard,
            _journal_tmp: journal_tmp,
            suite: tempfile::tempdir().unwrap(),
        }
    }

    fn write_input(dir: &std::path::Path, rel: &str, content: &str) {
        let path = dir.join("input").join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    struct Echo;
    impl crate::einmo_suite::Evaluator for Echo {
        fn evaluate(&self, source: &str) -> std::result::Result<Vec<String>, String> {
            Ok(vec![source.trim().to_string()])
        }
    }

    fn seeded_suite() -> TestContext {
        let ctx = test_context();
        write_input(ctx.path(), "a.foo", "{1+1;}");
        let config =
            crate::config::TestConfig::new(ctx.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        suite.evaluate_all(&Echo).unwrap();
        ctx
    }

    fn promote_output_to_checked(dir: &std::path::Path) {
        let config =
            crate::config::TestConfig::new(dir, crate::einmo_suite::ValidationLevel::Output);
        crate::transitions::promote(
            &config,
            Stage::Output,
            Stage::Checked,
            &crate::config::KeySource::from_passphrase(""),
            None,
            None,
        )
        .unwrap();
    }

    async fn body_json<T: serde::de::DeserializeOwned>(response: Response) -> T {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn create_session_then_list_cases() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let app = router(state.clone());

        let req = Request::post("/einmo/sessions")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({ "suite": tmp.path() })).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let created: CreateSessionResponse = body_json(resp).await;

        let req = Request::get(format!("/einmo/{}/cases", created.session))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let cases: Vec<CaseSummary> = body_json(resp).await;
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].id, "a.foo");
    }

    #[tokio::test]
    async fn unknown_session_404s_on_every_route() {
        let state = Arc::new(AppState::default());
        let app = router(state);

        for uri in [
            "/einmo/deadbeefdeadbeef/cases",
            "/einmo/deadbeefdeadbeef/cases/a.foo",
            "/einmo/deadbeefdeadbeef/cases/a.foo/body/output",
            "/einmo/deadbeefdeadbeef/cases/a.foo/diff/output/checked",
        ] {
            let req = Request::get(uri).body(Body::empty()).unwrap();
            let resp = app.clone().oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND, "{uri}");
        }
    }

    #[tokio::test]
    async fn case_body_returns_verified_sections() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::get(format!("/einmo/{session}/cases/a.foo/body/output"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: BodyResponse = body_json(resp).await;
        assert!(body.sections.iter().any(|(name, _)| name == "OUTPUT"));
    }

    #[tokio::test]
    async fn case_diff_returns_all_equal_when_stages_match() {
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::get(format!("/einmo/{session}/cases/a.foo/diff/output/checked"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let diff: DiffResponse = body_json(resp).await;
        assert!(!diff.sections.is_empty());
        assert!(!diff.sections.iter().any(|s| s.name == "STAMPS"));
        for section in &diff.sections {
            assert!(
                section
                    .lines
                    .iter()
                    .all(|l| matches!(l, DiffLineResponse::Equal { .. })),
                "{:?}",
                section
            );
        }
    }

    #[tokio::test]
    async fn case_diff_404s_on_unknown_case() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::get(format!(
            "/einmo/{session}/cases/does-not-exist.foo/diff/output/checked"
        ))
        .body(Body::empty())
        .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn case_diff_400s_on_invalid_stage() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::get(format!("/einmo/{session}/cases/a.foo/diff/output/bogus"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn unknown_case_404s() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::get(format!("/einmo/{session}/cases/does-not-exist.foo"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    /// A path traversal attempt in the `<id>` segment must be rejected by
    /// the `Path<EinmoId>` extractor itself (`EinmoId`'s `Deserialize`
    /// routes through the same validation as `TryFrom<&str>`) — a `400`,
    /// before `case_detail`'s body runs at all (EIMP-2 §3a).
    #[tokio::test]
    async fn malformed_id_400s_at_the_extractor() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::get(format!("/einmo/{session}/cases/..%2f..%2fetc%2fpasswd"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn flag_endpoint_moves_the_case_and_returns_ok() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::post(format!("/einmo/{session}/cases/a.foo/flag/output"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({ "reason": "looks wrong" })).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        assert!(!tmp.path().join("output/a.foo.einmo").exists());
        let flagged = std::fs::read_to_string(tmp.path().join("flagged/a.foo.einmo")).unwrap();
        assert!(flagged.contains("# flagged: looks wrong"));

        // The case still appears in the worklist (now living under
        // flagged/), it just no longer has an `output` stage entry.
        let req = Request::get(format!("/einmo/{session}/cases/a.foo"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let detail: CaseSummary = body_json(resp).await;
        assert!(
            detail
                .stages
                .iter()
                .all(|(name, status)| name != "output" || status.is_none())
        );
    }

    #[tokio::test]
    async fn flag_endpoint_404s_on_unknown_case() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::post(format!(
            "/einmo/{session}/cases/does-not-exist.foo/flag/output"
        ))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({ "reason": "n/a" })).unwrap(),
        ))
        .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn flag_endpoint_400s_on_invalid_stage() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::post(format!("/einmo/{session}/cases/a.foo/flag/not-a-stage"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({ "reason": "n/a" })).unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn retract_endpoint_removes_the_case_and_returns_ok() {
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::post(format!("/einmo/{session}/cases/a.foo/retract/checked"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        assert!(!tmp.path().join("checked/a.foo.einmo").exists());
    }

    #[tokio::test]
    async fn retract_endpoint_404s_on_unknown_case() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::post(format!(
            "/einmo/{session}/cases/does-not-exist.foo/retract/checked"
        ))
        .body(Body::empty())
        .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn retract_endpoint_400s_on_invalid_stage() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::post(format!("/einmo/{session}/cases/a.foo/retract/not-a-stage"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn retract_endpoint_400s_on_output_stage() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::post(format!("/einmo/{session}/cases/a.foo/retract/output"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn put_decision_then_plan_reflects_it() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "promote", "to": "checked"}))
                    .unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let req = Request::get(format!("/einmo/{session}/plan"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let plan: PlanResponse = body_json(resp).await;
        assert_eq!(plan.actions.len(), 1);
        assert!(matches!(
            &plan.actions[0],
            PlannedActionResponse::Promote { id, to } if id == "a.foo" && to == "checked"
        ));

        // the case's own decision is visible too
        let req = Request::get(format!("/einmo/{session}/cases/a.foo"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let detail: CaseSummary = body_json(resp).await;
        assert_eq!(detail.decision.as_deref(), Some("promote to checked"));
    }

    #[tokio::test]
    async fn put_decision_replaces_not_stacks() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        for to in ["checked", "verified"] {
            let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({"kind": "promote", "to": to})).unwrap(),
                ))
                .unwrap();
            let resp = app.clone().oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }

        let req = Request::get(format!("/einmo/{session}/plan"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let plan: PlanResponse = body_json(resp).await;
        assert_eq!(plan.actions.len(), 1, "replace-not-stack: still one action");
    }

    #[tokio::test]
    async fn put_decision_400s_on_invalid_kind() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "not-a-kind"})).unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // axum's Json<T> extractor rejects a deserialize failure with its
        // own default (422 Unprocessable Entity), before ApiError/handler
        // code ever runs — unlike Path<EinmoId>, whose custom Deserialize
        // error axum's Path rejection maps to 400 (malformed_id_400s test).
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn delete_decision_clears_it() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "skip"})).unwrap(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        let req = Request::delete(format!("/einmo/{session}/cases/a.foo/decision"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let req = Request::get(format!("/einmo/{session}/cases/a.foo"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let detail: CaseSummary = body_json(resp).await;
        assert_eq!(detail.decision, None);
    }

    #[tokio::test]
    async fn delete_decision_is_a_no_op_when_already_undecided() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        // no PUT first — a.foo starts undecided
        let req = Request::delete(format!("/einmo/{session}/cases/a.foo/decision"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn put_decision_after_delete_starts_a_fresh_decision() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "promote", "to": "checked"}))
                    .unwrap(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        let req = Request::delete(format!("/einmo/{session}/cases/a.foo/decision"))
            .body(Body::empty())
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        // revisit: back in with a different decision
        let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "skip"})).unwrap(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        let req = Request::get(format!("/einmo/{session}/cases/a.foo"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let detail: CaseSummary = body_json(resp).await;
        assert_eq!(detail.decision.as_deref(), Some("skip"));
    }

    #[tokio::test]
    async fn execute_without_confirm_refuses_a_pending_promotion() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "promote", "to": "checked"}))
                    .unwrap(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        // a syntactically valid body, but the wrong (or absent) confirm
        // token — this exercises put_execute's own gate, not axum's
        // required-field extractor rejection (a missing "confirm" key
        // entirely is itself a 422 at the Json<ExecuteRequest> extractor,
        // covered implicitly since confirm has no #[serde(default)]).
        let req = Request::post(format!("/einmo/{session}/execute"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"confirm": ""})).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        assert!(!tmp.path().join("checked/a.foo.einmo").exists());
    }

    #[test]
    fn execute_request_debug_never_renders_the_raw_passphrase() {
        let req = ExecuteRequest {
            confirm: "PROMOTE".to_string(),
            passphrase: Some("very-secret-value".to_string()),
        };
        let rendered = format!("{req:?}");
        assert!(
            !rendered.contains("very-secret-value"),
            "Debug output must never contain the raw passphrase: {rendered}"
        );
        assert!(rendered.contains("<redacted>"));
        assert!(
            rendered.contains("PROMOTE"),
            "non-secret fields still render"
        );
    }

    #[tokio::test]
    async fn execute_with_confirm_promotes_to_checked() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "promote", "to": "checked"}))
                    .unwrap(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        let req = Request::post(format!("/einmo/{session}/execute"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"confirm": "PROMOTE"})).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let report: ExecuteResponse = body_json(resp).await;
        assert_eq!(report.executed.len(), 1);

        assert!(tmp.path().join("checked/a.foo.einmo").exists());

        // the executed decision is no longer pending
        let req = Request::get(format!("/einmo/{session}/plan"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let plan: PlanResponse = body_json(resp).await;
        assert!(plan.actions.is_empty());
    }

    #[tokio::test]
    async fn execute_promote_to_verified_needs_a_passphrase() {
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "promote", "to": "verified"}))
                    .unwrap(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        // no passphrase: refused, nothing written
        let req = Request::post(format!("/einmo/{session}/execute"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"confirm": "PROMOTE"})).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert!(!tmp.path().join("verified/a.foo.einmo").exists());

        // with a passphrase: succeeds
        let req = Request::post(format!("/einmo/{session}/execute"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(
                    &serde_json::json!({"confirm": "PROMOTE", "passphrase": "s3cr3t"}),
                )
                .unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(tmp.path().join("verified/a.foo.einmo").exists());
    }

    #[tokio::test]
    async fn execute_response_surfaces_non_human_for_a_computer_key_verified_promotion() {
        // An explicit EMPTY passphrase is the well-known computer key
        // (distinct from omitting `passphrase` entirely, which is refused
        // above) -- promoting to verified with it must be flagged
        // `non_human: true` in the HTTP response, the same signal
        // `einmo promote`'s own CLI warning already surfaces (`cli.rs`).
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "promote", "to": "verified"}))
                    .unwrap(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        let req = Request::post(format!("/einmo/{session}/execute"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"confirm": "PROMOTE", "passphrase": ""}))
                    .unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let report: ExecuteResponse = body_json(resp).await;
        assert_eq!(report.executed.len(), 1);
        assert!(
            report.executed[0].non_human,
            "an empty-passphrase verified promotion must report non_human: true, \
             not silently look identical to a genuine human attestation"
        );
    }

    #[tokio::test]
    async fn invalid_stage_400s() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::get(format!("/einmo/{session}/cases/a.foo/body/not-a-stage"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    /// End-to-end over a real unix-domain socket: bind, request via
    /// `hyperlocal`'s client extension, confirm the response, then trigger
    /// shutdown and confirm both the socket and (in the binary's own
    /// wiring, not exercised here) session file are gone.
    #[tokio::test]
    async fn serve_uds_end_to_end_and_cleans_up_on_shutdown() {
        use hyperlocal::{UnixClientExt, Uri};

        let tmp = seeded_suite();
        let socket_dir = tempfile::tempdir().unwrap();
        let socket_path = socket_dir.path().join("test.sock");

        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let shutdown = async move {
            let _ = shutdown_rx.await;
        };
        let socket_path_for_serve = socket_path.clone();
        let serve_handle =
            tokio::spawn(async move { serve_uds(app, &socket_path_for_serve, shutdown).await });

        // Give the accept loop a moment to bind.
        for _ in 0..50 {
            if socket_path.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(socket_path.exists(), "socket was never bound");

        let client: hyper_util::client::legacy::Client<_, axum::body::Body> =
            hyper_util::client::legacy::Client::unix();
        let uri: hyper::Uri = Uri::new(&socket_path, &format!("/einmo/{session}/cases")).into();
        let resp = client.get(uri).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        shutdown_tx.send(()).unwrap();
        serve_handle.await.unwrap().unwrap();
        assert!(!socket_path.exists(), "socket must be removed on shutdown");
    }

    /// A second bind attempt on a socket a live server still holds must be
    /// refused, not silently steal the path out from under it.
    #[tokio::test]
    async fn serve_uds_refuses_a_live_socket() {
        let tmp = seeded_suite();
        let socket_dir = tempfile::tempdir().unwrap();
        let socket_path = socket_dir.path().join("test.sock");

        let state = Arc::new(AppState::default());
        state.create_session(tmp.path());
        let app1 = router(state.clone());
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let shutdown = async move {
            let _ = shutdown_rx.await;
        };
        let socket_path_for_serve = socket_path.clone();
        let serve_handle =
            tokio::spawn(async move { serve_uds(app1, &socket_path_for_serve, shutdown).await });

        for _ in 0..50 {
            if socket_path.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        let app2 = router(state);
        let never = std::future::pending();
        let second = serve_uds(app2, &socket_path, never).await;
        assert!(second.is_err(), "a live socket must refuse a second bind");

        shutdown_tx.send(()).unwrap();
        serve_handle.await.unwrap().unwrap();
    }

    /// A stale socket file (no listener behind it) must be cleaned up and
    /// rebound, not treated as permanently occupied.
    #[tokio::test]
    async fn serve_uds_rebinds_a_stale_socket_file() {
        let tmp = seeded_suite();
        let socket_dir = tempfile::tempdir().unwrap();
        let socket_path = socket_dir.path().join("test.sock");
        // A plain file at the socket path, with nothing listening on it.
        std::fs::write(&socket_path, b"").unwrap();
        assert!(socket_path.exists());

        let state = Arc::new(AppState::default());
        state.create_session(tmp.path());
        let app = router(state);
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let shutdown = async move {
            let _ = shutdown_rx.await;
        };
        let socket_path_for_serve = socket_path.clone();
        let serve_handle =
            tokio::spawn(async move { serve_uds(app, &socket_path_for_serve, shutdown).await });

        for _ in 0..50 {
            if socket_path.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(
            socket_path.exists(),
            "the stale file must be rebound, not left alone"
        );

        shutdown_tx.send(()).unwrap();
        let result = serve_handle.await.unwrap();
        assert!(result.is_ok(), "rebinding a stale socket must succeed");
    }

    // --- claim endpoint (EIMP-1 §S.5/§S.7) ---------------------------------

    #[tokio::test]
    async fn claim_endpoint_surfaces_in_plan() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::post(format!("/einmo/{session}/cases/a.foo/claim"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({})).unwrap(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let req = Request::get(format!("/einmo/{session}/plan"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let plan: PlanResponse = body_json(resp).await;
        assert!(plan.actions.is_empty(), "claiming alone decides nothing");
        assert_eq!(
            plan.claims.len(),
            1,
            "the claim must surface in PlanResponse.claims"
        );
        assert_eq!(plan.claims[0].id, "a.foo");
        assert!(plan.claims[0].remaining_secs > 0);
    }

    #[tokio::test]
    async fn claim_endpoint_404s_on_unknown_case() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let req = Request::post(format!("/einmo/{session}/cases/does-not-exist.foo/claim"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({})).unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // claim() is unconditional on the library side (no existence check --
        // a claim is advisory metadata keyed by EinmoId, not a lookup into
        // items()), so this is NOT expected to 404 today; documented here so
        // a future change to that behavior is a deliberate, tested decision
        // rather than a silent regression either way.
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- SSE events (EIMP-1 §S.7) -------------------------------------------

    /// Subscribing then deciding must push exactly the `decision-made` event
    /// the decide caused, over the real streaming `Sse` body (not a
    /// buffered response) — proves the broadcast wiring end to end, not just
    /// that the endpoint returns 200.
    #[tokio::test]
    async fn events_stream_reports_a_decision_made_event() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let events_req = Request::get(format!("/einmo/{session}/events"))
            .body(Body::empty())
            .unwrap();
        let events_resp = app.clone().oneshot(events_req).await.unwrap();
        assert_eq!(events_resp.status(), StatusCode::OK);
        let mut stream = events_resp.into_body().into_data_stream();

        let decide_req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "skip"})).unwrap(),
            ))
            .unwrap();
        let decide_resp = app.oneshot(decide_req).await.unwrap();
        assert_eq!(decide_resp.status(), StatusCode::OK);

        let chunk = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
            .await
            .expect("timed out waiting for the SSE event")
            .expect("stream ended before any event arrived")
            .unwrap();
        let text = String::from_utf8(chunk.to_vec()).unwrap();
        assert!(text.contains("event: decision-made"), "{text}");
        assert!(text.contains("a.foo"), "{text}");
    }

    #[tokio::test]
    async fn events_endpoint_404s_on_unknown_session() {
        let state = Arc::new(AppState::default());
        let app = router(state);
        let req = Request::get("/einmo/deadbeefdeadbeef/events")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // --- TCP + bearer token (EIMP-1 §S.7) -----------------------------------

    #[tokio::test]
    async fn router_tcp_rejects_missing_bearer_token() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router_tcp(state, "s3cr3t-token".to_string(), false);

        let req = Request::get(format!("/einmo/{session}/cases"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn router_tcp_rejects_wrong_bearer_token() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router_tcp(state, "s3cr3t-token".to_string(), false);

        let req = Request::get(format!("/einmo/{session}/cases"))
            .header("authorization", "Bearer wrong-token")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn router_tcp_accepts_the_correct_bearer_token() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router_tcp(state, "s3cr3t-token".to_string(), false);

        let req = Request::get(format!("/einmo/{session}/cases"))
            .header("authorization", "Bearer s3cr3t-token")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// The passphrase-in-body risk `EIMP-2` flagged and `EIMP-1` §S.7
    /// restates: over TCP, by default, `execute` refuses any request
    /// carrying a non-null `passphrase`, even with a valid bearer token —
    /// authorization and transport-confidentiality are orthogonal, and a
    /// valid token does not make plaintext-over-TCP any less plaintext.
    #[tokio::test]
    async fn router_tcp_refuses_execute_with_a_passphrase_by_default() {
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router_tcp(state, "s3cr3t-token".to_string(), false);

        let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("authorization", "Bearer s3cr3t-token")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "promote", "to": "verified"}))
                    .unwrap(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        let req = Request::post(format!("/einmo/{session}/execute"))
            .header("authorization", "Bearer s3cr3t-token")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(
                    &serde_json::json!({"confirm": "PROMOTE", "passphrase": "s3cr3t"}),
                )
                .unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        assert!(!tmp.path().join("verified/a.foo.einmo").exists());
    }

    /// A passphrase-less `execute` (e.g. a `checked`-only promotion, which
    /// always uses the computer key) is never blocked by the TCP guard —
    /// the refusal above is specifically about the plaintext passphrase
    /// field, not about `execute` over TCP in general.
    #[tokio::test]
    async fn router_tcp_allows_execute_without_a_passphrase() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router_tcp(state, "s3cr3t-token".to_string(), false);

        let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("authorization", "Bearer s3cr3t-token")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "promote", "to": "checked"}))
                    .unwrap(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        let req = Request::post(format!("/einmo/{session}/execute"))
            .header("authorization", "Bearer s3cr3t-token")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"confirm": "PROMOTE"})).unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(tmp.path().join("checked/a.foo.einmo").exists());
    }

    /// The explicit, separate opt-in lifts the refusal — on the caller's own
    /// informed judgment, never implied merely by requesting TCP.
    #[tokio::test]
    async fn router_tcp_allows_execute_with_a_passphrase_when_explicitly_opted_in() {
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router_tcp(state, "s3cr3t-token".to_string(), true);

        let req = Request::put(format!("/einmo/{session}/cases/a.foo/decision"))
            .header("authorization", "Bearer s3cr3t-token")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"kind": "promote", "to": "verified"}))
                    .unwrap(),
            ))
            .unwrap();
        app.clone().oneshot(req).await.unwrap();

        let req = Request::post(format!("/einmo/{session}/execute"))
            .header("authorization", "Bearer s3cr3t-token")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(
                    &serde_json::json!({"confirm": "PROMOTE", "passphrase": "s3cr3t"}),
                )
                .unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(tmp.path().join("verified/a.foo.einmo").exists());
    }

    #[tokio::test]
    async fn serve_tcp_refuses_a_non_loopback_address() {
        let state = Arc::new(AppState::default());
        let app = router(state);
        let addr: std::net::SocketAddr = "0.0.0.0:0".parse().unwrap();
        let never = std::future::pending();
        let result = serve_tcp(app, addr, never).await;
        assert!(result.is_err(), "a non-loopback bind must be refused");
    }

    /// End-to-end over a real TCP loopback socket, mirroring
    /// `serve_uds_end_to_end_and_cleans_up_on_shutdown`'s harness shape:
    /// bind on an OS-assigned port (`:0`), request with a plain HTTP client,
    /// confirm both the bearer-token gate and a normal response.
    #[tokio::test]
    async fn serve_tcp_end_to_end_enforces_the_bearer_token() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router_tcp(state, "s3cr3t-token".to_string(), false);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let shutdown = async move {
            let _ = shutdown_rx.await;
        };
        let serve_handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown)
                .await
        });

        let client: hyper_util::client::legacy::Client<_, axum::body::Body> =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let uri: hyper::Uri = format!("http://{addr}/einmo/{session}/cases")
            .parse()
            .unwrap();
        let unauthorized = client.get(uri.clone()).await.unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let req = hyper::Request::builder()
            .uri(uri)
            .header("authorization", "Bearer s3cr3t-token")
            .body(axum::body::Body::empty())
            .unwrap();
        let authorized = client.request(req).await.unwrap();
        assert_eq!(authorized.status(), StatusCode::OK);

        shutdown_tx.send(()).unwrap();
        serve_handle.await.unwrap().unwrap();
    }

    // --- private socket path (EIMP-1 §S.7a) --------------------------------

    /// Serializes every test that touches the process-global
    /// `EINMO_REVIEW_PRIVATE_DIR`, mirroring `journal.rs`'s own `ENV_LOCK`
    /// discipline — without this, two tests setting/restoring the same env
    /// var while `cargo test`'s default parallel runner interleaves them
    /// would race (one test's `private_socket_path()` call could resolve
    /// under the OTHER test's base dir mid-flight).
    static PRIVATE_SOCKET_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[tokio::test]
    async fn private_socket_path_is_hardened_and_unique() {
        use std::os::unix::fs::PermissionsExt;

        let _guard = PRIVATE_SOCKET_ENV_LOCK.lock().unwrap();
        let base_dir = tempfile::tempdir().unwrap();
        // SAFETY: serialized by PRIVATE_SOCKET_ENV_LOCK above.
        unsafe {
            std::env::set_var(PRIVATE_SOCKET_BASE_ENV, base_dir.path());
        }

        let first = private_socket_path().unwrap();
        let second = private_socket_path().unwrap();
        assert_ne!(first, second, "each call mints a fresh, unpredictable path");
        assert!(first.starts_with(base_dir.path()));

        let dir = first.parent().unwrap();
        let mode = std::fs::metadata(dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "the containing directory must be mode 0700");

        // The BASE dir must be hardened too, not just the per-session leaf
        // -- under a real umask (e.g. 022) a bare `create_dir_all` would
        // leave it world-readable/executable, letting any local user list
        // it and see every "unpredictable" session directory's name.
        let base_mode = std::fs::metadata(base_dir.path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(base_mode, 0o700, "the base directory must be mode 0700 too");

        // SAFETY: cleanup for this test's own override, same scope as above.
        unsafe {
            std::env::remove_var(PRIVATE_SOCKET_BASE_ENV);
        }
    }

    /// A private socket path is only ever a *reservation* — [`serve_uds`]
    /// binds it exactly like any other socket path, proving the two halves
    /// of `EIMP-1` §S.7a's design (mint the path; bind it) actually compose.
    #[tokio::test]
    async fn private_socket_path_can_be_served_over() {
        let _guard = PRIVATE_SOCKET_ENV_LOCK.lock().unwrap();
        let base_dir = tempfile::tempdir().unwrap();
        // SAFETY: serialized by PRIVATE_SOCKET_ENV_LOCK above.
        unsafe {
            std::env::set_var(PRIVATE_SOCKET_BASE_ENV, base_dir.path());
        }
        let socket_path = private_socket_path().unwrap();
        // SAFETY: serialized by PRIVATE_SOCKET_ENV_LOCK above.
        unsafe {
            std::env::remove_var(PRIVATE_SOCKET_BASE_ENV);
        }
        // Env-var access is done — release the lock before any `.await`
        // point below (holding a std `MutexGuard` across `.await` is a
        // clippy deny in this workspace, and there's nothing left here that
        // needs the lock's protection).
        drop(_guard);

        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        state.create_session(tmp.path());
        let app = router(state);
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let shutdown = async move {
            let _ = shutdown_rx.await;
        };
        let socket_path_for_serve = socket_path.clone();
        let serve_handle =
            tokio::spawn(async move { serve_uds(app, &socket_path_for_serve, shutdown).await });

        for _ in 0..50 {
            if socket_path.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(socket_path.exists(), "the minted private socket must bind");

        shutdown_tx.send(()).unwrap();
        serve_handle.await.unwrap().unwrap();
    }

    // --- concurrency (EIMP-1 §S.7, plan: "N verifiers, single-flight verify
    //     counts, no lost updates, claims expire") ---------------------------

    /// N concurrent `GET .../body/<stage>` requests for the SAME artifact,
    /// through the HTTP server, must still trigger exactly one underlying
    /// verification — `VerifiedCache`'s single-flight guarantee (proved at
    /// the library level in `review.rs`) exercised through the actual HTTP
    /// handlers concurrently, per this plan item's own framing.
    #[tokio::test]
    async fn concurrent_body_requests_single_flight_verify_exactly_once() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let review = {
            // Peek the review out of state before spawning -- state.get is
            // private within this module, fine to call directly here.
            state.get(session).unwrap()
        };
        let app = router(state);

        let mut handles = Vec::new();
        for _ in 0..16 {
            let app = app.clone();
            handles.push(tokio::spawn(async move {
                let req = Request::get(format!("/einmo/{session}/cases/a.foo/body/output"))
                    .body(Body::empty())
                    .unwrap();
                app.oneshot(req).await.unwrap().status()
            }));
        }
        for handle in handles {
            assert_eq!(handle.await.unwrap(), StatusCode::OK);
        }

        assert_eq!(
            review.cache_verify_count(),
            1,
            "16 concurrent HTTP reads of the same artifact must verify exactly once"
        );
    }

    /// Many verifiers deciding many DIFFERENT cases concurrently must lose
    /// none of them — `plan()` afterward must reflect every decision, not a
    /// subset a race clobbered. Uses a suite seeded with several cases so
    /// each concurrent PUT targets a distinct `EinmoId` (simulating N
    /// verifiers working the worklist in parallel, `EIMP-1` §S.5's
    /// concurrency semantics, exercised at the HTTP layer).
    #[tokio::test]
    async fn concurrent_decisions_on_distinct_cases_lose_none() {
        let tmp = tempfile::tempdir().unwrap();
        const N: usize = 20;
        for i in 0..N {
            write_input(tmp.path(), &format!("case-{i}.foo"), "{1+1;}");
        }
        let config =
            crate::config::TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        suite.evaluate_all(&Echo).unwrap();

        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let app = router(state);

        let mut handles = Vec::new();
        for i in 0..N {
            let app = app.clone();
            handles.push(tokio::spawn(async move {
                let req = Request::put(format!("/einmo/{session}/cases/case-{i}.foo/decision"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(
                            &serde_json::json!({"kind": "promote", "to": "checked"}),
                        )
                        .unwrap(),
                    ))
                    .unwrap();
                app.oneshot(req).await.unwrap().status()
            }));
        }
        for handle in handles {
            assert_eq!(handle.await.unwrap(), StatusCode::OK);
        }

        let req = Request::get(format!("/einmo/{session}/plan"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let plan: PlanResponse = body_json(resp).await;
        assert_eq!(
            plan.actions.len(),
            N,
            "every one of {N} concurrent decisions must survive, none lost to a race"
        );
    }

    /// A claim's TTL is honored through the HTTP endpoint too: a short-TTL
    /// claim disappears from `plan().claims` once it expires, exactly as
    /// `review.rs`'s own library-level claim-expiry tests already prove for
    /// `claim_for` directly -- this is the same behavior reached through
    /// `POST .../claim`.
    #[tokio::test]
    async fn claim_via_http_expires_and_is_auto_reclaimed() {
        let tmp = seeded_suite();
        let state = Arc::new(AppState::default());
        let session = state.create_session(tmp.path());
        let review = state.get(session).unwrap();
        let app = router(state);

        let id = EinmoId::try_from("a.foo").unwrap();
        review.claim_for(&id, std::time::Duration::from_millis(20));
        assert_eq!(
            review.plan().claims.len(),
            1,
            "the claim must be active immediately"
        );

        tokio::time::sleep(std::time::Duration::from_millis(60)).await;

        let req = Request::get(format!("/einmo/{session}/plan"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let plan: PlanResponse = body_json(resp).await;
        assert!(
            plan.claims.is_empty(),
            "an expired claim must be auto-reclaimed in the HTTP response, not linger"
        );
        assert!(
            review.plan().claims.is_empty(),
            "an expired claim must be auto-reclaimed, not linger"
        );
    }
}
