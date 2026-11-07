pub struct Pulse {
    pub amount: i64,
}

pub struct Relay {
    pub amount: i64,
}

pub struct Cycle {
    pub amount: i64,
}

pub struct Isolation {
    pub value: i64,
}

pub struct Ordered {
    pub status: String,
    pub total: i64,
}

pub struct Other {
    pub status: String,
    pub total: i64,
}

pub struct Echo {
    pub amount: i64,
}
