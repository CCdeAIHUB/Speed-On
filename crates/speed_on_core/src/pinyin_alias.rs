use pinyin::ToPinyin;

use crate::alias::{PinyinAliasProvider, PinyinAliases};

#[derive(Debug, Clone, Copy, Default)]
pub struct PinyinCrateAliasProvider;

impl PinyinAliasProvider for PinyinCrateAliasProvider {
    fn aliases_for_title(&self, title: &str) -> PinyinAliases {
        let mut full = String::new();
        let mut initials = String::new();
        let mut converted = false;

        for pinyin in title.to_pinyin().flatten() {
            converted = true;
            full.push_str(pinyin.plain());
            initials.push_str(pinyin.first_letter());
        }

        if converted {
            PinyinAliases {
                full: Some(full),
                initials: Some(initials),
            }
        } else {
            PinyinAliases::empty()
        }
    }
}
