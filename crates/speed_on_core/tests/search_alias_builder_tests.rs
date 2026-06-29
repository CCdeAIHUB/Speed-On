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

#[test]
fn builder_generates_pinyin_aliases_for_chinese_titles() {
    // 场景：默认 pinyin provider 应为中文标题生成全拼和首字母别名，
    // 验证 DefaultPinyinAliasProvider（原 NoopPinyinAliasProvider）确实执行了拼音转换。
    let builder = SearchAliasBuilder::default();
    let resource = IndexedResource {
        id: "app-wechat".to_owned(),
        kind: ResourceKind::Application,
        title: "微信".to_owned(),
        target: "/Applications/WeChat.app".to_owned(),
        icon_path: None,
        source: "test".to_owned(),
        first_seen_at_millis: 1,
        last_seen_at_millis: 1,
    };

    let entries = builder.aliases_for_resource(&resource);

    assert_eq!(entries.len(), 4);
    assert!(entries.iter().any(|e| e.kind == SearchAliasKind::Title));
    assert!(entries.iter().any(|e| e.kind == SearchAliasKind::Target));
    assert!(entries.iter().any(|e| e.kind == SearchAliasKind::PinyinFull && e.value == "weixin"));
    assert!(entries.iter().any(|e| e.kind == SearchAliasKind::PinyinInitials && e.value == "wx"));
}
