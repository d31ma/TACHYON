struct PureValidator {}

impl PureValidator {
    fn check(&self, #[Check(ProductInput)] input: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(input) }
}

