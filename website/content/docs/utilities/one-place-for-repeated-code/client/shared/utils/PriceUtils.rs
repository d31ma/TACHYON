pub struct PriceUtils;

impl PriceUtils {
    pub fn label(cents: u32) -> String {
        format!("${}", Self::dollars(cents))
    }

    fn dollars(cents: u32) -> String {
        format!("{:.2}", f64::from(cents) / 100.0)
    }
}
