import models.FeatureCatalogue
import models.FeatureEntry
import models.SnippetFile

@Service
@NeedsAll
class FeatureService {
    var routePath: String = ""
    var catalogue: FeatureCatalogue = FeatureCatalogue(emptyList(), emptyList())

    @Needs
    private val documentTitle: DocumentTitle

    val id: String get() = lastSegment(path())

    val source: String get() = sourceOf(foundEntry(), id)

    fun path(): String = normalizedPath(routePath)

    fun entry(): FeatureEntry = entryOrFallback(foundEntry())

    fun groupAnchor(): String = "/docs/features#${groupSlug(entry().group)}"

    fun files(): List<SnippetFile> = entry().files

    @OnStart
    fun initialize() {
        documentTitle.set("${entry().title} — Tachyon docs")
    }

    private fun normalizedPath(value: String): String = value.replace(Regex("(.)/+\$"), "$1")

    private fun lastSegment(value: String): String = value.split('/').lastOrNull { it.isNotEmpty() } ?: ""

    private fun foundEntry(): FeatureEntry? = catalogue.features.firstOrNull { it.id == id }

    private fun sourceOf(found: FeatureEntry?, slug: String): String {
        if (found == null) return "website/content/features/page.json"
        return "website/content/features/$slug/feature.json"
    }

    private fun entryOrFallback(found: FeatureEntry?): FeatureEntry =
        found?.copy(path = "/docs/features/${found.id}") ?: NOT_FOUND

    private fun groupSlug(value: String): String =
        value.lowercase().replace(Regex("[^a-z0-9]+"), "-").replace(Regex("^-|-$"), "")

    companion object {
        private val NOT_FOUND: FeatureEntry = FeatureEntry(
            "", "Feature not found",
            "That feature does not exist. Open the list of features instead.",
            "", emptyList()
        )
    }
}
