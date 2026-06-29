use speed_on_core::{IndexedResource, ResourceKind, SearchAliasBuilder, SearchAliasKind};

fn app_resource() -> IndexedResource {
    IndexedResource {
        id: "app-terminal".to_owned(),
        kind: ResourceKind::Application,
        title: "Terminal".to_owned(),
        target: "/apps/terminal".to_owned(),
        icon_path: None,
        source: "test".to_owned(),
        first_seen_at_millis: 1,
        last_seen_at_millis: 1,
    }
}

#[test]
fn builder_creates_title_and_target_entries() {
    let builder = SearchAliasBuilder::default();
    let entries = builder.aliases_for_resource(&app_resource());

    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|entry| entry.kind == SearchAliasKind::Title));
    assert!(entries.iter().any(|entry| entry.kind == SearchAliasKind::Target));
}
