pub struct PriceUtils;

impl PriceUtils {
    pub fn label(cents: i64) -> String {
        format!("${}", Self::dollars(cents))
    }

    fn dollars(cents: i64) -> String {
        format!("{:.2}", cents as f64 / 100.0)
    }
}
