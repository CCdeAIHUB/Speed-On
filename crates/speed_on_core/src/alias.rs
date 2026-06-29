use std::collections::HashSet;

use crate::domain::IndexedResource;
use crate::pinyin_alias::PinyinCrateAliasProvider;
use crate::search::{normalize_search_query, SearchAlias, SearchAliasKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinyinAliases {
    pub full: Option<String>,
    pub initials: Option<String>,
}

impl PinyinAliases {
    pub fn empty() -> Self {
        Self {
            full: None,
            initials: None,
        }
    }
}

pub trait PinyinAliasProvider {
    fn aliases_for_title(&self, title: &str) -> PinyinAliases;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultPinyinAliasProvider;

impl PinyinAliasProvider for DefaultPinyinAliasProvider {
    fn aliases_for_title(&self, title: &str) -> PinyinAliases {
        PinyinCrateAliasProvider.aliases_for_title(title)
    }
}

/// Delegate `PinyinAliasProvider` through references so that `&dyn PinyinAliasProvider`
/// and `Box<dyn PinyinAliasProvider>` can be used wherever a `P: PinyinAliasProvider` is expected.
impl<P: PinyinAliasProvider + ?Sized> PinyinAliasProvider for &P {
    fn aliases_for_title(&self, title: &str) -> PinyinAliases {
        (**self).aliases_for_title(title)
    }
}

impl<P: PinyinAliasProvider + ?Sized> PinyinAliasProvider for Box<P> {
    fn aliases_for_title(&self, title: &str) -> PinyinAliases {
        (**self).aliases_for_title(title)
    }
}

#[derive(Debug, Clone)]
pub struct SearchAliasBuilder<P>
where
    P: PinyinAliasProvider,
{
    pinyin_provider: P,
}

impl Default for SearchAliasBuilder<DefaultPinyinAliasProvider> {
    fn default() -> Self {
        Self::new(DefaultPinyinAliasProvider)
    }
}

impl<P> SearchAliasBuilder<P>
where
    P: PinyinAliasProvider,
{
    pub fn new(pinyin_provider: P) -> Self {
        Self { pinyin_provider }
    }

    pub fn aliases_for_resource(&self, resource: &IndexedResource) -> Vec<SearchAlias> {
        let mut aliases = Vec::new();
        let mut seen = HashSet::new();

        // Title and target aliases are the minimum search contract for every
        // indexed resource. They let newly scanned applications become searchable
        // immediately, before optional pinyin/browser metadata builders run.
        push_unique_alias(
            &mut aliases,
            &mut seen,
            SearchAliasKind::Title,
            &resource.title,
        );
        push_unique_alias(
            &mut aliases,
            &mut seen,
            SearchAliasKind::Target,
            &resource.target,
        );

        let pinyin_aliases = self.pinyin_provider.aliases_for_title(&resource.title);
        if let Some(full) = pinyin_aliases.full {
            push_unique_alias(&mut aliases, &mut seen, SearchAliasKind::PinyinFull, &full);
        }
        if let Some(initials) = pinyin_aliases.initials {
            push_unique_alias(
                &mut aliases,
                &mut seen,
                SearchAliasKind::PinyinInitials,
                &initials,
            );
        }

        aliases
    }
}

fn push_unique_alias(
    aliases: &mut Vec<SearchAlias>,
    seen: &mut HashSet<(SearchAliasKind, String)>,
    kind: SearchAliasKind,
    value: &str,
) {
    let normalized = normalize_search_query(value);
    if normalized.is_empty() {
        return;
    }

    if seen.insert((kind, normalized)) {
        aliases.push(SearchAlias::new(kind, value));
    }
}
