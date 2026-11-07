[Service]
[NeedsAll]
public class DocumentationService
{
    private const string Title = "title";
    private const string Summary = "summary";
    private const string TopicsKey = "topics";
    private const string FeaturesKey = "features";
    private const string GroupsKey = "groups";
    private const string Anchor = "anchor";

    public string Source = "website/content/docs/page.json";
    public required object Documents;
    public required object Features;

    public object Contents = new List<object>
    {
        new Dictionary<string, object> { ["heading"] = "Guides", [Anchor] = "#guides" },
        new Dictionary<string, object> { ["heading"] = "Features", [Anchor] = "#features" },
    };

    private object Topics()
    {
        var docs = MapValue(Documents);
        return TopicsIn(ListValue(Value(docs, "order")), MapValue(Value(docs, TopicsKey)));
    }

    public object Guides()
    {
        var topics = TopicsBySlug((List<object>)Topics());
        var groups = GuideGroups(ListValue(Value(MapValue(Documents), GroupsKey)), topics);
        return GroupsOrEveryTopic(WithTopics(groups));
    }

    public object List()
    {
        var catalogue = MapValue(Features);
        var groups = FeatureGroups(ListValue(Value(catalogue, GroupsKey)), ListValue(Value(catalogue, FeaturesKey)));
        return WithFeatures(groups);
    }

    public int Count()
    {
        return CountFeatures((List<object>)List());
    }

    private static List<object> TopicsIn(object[] order, IDictionary<string, object> topics)
    {
        return order.OfType<string>().Where(topics.ContainsKey).Select(slug => TopicEntry(slug, topics)).ToList<object>();
    }

    private static object TopicEntry(string slug, IDictionary<string, object> topics)
    {
        var topic = MapValue(topics[slug]);
        return new Dictionary<string, object>
        {
            ["slug"] = slug,
            [Title] = TextValue(topic, Title),
            [Summary] = TextValue(topic, Summary),
            ["path"] = "/docs/" + slug,
        };
    }

    private static Dictionary<string, object> TopicsBySlug(List<object> topics)
    {
        return topics.Cast<IDictionary<string, object>>().ToDictionary(topic => (string)topic["slug"], topic => (object)topic);
    }

    private static List<Dictionary<string, object>> GuideGroups(object[] groups, Dictionary<string, object> topics)
    {
        return groups.Select(group => GuideGroup(group, topics)).ToList();
    }

    private static Dictionary<string, object> GuideGroup(object value, Dictionary<string, object> topics)
    {
        var group = MapValue(value);
        return new Dictionary<string, object>
        {
            [Title] = TextValue(group, Title),
            [Summary] = TextValue(group, Summary),
            [TopicsKey] = TopicsNamed(ListValue(Value(group, TopicsKey)), topics),
        };
    }

    private static List<object> TopicsNamed(object[] slugs, Dictionary<string, object> topics)
    {
        return slugs.OfType<string>().Where(topics.ContainsKey).Select(slug => topics[slug]).ToList();
    }

    private static List<object> WithTopics(List<Dictionary<string, object>> groups)
    {
        groups.RemoveAll(HasNoTopics);
        return groups.ToList<object>();
    }

    private static bool HasNoTopics(Dictionary<string, object> group)
    {
        return ((List<object>)group[TopicsKey]).Count == 0;
    }

    private List<object> GroupsOrEveryTopic(List<object> groups)
    {
        if (groups.Count == 0)
            groups.Add(new Dictionary<string, object> { [Title] = "", [Summary] = "", [TopicsKey] = (List<object>)Topics() });
        return groups;
    }

    private static List<Dictionary<string, object>> FeatureGroups(object[] names, object[] features)
    {
        return names.Select(name => FeatureGroup(name, features)).ToList();
    }

    private static Dictionary<string, object> FeatureGroup(object name, object[] features)
    {
        return new Dictionary<string, object>
        {
            ["name"] = name,
            ["slug"] = SlugOf((string)name),
            [FeaturesKey] = FeaturesInGroup(features, name),
        };
    }

    private static List<object> FeaturesInGroup(object[] features, object groupName)
    {
        return features.OfType<IDictionary<string, object>>().Where(feature => Equals(feature["group"], groupName)).ToList<object>();
    }

    private static List<object> WithFeatures(List<Dictionary<string, object>> groups)
    {
        groups.RemoveAll(HasNoFeatures);
        return groups.ToList<object>();
    }

    private static bool HasNoFeatures(Dictionary<string, object> group)
    {
        return ((List<object>)group[FeaturesKey]).Count == 0;
    }

    private static int CountFeatures(List<object> groups)
    {
        return groups.Cast<IDictionary<string, object>>().Sum(FeatureCount);
    }

    private static int FeatureCount(IDictionary<string, object> group)
    {
        return ((List<object>)group[FeaturesKey]).Count;
    }

    private static IDictionary<string, object> MapValue(object value)
    {
        return value as IDictionary<string, object> ?? new Dictionary<string, object>();
    }

    private static object[] ListValue(object value)
    {
        return value as object[] ?? System.Array.Empty<object>();
    }

    private static object Value(IDictionary<string, object> map, string key)
    {
        return map.TryGetValue(key, out var value) ? value : null;
    }

    private static object TextValue(IDictionary<string, object> map, string key)
    {
        return map.TryGetValue(key, out var value) ? value : "";
    }

    private static string SlugOf(string value)
    {
        var dashed = System.Text.RegularExpressions.Regex.Replace(value.ToLowerInvariant(), "[^a-z0-9]+", "-");
        return System.Text.RegularExpressions.Regex.Replace(dashed, "^-|-$", "");
    }
}
