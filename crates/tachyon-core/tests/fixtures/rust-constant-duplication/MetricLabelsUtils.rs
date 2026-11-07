pub struct MetricLabelsUtils;

impl MetricLabelsUtils {
    const BYTE: &'static str = "Unit byte";
    const KIBIBYTE: &'static str = "Unit kibibyte";
    const MEBIBYTE: &'static str = "Unit mebibyte";
    const GIBIBYTE: &'static str = "Unit gibibyte";
    const READ_RATE: &'static str = "Unit read_rate";
    const WRITE_RATE: &'static str = "Unit write_rate";
    const DOWNLOAD_RATE: &'static str = "Unit download_rate";
    const UPLOAD_RATE: &'static str = "Unit upload_rate";
    const CORE_SECONDS: &'static str = "Unit core_seconds";
    const RESIDENT_MEMORY: &'static str = "Unit resident_memory";
    const PEAK_MEMORY: &'static str = "Unit peak_memory";

    pub fn text() -> String {
        Self::BYTE.to_owned()
    }
}
