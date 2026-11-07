[Test(typeof(SiteTestSetup), {"partition":"tests"})]
sealed class DocumentationServiceTest {
    static Dictionary<string, object> Feature(string id, string group) =>
        new() { ["id"] = id, ["title"] = id.ToUpperInvariant(), ["summary"] = "About " + id, ["group"] = group };

    static Dictionary<string, object> Catalogue() => new()
    {
        ["groups"] = new object[] { "Tac views", "Native", "Tooling" },
        ["features"] = new object[] { Feature("loops", "Tac views"), Feature("hosts", "Native"), Feature("slots", "Tac views") },
    };

    static Dictionary<string, object> Docs() => new()
    {
        ["order"] = new object[] { "introduction", "routing" },
        ["topics"] = new Dictionary<string, object>
        {
            ["introduction"] = new Dictionary<string, object> { ["title"] = "Introduction", ["summary"] = "Start here." },
            ["routing"] = new Dictionary<string, object> { ["title"] = "Routing", ["summary"] = "Paths." },
        },
    };

    void testTopicsAreGuidesInReadingOrder() {
        var documentation = new DocumentationService { Documents = Docs(), Features = Catalogue() };
        var everyTopic = ((List<object>)documentation.Guides()).Cast<Dictionary<string, object>>().Single();
        var topics = ((List<object>)everyTopic["topics"]).Cast<Dictionary<string, object>>().ToList();
        Assert.equals("/docs/introduction,/docs/routing", string.Join(",", topics.Select(topic => topic["path"])), "guides, in order, under /docs/");
        Assert.equals("Start here.", topics[0]["summary"], "each with its summary");
    }

    void testGuidesFollowTheDocsGroups() {
        var docs = Docs();
        docs["groups"] = new object[]
        {
            new Dictionary<string, object> { ["title"] = "Get started", ["summary"] = "First.", ["topics"] = new object[] { "introduction", "missing" } },
            new Dictionary<string, object> { ["title"] = "Empty", ["topics"] = new object[] { } },
            new Dictionary<string, object> { ["title"] = "Pages", ["topics"] = new object[] { "routing" } },
        };
        var documentation = new DocumentationService { Documents = docs, Features = Catalogue() };
        var groups = ((List<object>)documentation.Guides()).Cast<Dictionary<string, object>>().ToList();
        Assert.equals("Get started,Pages", string.Join(",", groups.Select(group => group["title"])), "groups that hold a topic, in order");
        var first = ((List<object>)groups[0]["topics"]).Cast<Dictionary<string, object>>().ToList();
        Assert.equals("/docs/introduction", string.Join(",", first.Select(topic => topic["path"])), "an unknown topic is dropped");
        var ungrouped = ((List<object>)new DocumentationService { Documents = Docs(), Features = Catalogue() }.Guides()).Cast<Dictionary<string, object>>().ToList();
        Assert.equals(1, ungrouped.Count, "without groups, one list holds every topic");
        Assert.equals(2, ((List<object>)ungrouped[0]["topics"]).Count, "the one list holds every topic");
    }

    void testListAndCountFollowTheCatalogue() {
        var documentation = new DocumentationService { Documents = Docs(), Features = Catalogue() };
        var groups = ((List<object>)documentation.List()).Cast<Dictionary<string, object>>().ToList();
        Assert.equals(2, groups.Count, "groups that hold a feature");
        Assert.equals("tac-views", groups[0]["slug"], "the first group is named by its slug");
        Assert.equals(3, documentation.Count(), "count agrees with the groups");
        var anchors = ((List<object>)documentation.Contents).Cast<Dictionary<string, object>>().Select(entry => entry["anchor"]);
        Assert.equals("#guides,#features", string.Join(",", anchors), "guides, then features");
        Assert.equals("website/content/docs/page.json", documentation.Source);
    }
}
