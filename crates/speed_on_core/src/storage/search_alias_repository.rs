use crate::error::AppResult;
use crate::ports::SearchAliasRepository;
use crate::search::SearchAlias;
use crate::storage::SqliteStore;

impl SearchAliasRepository for SqliteStore {
    fn upsert_search_aliases(
        &mut self,
        resource_id: &str,
        aliases: &[SearchAlias],
        created_at_millis: u64,
    ) -> AppResult<()> {
        SqliteStore::upsert_search_aliases(self, resource_id, aliases, created_at_millis)
    }
}
