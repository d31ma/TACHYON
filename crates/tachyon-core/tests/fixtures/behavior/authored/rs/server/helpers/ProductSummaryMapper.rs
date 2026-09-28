trait ProductSummaryMapper {
    #[Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})]
    fn to_summary(&self, product: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}


