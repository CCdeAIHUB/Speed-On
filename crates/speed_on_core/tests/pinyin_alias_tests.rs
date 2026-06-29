use speed_on_core::{PinyinAliasProvider, PinyinCrateAliasProvider};

fn char_from_codepoint(codepoint: u32) -> char {
    match char::from_u32(codepoint) {
        Some(value) => value,
        None => panic!("invalid unicode codepoint: {codepoint}"),
    }
}

#[test]
fn pinyin_crate_provider_generates_plain_and_initial_aliases() {
    // Scenario: pinyin provider must generate full pinyin and initials for Chinese titles.
    let title = [char_from_codepoint(0x4e2d), char_from_codepoint(0x56fd)]
        .iter()
        .collect::<String>();

    let aliases = PinyinCrateAliasProvider.aliases_for_title(&title);

    assert_eq!(aliases.full, Some("zhongguo".to_owned()));
    assert_eq!(aliases.initials, Some("zg".to_owned()));
}
