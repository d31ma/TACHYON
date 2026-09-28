@Service
class ReportService {
    Object summarize(Object period) {
        return java.util.Map.of("period", period, "total", 42, "language", "java")
    }
}
