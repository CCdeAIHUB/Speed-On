use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::domain::{IndexedResource, ResourceKind};
use crate::error::{AppError, AppResult};
use crate::logging::{UserSearchLogEntry, UserSelectionLogEntry};
use crate::ports::{SearchIndexRepository, UserOperationLogRepository};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchAliasKind {
    Title,
    Target,
    BrowserTitle,
    PinyinFull,
    PinyinInitials,
    Custom,
}

impl SearchAliasKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Target => "target",
            Self::BrowserTitle => "browser_title",
            Self::PinyinFull => "pinyin_full",
            Self::PinyinInitials => "pinyin_initials",
            Self::Custom => "custom",
        }
    }
}

impl TryFrom<&str> for SearchAliasKind {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "title" => Ok(Self::Title),
            "target" => Ok(Self::Target),
            "browser_title" => Ok(Self::BrowserTitle),
            "pinyin_full" => Ok(Self::PinyinFull),
            "pinyin_initials" => Ok(Self::PinyinInitials),
            "custom" => Ok(Self::Custom),
            _ => Err(AppError::invalid_argument(
                format!("unknown search alias kind: {value}"),
                "search::SearchAliasKind",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAlias {
    pub kind: SearchAliasKind,
    pub value: String,
}

impl SearchAlias {
    pub fn new(kind: SearchAliasKind, value: impl Into<String>) -> Self {
        Self {
            kind,
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSelectionSignal {
    pub normalized_query: String,
    pub selection_count: u64,
    pub last_selected_at_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchCandidate {
    pub resource: IndexedResource,
    pub aliases: Vec<SearchAlias>,
    pub user_selection_signals: Vec<UserSelectionSignal>,
    pub open_count: u64,
    pub last_opened_at_millis: Option<u64>,
}

impl SearchCandidate {
    pub fn new(resource: IndexedResource) -> Self {
        Self {
            resource,
            aliases: Vec::new(),
            user_selection_signals: Vec::new(),
            open_count: 0,
            last_opened_at_millis: None,
        }
    }

    pub fn with_aliases(mut self, aliases: Vec<SearchAlias>) -> Self {
        self.aliases = aliases;
        self
    }

    pub fn with_user_selection_signals(mut self, signals: Vec<UserSelectionSignal>) -> Self {
        self.user_selection_signals = signals;
        self
    }

    pub fn with_usage(mut self, open_count: u64, last_opened_at_millis: Option<u64>) -> Self {
        self.open_count = open_count;
        self.last_opened_at_millis = last_opened_at_millis;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequest {
    pub query: String,
    pub limit: usize,
    pub kinds: Option<Vec<ResourceKind>>,
    pub now_millis: u64,
}

impl SearchRequest {
    pub fn new(query: impl Into<String>, limit: usize, now_millis: u64) -> Self {
        Self {
            query: query.into(),
            limit,
            kinds: None,
            now_millis,
        }
    }

    pub fn with_kinds(mut self, kinds: Vec<ResourceKind>) -> Self {
        self.kinds = Some(kinds);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMatchKind {
    UserHistory,
    Title,
    Target,
    BrowserTitle,
    PinyinFull,
    PinyinInitials,
    CustomAlias,
}

impl SearchMatchKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserHistory => "user_history",
            Self::Title => "title",
            Self::Target => "target",
            Self::BrowserTitle => "browser_title",
            Self::PinyinFull => "pinyin_full",
            Self::PinyinInitials => "pinyin_initials",
            Self::CustomAlias => "custom_alias",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub resource: IndexedResource,
    pub score: u64,
    pub match_kind: SearchMatchKind,
    pub reason: String,
}

pub struct SearchService<R>
where
    R: SearchIndexRepository + UserOperationLogRepository,
{
    repository: R,
}

impl<R> SearchService<R>
where
    R: SearchIndexRepository + UserOperationLogRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn search(&mut self, request: SearchRequest) -> AppResult<Vec<SearchResult>> {
        let normalized_query = normalize_search_query(&request.query);
        if normalized_query.is_empty() {
            return Err(AppError::invalid_argument(
                "search query must not be empty",
                "search::SearchService",
            ));
        }
        if request.limit == 0 {
            return Err(AppError::invalid_argument(
                "search limit must be greater than zero",
                "search::SearchService",
            ));
        }

        let candidates = self
            .repository
            .load_search_candidates(request.kinds.as_deref())?;
        let results = rank_search_candidates(candidates, &request, &normalized_query);
        self.repository.record_user_search(&UserSearchLogEntry {
            id: build_log_id("search", request.now_millis, &normalized_query),
            raw_query: request.query,
            normalized_query,
            result_count: results.len(),
            searched_at_millis: request.now_millis,
        })?;
        Ok(results)
    }

    pub fn record_selection(
        &mut self,
        raw_query: impl Into<String>,
        selected: &IndexedResource,
        selected_rank: usize,
        opened_at_millis: u64,
    ) -> AppResult<()> {
        let raw_query = raw_query.into();
        let normalized_query = normalize_search_query(&raw_query);
        if normalized_query.is_empty() {
            return Err(AppError::invalid_argument(
                "selection query must not be empty",
                "search::SearchService",
            ));
        }
        if selected_rank == 0 {
            return Err(AppError::invalid_argument(
                "selected rank must be one-based and greater than zero",
                "search::SearchService",
            ));
        }
        self.repository
            .record_user_selection(&UserSelectionLogEntry {
                id: build_log_id("selection", opened_at_millis, &normalized_query),
                raw_query,
                normalized_query,
                selected_resource_id: selected.id.clone(),
                selected_kind: selected.kind,
                selected_title: selected.title.clone(),
                selected_target: selected.target.clone(),
                selected_rank,
                opened_at_millis,
            })
    }
}

pub fn rank_search_candidates(
    candidates: Vec<SearchCandidate>,
    request: &SearchRequest,
    normalized_query: &str,
) -> Vec<SearchResult> {
    let mut seen_ids = HashSet::new();
    let mut results = candidates
        .into_iter()
        .filter(|candidate| kind_allowed(candidate.resource.kind, request.kinds.as_deref()))
        .filter_map(|candidate| score_candidate(candidate, normalized_query, request.now_millis))
        .filter(|result| seen_ids.insert(result.resource.id.clone()))
        .collect::<Vec<_>>();

    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.resource.title.cmp(&right.resource.title))
    });
    results.truncate(request.limit);
    results
}

pub fn normalize_search_query(value: &str) -> String {
    value
        .trim()
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn kind_allowed(kind: ResourceKind, allowed: Option<&[ResourceKind]>) -> bool {
    match allowed {
        Some(kinds) => kinds.contains(&kind),
        None => true,
    }
}

fn score_candidate(
    candidate: SearchCandidate,
    normalized_query: &str,
    now_millis: u64,
) -> Option<SearchResult> {
    let history = best_history_match(&candidate, normalized_query, now_millis);
    let alias = best_alias_match(&candidate, normalized_query);
    match (history, alias) {
        (Some(history), Some(alias)) if history.score >= alias.score => Some(SearchResult {
            resource: candidate.resource,
            score: history.score.saturating_add(alias.score / 10),
            match_kind: SearchMatchKind::UserHistory,
            reason: format!(
                "{}; also matched {}",
                history.reason,
                alias.match_kind.as_str()
            ),
        }),
        (Some(history), Some(alias)) => Some(SearchResult {
            resource: candidate.resource,
            score: alias.score.saturating_add(history.score / 10),
            match_kind: alias.match_kind,
            reason: format!("{}; also matched user history", alias.reason),
        }),
        (Some(history), None) => Some(SearchResult {
            resource: candidate.resource,
            score: history.score,
            match_kind: SearchMatchKind::UserHistory,
            reason: history.reason,
        }),
        (None, Some(alias)) => Some(SearchResult {
            resource: candidate.resource,
            score: alias
                .score
                .saturating_add(candidate.open_count.saturating_mul(5)),
            match_kind: alias.match_kind,
            reason: alias.reason,
        }),
        (None, None) => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CandidateScore {
    score: u64,
    match_kind: SearchMatchKind,
    reason: String,
}

fn best_history_match(
    candidate: &SearchCandidate,
    normalized_query: &str,
    now_millis: u64,
) -> Option<CandidateScore> {
    candidate
        .user_selection_signals
        .iter()
        .filter(|signal| queries_are_similar(normalized_query, &signal.normalized_query))
        .map(|signal| {
            let score = 2_000_u64
                .saturating_add(signal.selection_count.saturating_mul(100))
                .saturating_add(score_recency(
                    now_millis.saturating_sub(signal.last_selected_at_millis),
                ));
            CandidateScore {
                score,
                match_kind: SearchMatchKind::UserHistory,
                reason: format!(
                    "previously selected {} times for a similar query; score {score}",
                    signal.selection_count
                ),
            }
        })
        .max_by(|left, right| left.score.cmp(&right.score))
}

fn best_alias_match(candidate: &SearchCandidate, normalized_query: &str) -> Option<CandidateScore> {
    candidate
        .aliases
        .iter()
        .filter_map(|alias| score_alias(alias, normalized_query))
        .max_by(|left, right| left.score.cmp(&right.score))
}

fn score_alias(alias: &SearchAlias, normalized_query: &str) -> Option<CandidateScore> {
    let normalized_alias = normalize_search_query(&alias.value);
    if normalized_alias.is_empty() || !normalized_alias.contains(normalized_query) {
        return None;
    }
    let exact_bonus = if normalized_alias == normalized_query {
        200
    } else {
        0
    };
    let prefix_bonus = if normalized_alias.starts_with(normalized_query) {
        100
    } else {
        0
    };
    let base_score = match alias.kind {
        SearchAliasKind::Title => 700,
        SearchAliasKind::BrowserTitle => 650,
        SearchAliasKind::PinyinInitials => 620,
        SearchAliasKind::PinyinFull => 600,
        SearchAliasKind::Target => 450,
        SearchAliasKind::Custom => 400,
    };
    let score = base_score + exact_bonus + prefix_bonus;
    Some(CandidateScore {
        score,
        match_kind: match alias.kind {
            SearchAliasKind::Title => SearchMatchKind::Title,
            SearchAliasKind::Target => SearchMatchKind::Target,
            SearchAliasKind::BrowserTitle => SearchMatchKind::BrowserTitle,
            SearchAliasKind::PinyinFull => SearchMatchKind::PinyinFull,
            SearchAliasKind::PinyinInitials => SearchMatchKind::PinyinInitials,
            SearchAliasKind::Custom => SearchMatchKind::CustomAlias,
        },
        reason: format!("matched {}; score {}", alias.kind.as_str(), score),
    })
}

fn queries_are_similar(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    let min_len = left.chars().count().min(right.chars().count());
    min_len >= 2 && (left.contains(right) || right.contains(left))
}

pub(crate) fn score_recency(age_millis: u64) -> u64 {
    const HOUR: u64 = 60 * 60 * 1_000;
    const DAY: u64 = 24 * HOUR;
    const WEEK: u64 = 7 * DAY;
    const MONTH: u64 = 30 * DAY;
    if age_millis <= HOUR {
        80
    } else if age_millis <= DAY {
        60
    } else if age_millis <= WEEK {
        40
    } else if age_millis <= MONTH {
        20
    } else {
        5
    }
}

/// Global monotonic counter used to disambiguate log IDs generated within the
/// same millisecond for the same normalized query.  Without this, two rapid
/// searches with identical text would collide on the `user_search_logs`
/// primary key and cause the second search to fail.
static LOG_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn build_log_id(prefix: &str, millis: u64, normalized_query: &str) -> String {
    let seq = LOG_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{millis}-{seq}-{normalized_query}")
}
