data class FeatureEntry(
    val id: String,
    val title: String,
    val summary: String,
    val group: String,
    val files: List<SnippetFile> = emptyList(),
    val verify: String? = null,
    val target: String? = null,
    val notApplicable: List<String>? = null,
    val axis: String? = null,
    val path: String? = null
)
