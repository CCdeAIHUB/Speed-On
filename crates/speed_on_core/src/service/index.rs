use crate::domain::IndexedResource;
use crate::error::AppResult;
use crate::ports::{InstalledApplicationScanner, ResourceRepository};

pub struct IndexService<R, S>
where
    R: ResourceRepository,
    S: InstalledApplicationScanner,
{
    repository: R,
    scanner: S,
}

impl<R, S> IndexService<R, S>
where
    R: ResourceRepository,
    S: InstalledApplicationScanner,
{
    pub fn new(repository: R, scanner: S) -> Self {
        Self { repository, scanner }
    }

    pub fn refresh_installed_applications(&mut self) -> AppResult<usize> {
        let resources = self.scanner.scan_installed_applications()?;
        let count = resources.len();
        self.repository.upsert_resources(&resources)?;
        Ok(count)
    }

    pub fn refresh_installed_application_resources(mut self) -> AppResult<Vec<IndexedResource>> {
        let resources = self.scanner.scan_installed_applications()?;
        self.repository.upsert_resources(&resources)?;
        Ok(resources)
    }
}
