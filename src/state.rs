use std::fmt;

use axum::extract::FromRef;
use sqlx::SqlitePool;

use crate::EvaltorArgs;
use crate::auth::DiscoveredClient;
use crate::runner_manager::RunnerManager;

#[derive(Clone)]
pub struct EvaltorState {
    pub db_pool: SqlitePool,
    pub runner_manager: RunnerManager,
    pub oidc_client: DiscoveredClient,
    pub config: EvaltorArgs,
}

impl fmt::Debug for EvaltorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvaltorState")
            .field("db_pool", &self.db_pool)
            .field("runner_manager", &self.runner_manager)
            .finish_non_exhaustive()
    }
}

impl FromRef<EvaltorState> for SqlitePool {
    fn from_ref(state: &EvaltorState) -> Self {
        state.db_pool.clone()
    }
}
