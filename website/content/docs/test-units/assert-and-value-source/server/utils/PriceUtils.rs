pub struct PriceUtils;

impl PriceUtils {
    pub fn with_tax(cents: i64) -> Result<i64, String> {
        Self::require_positive(cents)?;
        Ok(cents * 120 / 100)
    }

    fn require_positive(cents: i64) -> Result<(), String> {
        if cents < 0 {
            return Err(String::from("A price is never negative."));
        }
        Ok(())
    }
}
