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
        if (string.Join(",", topics.Select(topic => topic["path"])) != "/docs/introduction,/docs/routing") throw new System.Exception("guides, in order, under /docs/");
        if ((string)topics[0]["summary"] != "Start here.") throw new System.Exception("each with its summary");
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
        if (string.Join(",", groups.Select(group => group["title"])) != "Get started,Pages") throw new System.Exception("groups that hold a topic, in order");
        var first = ((List<object>)groups[0]["topics"]).Cast<Dictionary<string, object>>().ToList();
        if (first.Count != 1 || (string)first[0]["path"] != "/docs/introduction") throw new System.Exception("an unknown topic is dropped");
        var ungrouped = ((List<object>)new DocumentationService { Documents = Docs(), Features = Catalogue() }.Guides()).Cast<Dictionary<string, object>>().ToList();
        if (ungrouped.Count != 1 || ((List<object>)ungrouped[0]["topics"]).Count != 2) throw new System.Exception("without groups, one list holds every topic");
    }

    void testListAndCountFollowTheCatalogue() {
        var documentation = new DocumentationService { Documents = Docs(), Features = Catalogue() };
        var groups = ((List<object>)documentation.List()).Cast<Dictionary<string, object>>().ToList();
        if (groups.Count != 2 || (string)groups[0]["slug"] != "tac-views") throw new System.Exception("groups that hold a feature");
        if (documentation.Count() != 3) throw new System.Exception("count agrees with the groups");
        var anchors = ((List<object>)documentation.Contents).Cast<Dictionary<string, object>>().Select(entry => entry["anchor"]);
        if (string.Join(",", anchors) != "#guides,#features") throw new System.Exception("guides, then features");
        if (documentation.Source != "website/content/docs/page.json") throw new System.Exception(documentation.Source);
    }
}
