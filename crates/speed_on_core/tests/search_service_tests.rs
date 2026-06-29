use std::cell::RefCell;
use std::rc::Rc;

use speed_on_core::{
    ActivityRecord, AppResult, CandidateResource, IndexedResource, ResourceKind, SearchAlias,
    SearchAliasKind, SearchCandidate, SearchIndexRepository, SearchMatchKind, SearchRequest,
    SearchService, UserOperationLogRepository, UserSearchLogEntry, UserSelectionLogEntry,
    UserSelectionSignal,
};

#[derive(Clone)]
struct InMemorySearchRepository {
    candidates: Vec<SearchCandidate>,
    searches: Rc<RefCell<Vec<UserSearchLogEntry>>>,
    selections: Rc<RefCell<Vec<UserSelectionLogEntry>>>,
}

impl InMemorySearchRepository {
    fn new(candidates: Vec<SearchCandidate>) -> Self {
        Self {
            candidates,
            searches: Rc::new(RefCell::new(Vec::new())),
            selections: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl SearchIndexRepository for InMemorySearchRepository {
    fn load_search_candidates(
        &self,
        kinds: Option<&[ResourceKind]>,
    ) -> AppResult<Vec<SearchCandidate>> {
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

impl UserOperationLogRepository for InMemorySearchRepository {
    fn record_user_search(&mut self, entry: &UserSearchLogEntry) -> AppResult<()> {
        self.searches.borrow_mut().push(entry.clone());
        Ok(())
    }

    fn record_user_selection(&mut self, entry: &UserSelectionLogEntry) -> AppResult<()> {
        self.selections.borrow_mut().push(entry.clone());
        Ok(())
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

fn title_candidate(id: &str, title: &str, target: &str) -> SearchCandidate {
    let indexed = resource(id, ResourceKind::Application, title, target);
    SearchCandidate::new(indexed).with_aliases(vec![
        SearchAlias::new(SearchAliasKind::Title, title),
        SearchAlias::new(SearchAliasKind::Target, target),
    ])
}

#[test]
fn empty_search_query_returns_structured_error() {
    // 场景：前端传入空搜索内容时，后端必须返回统一错误，不能返回空列表伪装成成功。
    let repository = InMemorySearchRepository::new(Vec::new());
    let mut service = SearchService::new(repository);

    let result = service.search(SearchRequest::new("   ", 10, 1));

    let error = match result {
        Ok(_) => panic!("empty search query should fail"),
        Err(error) => error,
    };

    assert_eq!(error.error_code, "CORE_INVALID_ARGUMENT");
    assert_eq!(error.module, "search::SearchService");
}

#[test]
fn search_matches_title_target_and_browser_title_aliases() {
    // 场景：搜索必须覆盖应用名称、文件名/路径、浏览器地址和地址名称，不允许只搜应用标题。
    let now = 1_000_000;
    let browser_resource = resource(
        "url-rust",
        ResourceKind::BrowserUrl,
        "https://www.rust-lang.org/learn",
        "https://www.rust-lang.org/learn",
    );
    let browser_candidate = SearchCandidate::new(browser_resource).with_aliases(vec![
        SearchAlias::new(SearchAliasKind::Target, "https://www.rust-lang.org/learn"),
        SearchAlias::new(
            SearchAliasKind::BrowserTitle,
            "Rust Programming Language Learn",
        ),
    ]);
    let file_candidate = SearchCandidate::new(resource(
        "file-report",
        ResourceKind::File,
        "report.txt",
        "/Users/test/Documents/report.txt",
    ))
    .with_aliases(vec![
        SearchAlias::new(SearchAliasKind::Title, "report.txt"),
        SearchAlias::new(SearchAliasKind::Target, "/Users/test/Documents/report.txt"),
    ]);
    let repository = InMemorySearchRepository::new(vec![browser_candidate, file_candidate]);
    let mut service = SearchService::new(repository);

    let results = match service.search(SearchRequest::new("learn", 10, now)) {
        Ok(results) => results,
        Err(error) => panic!("search failed unexpectedly: {error}"),
    };

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].resource.id, "url-rust");
    assert_eq!(results[0].match_kind, SearchMatchKind::BrowserTitle);
}

#[test]
fn search_matches_pinyin_initial_aliases() {
    // 场景：中文资源标题需要支持拼音首字母，例如输入 wx 能匹配“微信”。
    let now = 1_000_000;
    let wechat = SearchCandidate::new(resource(
        "app-wechat",
        ResourceKind::Application,
        "微信",
        "/Applications/WeChat.app",
    ))
    .with_aliases(vec![
        SearchAlias::new(SearchAliasKind::Title, "微信"),
        SearchAlias::new(SearchAliasKind::PinyinFull, "weixin"),
        SearchAlias::new(SearchAliasKind::PinyinInitials, "wx"),
    ]);
    let repository = InMemorySearchRepository::new(vec![wechat]);
    let mut service = SearchService::new(repository);

    let results = match service.search(SearchRequest::new("wx", 10, now)) {
        Ok(results) => results,
        Err(error) => panic!("search failed unexpectedly: {error}"),
    };

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].resource.id, "app-wechat");
    assert_eq!(results[0].match_kind, SearchMatchKind::PinyinInitials);
}

#[test]
fn previous_user_selection_for_similar_query_ranks_before_plain_match() {
    // 场景：用户上次输入相同或相近内容后打开的资源，下一次搜索时必须排在普通文本匹配结果前。
    let now = 2_000_000;
    let history_candidate = SearchCandidate::new(resource(
        "app-vscode",
        ResourceKind::Application,
        "Visual Studio Code",
        "/Applications/Visual Studio Code.app",
    ))
    .with_user_selection_signals(vec![UserSelectionSignal {
        normalized_query: "vscode".to_owned(),
        selection_count: 3,
        last_selected_at_millis: now - 1_000,
    }]);
    let plain_candidate = title_candidate("app-vs-tool", "VS Tool", "/apps/vs-tool");
    let repository = InMemorySearchRepository::new(vec![plain_candidate, history_candidate]);
    let mut service = SearchService::new(repository);

    let results = match service.search(SearchRequest::new("vs", 10, now)) {
        Ok(results) => results,
        Err(error) => panic!("search failed unexpectedly: {error}"),
    };

    assert_eq!(results[0].resource.id, "app-vscode");
    assert_eq!(results[0].match_kind, SearchMatchKind::UserHistory);
}

#[test]
fn search_deduplicates_when_history_and_alias_match_same_resource() {
    // 场景：同一个资源同时命中用户历史和普通搜索时，只能出现一次，不能在结果里重复展示。
    let now = 2_000_000;
    let candidate = SearchCandidate::new(resource(
        "app-vscode",
        ResourceKind::Application,
        "Visual Studio Code",
        "/Applications/Visual Studio Code.app",
    ))
    .with_aliases(vec![SearchAlias::new(
        SearchAliasKind::Title,
        "Visual Studio Code",
    )])
    .with_user_selection_signals(vec![UserSelectionSignal {
        normalized_query: "vscode".to_owned(),
        selection_count: 1,
        last_selected_at_millis: now - 1_000,
    }]);
    let repository = InMemorySearchRepository::new(vec![candidate]);
    let mut service = SearchService::new(repository);

    let results = match service.search(SearchRequest::new("vscode", 10, now)) {
        Ok(results) => results,
        Err(error) => panic!("search failed unexpectedly: {error}"),
    };

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].resource.id, "app-vscode");
    assert_eq!(results[0].match_kind, SearchMatchKind::UserHistory);
}

#[test]
fn search_records_frontend_query_log() {
    // 场景：前端传入搜索内容后，用户操作日志必须记录原始 query、归一化 query 和返回结果数量。
    let now = 1_000_000;
    let repository = InMemorySearchRepository::new(vec![title_candidate(
        "app-terminal",
        "Terminal",
        "/System/Applications/Utilities/Terminal.app",
    )]);
    let searches = Rc::clone(&repository.searches);
    let mut service = SearchService::new(repository);

    match service.search(SearchRequest::new(" Term ", 10, now)) {
        Ok(_) => {}
        Err(error) => panic!("search failed unexpectedly: {error}"),
    }

    let logs = searches.borrow();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].raw_query, " Term ");
    assert_eq!(logs[0].normalized_query, "term");
    assert_eq!(logs[0].result_count, 1);
}

#[test]
fn record_selection_logs_final_user_choice() {
    // 场景：用户最终打开了哪个搜索结果必须被详细记录，供后续搜索排序分析使用。
    let now = 1_000_000;
    let selected = resource(
        "app-terminal",
        ResourceKind::Application,
        "Terminal",
        "/System/Applications/Utilities/Terminal.app",
    );
    let repository = InMemorySearchRepository::new(Vec::new());
    let selections = Rc::clone(&repository.selections);
    let mut service = SearchService::new(repository);

    match service.record_selection("term", &selected, 1, now) {
        Ok(_) => {}
        Err(error) => panic!("selection logging failed unexpectedly: {error}"),
    }

    let logs = selections.borrow();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].raw_query, "term");
    assert_eq!(logs[0].selected_resource_id, "app-terminal");
    assert_eq!(logs[0].selected_kind, ResourceKind::Application);
    assert_eq!(logs[0].selected_title, "Terminal");
    assert_eq!(
        logs[0].selected_target,
        "/System/Applications/Utilities/Terminal.app"
    );
    assert_eq!(logs[0].selected_rank, 1);
}

#[allow(dead_code)]
fn _keep_existing_repository_types_referenced(
    _activity: ActivityRecord,
    _candidate: CandidateResource,
) {
}
