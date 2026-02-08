pub struct Points {
    maximum: i64,
    achieved: i64,
}

impl Points {
    #[must_use]
    pub const fn new(maximum: i64, achieved: i64) -> Self {
        Self { maximum, achieved }
    }

    #[must_use]
    pub const fn maximum(&self) -> i64 {
        self.maximum
    }

    #[must_use]
    pub const fn achieved(&self) -> i64 {
        self.achieved
    }
}
