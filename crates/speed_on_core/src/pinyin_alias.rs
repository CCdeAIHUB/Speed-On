use crate::alias::{PinyinAliasProvider, PinyinAliases};

#[derive(Debug, Clone, Copy, Default)]
pub struct PinyinCrateAliasProvider;

impl PinyinAliasProvider for PinyinCrateAliasProvider {
    fn aliases_for_title(&self, _title: &str) -> PinyinAliases {
        PinyinAliases::empty()
    }
}
