use crate::alias::{NoopPinyinAliasProvider, SearchAliasBuilder};
use crate::domain::{
    CandidateResource, IndexedResource, Recommendation, RecommendationRequest, ResourceKind,
};
use crate::error::{AppError, AppResult};
use crate::ports::{InstalledApplicationScanner, ResourceRepository, SearchAliasRepository};

pub struct IndexService<R, S>
where
    R: ResourceRepository,
    S: InstalledApplicationScanner,
{
    repository: R,
    scanner: S,
}

impl<R, S> IndexService<R, S>
where
    R: ResourceRepository,
    S: InstalledApplicationScanner,
{
    pub fn new(repository: R, scanner: S) -> Self {
        Self { repository, scanner }
    }

    pub fn refresh_installed_applications(&mut self) -> AppResult<usize> {
        Ok(self.refresh_installed_application_resources()?.len())
    }

    pub fn refresh_installed_application_resources(&mut self) -> AppResult<Vec<IndexedResource>> {
        let resources = self.scanner.scan_installed_applications()?;
        self.repository.upsert_resources(&resources)?;
        Ok(resources)
    }
}

impl<R, S> IndexService<R, S>
where
    R: ResourceRepository + SearchAliasRepository,
    S: InstalledApplicationScanner,
{
    pub fn refresh_installed_applications_with_aliases(
        &mut self,
        created_at_millis: u64,
    ) -> AppResult<(usize, usize)> {
        let resources = self.refresh_installed_application_resources()?;
        let builder = SearchAliasBuilder::new(NoopPinyinAliasProvider);
        let mut alias_count = 0;

        for resource in &resources {
            let aliases = builder.aliases_for_resource(resource);
            alias_count += aliases.len();
            self.repository
                .upsert_search_aliases(&resource.id, &aliases, created_at_millis)?;
        }

        Ok((resources.len(), alias_count))
    }
}

pub struct RecommendationService<R>
where
    R: ResourceRepository,
{
    repository: R,
}

impl<R> RecommendationService<R>
where
    R: ResourceRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn recommend(&self, request: RecommendationRequest) -> AppResult<Vec<Recommendation>> {
        if request.limit == 0 {
            return Err(AppError::invalid_argument(
                "recommendation limit must be greater than zero",
                "service::RecommendationService",
            ));
        }

        let candidates = self
            .repository
            .load_recommendation_candidates(request.kinds.as_deref())?;

        Ok(rank_candidates(candidates, &request))
    }
}

pub fn rank_candidates(
    candidates: Vec<CandidateResource>,
    request: &RecommendationRequest,
) -> Vec<Recommendation> {
    let mut recommendations = candidates
        .into_iter()
        .filter(|candidate| resource_kind_allowed(candidate.resource.kind, request.kinds.as_deref()))
        .map(|candidate| {
            let (score, reason) = calculate_score(
                candidate.open_count,
                candidate.last_opened_at_millis,
                request.now_millis,
            );

            Recommendation {
                resource: candidate.resource,
                score,
                reason,
            }
        })
        .collect::<Vec<_>>();

    recommendations.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| {
                right
                    .resource
                    .last_seen_at_millis
                    .cmp(&left.resource.last_seen_at_millis)
            })
            .then_with(|| left.resource.title.cmp(&right.resource.title))
    });

    recommendations.truncate(request.limit);
    recommendations
}

fn resource_kind_allowed(kind: ResourceKind, allowed: Option<&[ResourceKind]>) -> bool {
    match allowed {
        Some(kinds) => kinds.contains(&kind),
        None => true,
    }
}

fn calculate_score(
    open_count: u64,
    last_opened_at_millis: Option<u64>,
    now_millis: u64,
) -> (u64, String) {
    let open_count_score = open_count.saturating_mul(100);
    let recency_score = match last_opened_at_millis {
        Some(last_opened) => {
            // System clocks can move backwards or events can be imported from a
            // newer profile snapshot. Saturating subtraction keeps ranking
            // deterministic without hiding the anomaly in storage or adapters.
            let age_millis = now_millis.saturating_sub(last_opened);
            score_recency(age_millis)
        }
        None => 0,
    };

    let score = open_count_score.saturating_add(recency_score);
    let reason = match last_opened_at_millis {
        Some(last_opened) => format!(
            "opened {open_count} times; last opened at {last_opened}; score {score}"
        ),
        None => format!("opened {open_count} times; no last-opened timestamp; score {score}"),
    };

    (score, reason)
}

fn score_recency(age_millis: u64) -> u64 {
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
