use sqlx::SqlitePool;
use uuid::Uuid;

use crate::nsjail::{Instance, NSJailBlueprint};

#[derive(Clone, Debug)]
pub struct RunnerManager {
    db: SqlitePool,
}

impl RunnerManager {
    pub const fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub fn run_from_blueprint(&self, blueprint: NSJailBlueprint, test_id: Uuid, attempt_id: Uuid) {
        let instance = Instance::new(blueprint);
        instance.spawn(self.db.clone(), test_id, attempt_id);
    }
}
