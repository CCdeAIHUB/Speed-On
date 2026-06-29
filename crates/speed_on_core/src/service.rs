mod index;

pub use index::IndexService;

use crate::domain::{CandidateResource, Recommendation, RecommendationRequest, ResourceKind};
use crate::error::{AppError, AppResult};
use crate::ports::ResourceRepository;
use crate::search::score_recency;

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
        let candidates = self.repository.load_recommendation_candidates(request.kinds.as_deref())?;
        Ok(rank_candidates(candidates, &request))
    }
}

pub fn rank_candidates(candidates: Vec<CandidateResource>, request: &RecommendationRequest) -> Vec<Recommendation> {
    let mut recommendations = candidates
        .into_iter()
        .filter(|candidate| match request.kinds.as_deref() {
            Some(kinds) => kinds.contains(&candidate.resource.kind),
            None => true,
        })
        .map(|candidate| {
            let open_score = candidate.open_count.saturating_mul(100);
            let recent_score = candidate
                .last_opened_at_millis
                .map(|last_opened| score_recency(request.now_millis.saturating_sub(last_opened)))
                .unwrap_or(0);
            let score = open_score.saturating_add(recent_score);
            let reason = match candidate.last_opened_at_millis {
                Some(last_opened) => format!("opened {} times; last opened at {}; score {}", candidate.open_count, last_opened, score),
                None => format!("opened {} times; no last-opened timestamp; score {}", candidate.open_count, score),
            };
            Recommendation { resource: candidate.resource, score, reason }
        })
        .collect::<Vec<_>>();

    recommendations.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.resource.last_seen_at_millis.cmp(&left.resource.last_seen_at_millis))
            .then_with(|| left.resource.title.cmp(&right.resource.title))
    });
    recommendations.truncate(request.limit);
    recommendations
}
