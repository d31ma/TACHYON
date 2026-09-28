struct Ledger {
    #[Needs]
    behaviors: BehaviorContext,
}

impl Ledger {
    #[AllOrNothing({"adapter":"data"})]
    fn post(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        self.behaviors.repository::<ProductEntity>()?.save(BehaviorValue::object([("id", id)]))
    }

    #[AllOrNothing({"adapter":"data"})]
    fn refuse(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let _ = self.behaviors.repository::<ProductEntity>()?.save(BehaviorValue::object([("id", id)]))?;
        Err(BehaviorError { code: "validation", message: "the ledger refused the entry", path: None })
    }
}

