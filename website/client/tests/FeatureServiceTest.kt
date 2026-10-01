import models.FeatureCatalogue
import models.FeatureEntry
import models.SnippetFile

@Test(SiteTestSetup::class, {"partition":"tests"})
class FeatureServiceTest {
    @Fake
    var documentTitle: DocumentTitle? = null

    private val titleWrites: MutableList<String> = mutableListOf()
    private val routing: FeatureEntry = FeatureEntry(
        "routing", "Page and controller routing", "Each directory is one page path.",
        "Routing", listOf(SnippetFile("layout", "<site-layout />"))
    )

    fun testEntryIsTheFeatureTheUrlNames() {
        val feature = featureAt("/docs/features/routing", listOf(routing))
        Assert.equals("routing", feature.id)
        Assert.equals("/docs/features/routing", feature.path())
        Assert.equals("routing", feature.entry().id, "entry is the routing feature")
        Assert.isTrue(feature.files().isNotEmpty(), "the routing feature ships its files")
        Assert.isTrue(feature.groupAnchor().startsWith("/docs/features#"), feature.groupAnchor())
        Assert.equals("website/content/features/routing/feature.json", feature.source)
        Assert.equals("/docs/features/routing", feature.entry().path, "derived navigation path is preserved")
    }

    fun testAnUnknownFeatureIsNotFoundRatherThanBroken() {
        val feature = featureAt("/docs/features/no-such-feature", listOf(routing))
        Assert.equals("Feature not found", feature.entry().title, "an unknown id is the not-found entry")
        Assert.equals(emptyList<SnippetFile>(), feature.files(), "and has no files")
        Assert.equals("website/content/features/page.json", feature.source)
        Assert.equals("/docs/features#", feature.groupAnchor())
    }

    fun testInitializeTitlesTheDocumentAfterTheFeature() {
        titleWrites.clear()
        val feature = featureAt("/docs/features/routing", listOf(routing))
        feature.initialize()
        Assert.equals(listOf("${feature.entry().title} — Tachyon docs"), titleWrites)
    }

    fun testChangedInputsRecomputeGettersWithoutAnAmbientLocation() {
        val feature = featureAt("/docs/features/routing///", listOf(routing))
        Assert.equals("/docs/features/routing", feature.path())
        Assert.equals("routing", feature.id)
        feature.routePath = "/docs/features/no-such-feature/"
        Assert.equals("no-such-feature", feature.id)
        Assert.equals("website/content/features/page.json", feature.source)
        feature.routePath = "/"
        Assert.equals("/", feature.path())
        Assert.equals("", feature.id)
        feature.routePath = "/docs/features/%72outing/"
        Assert.equals("%72outing", feature.id, "path segments are not URL decoded")
        Assert.equals("Feature not found", feature.entry().title)
    }

    fun testOptionalMetadataAndAbsentFilesKeepTheirValueContract() {
        val sparse = FeatureEntry("sparse", "Sparse", "Summary", "  Views & Routing!  ",
            verify = "ty test", target = "web", notApplicable = listOf("js", "ts"), axis = "language")
        val feature = featureAt("/docs/features/sparse", listOf(sparse))
        Assert.equals(emptyList<SnippetFile>(), feature.files())
        Assert.equals("/docs/features#views-routing", feature.groupAnchor())
        Assert.equals("ty test", feature.entry().verify)
        Assert.equals("web", feature.entry().target)
        Assert.equals(listOf("js", "ts"), feature.entry().notApplicable)
        Assert.equals("language", feature.entry().axis)
        feature.catalogue = FeatureCatalogue(emptyList(), listOf(routing))
        Assert.equals("Feature not found", feature.entry().title, "changed catalogue is read on demand")
    }

    fun testConstructedServicesRetainTheirOwnTitleFake() {
        val firstWrites = mutableListOf<String>()
        documentTitle = object : DocumentTitle { override fun set(title: String) { firstWrites.add(title) } }
        val first = construct(FeatureService::class)
        first.routePath = "/docs/features/routing"
        first.catalogue = FeatureCatalogue(listOf("Routing"), listOf(routing))
        val secondWrites = mutableListOf<String>()
        documentTitle = object : DocumentTitle { override fun set(title: String) { secondWrites.add(title) } }
        val second = construct(FeatureService::class)
        second.routePath = "/docs/features/missing"
        first.initialize()
        second.initialize()
        Assert.equals(listOf("Page and controller routing — Tachyon docs"), firstWrites)
        Assert.equals(listOf("Feature not found — Tachyon docs"), secondWrites)
    }

    private fun featureAt(route: String, entries: List<FeatureEntry>): FeatureService {
        documentTitle = object : DocumentTitle { override fun set(title: String) { titleWrites.add(title) } }
        val feature = construct(FeatureService::class)
        feature.routePath = route
        feature.catalogue = FeatureCatalogue(listOf("Routing"), entries)
        return feature
    }
}
