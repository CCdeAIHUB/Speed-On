use speed_on_core::{
    ActivityRecord, AppResult, CandidateResource, IndexedResource, RecommendationRequest,
    RecommendationService, ResourceKind, ResourceRepository,
};

#[derive(Clone)]
struct InMemoryRepository {
    candidates: Vec<CandidateResource>,
}

impl ResourceRepository for InMemoryRepository {
    fn upsert_resources(&mut self, _resources: &[IndexedResource]) -> AppResult<()> {
        Ok(())
    }

    fn record_activity(&mut self, _activity: &ActivityRecord) -> AppResult<()> {
        Ok(())
    }

    fn load_recommendation_candidates(
        &self,
        kinds: Option<&[ResourceKind]>,
    ) -> AppResult<Vec<CandidateResource>> {
        let candidates = self
            .candidates
            .iter()
            .filter(|candidate| match kinds {
                Some(kinds) => kinds.contains(&candidate.resource.kind),
                None => true,
            })
            .cloned()
            .collect();

        Ok(candidates)
    }
}

fn resource(id: &str, kind: ResourceKind, title: &str, target: &str) -> IndexedResource {
    IndexedResource {
        id: id.to_owned(),
        kind,
        title: title.to_owned(),
        target: target.to_owned(),
        icon_path: None,
        source: "test".to_owned(),
        first_seen_at_millis: 1,
        last_seen_at_millis: 1,
    }
}

fn recommend_or_panic(
    service: &RecommendationService<InMemoryRepository>,
    request: RecommendationRequest,
) -> Vec<speed_on_core::Recommendation> {
    match service.recommend(request) {
        Ok(recommendations) => recommendations,
        Err(error) => panic!("recommendation failed unexpectedly: {error}"),
    }
}

#[test]
fn recommendations_respect_requested_limit() {
    // 场景：前端只请求 2 个结果时，后端不能返回更多候选项，避免 UI 被过量数据污染。
    let now = 1_000_000;
    let repository = InMemoryRepository {
        candidates: vec![
            CandidateResource::new(
                resource("app-a", ResourceKind::Application, "App A", "/apps/a"),
                10,
                Some(now - 1_000),
            ),
            CandidateResource::new(
                resource("app-b", ResourceKind::Application, "App B", "/apps/b"),
                9,
                Some(now - 1_000),
            ),
            CandidateResource::new(
                resource("app-c", ResourceKind::Application, "App C", "/apps/c"),
                8,
                Some(now - 1_000),
            ),
        ],
    };

    let service = RecommendationService::new(repository);
    let recommendations = recommend_or_panic(&service, RecommendationRequest::new(2, now));

    assert_eq!(recommendations.len(), 2);
    assert_eq!(recommendations[0].resource.id, "app-a");
    assert_eq!(recommendations[1].resource.id, "app-b");
}

#[test]
fn recommendations_filter_by_requested_resource_kinds() {
    // 场景：前端只需要应用推荐时，文件、文件夹和浏览器地址不能混入返回结果。
    let now = 1_000_000;
    let repository = InMemoryRepository {
        candidates: vec![
            CandidateResource::new(
                resource("app-a", ResourceKind::Application, "App A", "/apps/a"),
                1,
                Some(now - 1_000),
            ),
            CandidateResource::new(
                resource("file-a", ResourceKind::File, "File A", "/docs/a.txt"),
                100,
                Some(now - 1_000),
            ),
            CandidateResource::new(
                resource(
                    "url-a",
                    ResourceKind::BrowserUrl,
                    "Site A",
                    "https://example.com",
                ),
                100,
                Some(now - 1_000),
            ),
        ],
    };

    let service = RecommendationService::new(repository);
    let request = RecommendationRequest::new(10, now).with_kinds(vec![ResourceKind::Application]);
    let recommendations = recommend_or_panic(&service, request);

    assert_eq!(recommendations.len(), 1);
    assert_eq!(recommendations[0].resource.kind, ResourceKind::Application);
}

#[test]
fn recent_activity_breaks_ties_when_open_counts_are_equal() {
    // 场景：两个资源打开次数相同时，更近打开的资源应该排在更前面。
    let now: u64 = 1_000_000_000_000;
    let repository = InMemoryRepository {
        candidates: vec![
            CandidateResource::new(
                resource("old-app", ResourceKind::Application, "Old App", "/apps/old"),
                3,
                Some(now - 40u64 * 24 * 60 * 60 * 1_000),
            ),
            CandidateResource::new(
                resource(
                    "recent-app",
                    ResourceKind::Application,
                    "Recent App",
                    "/apps/recent",
                ),
                3,
                Some(now - 1_000),
            ),
        ],
    };

    let service = RecommendationService::new(repository);
    let recommendations = recommend_or_panic(&service, RecommendationRequest::new(2, now));

    assert_eq!(recommendations[0].resource.id, "recent-app");
    assert!(recommendations[0].score > recommendations[1].score);
}

#[test]
fn zero_limit_returns_structured_error() {
    // 场景：前端传入 0 个推荐数量时，后端必须返回统一 AppError，不能沉默成功或返回空数组掩盖调用错误。
    let repository = InMemoryRepository { candidates: vec![] };
    let service = RecommendationService::new(repository);

    let result = service.recommend(RecommendationRequest::new(0, 1));

    let error = match result {
        Ok(_) => panic!("zero limit should fail"),
        Err(error) => error,
    };

    assert_eq!(error.error_code, "CORE_INVALID_ARGUMENT");
    assert_eq!(error.module, "service::RecommendationService");
    assert!(error.recoverable);
}
