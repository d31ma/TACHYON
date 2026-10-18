[Test(typeof(ResultTestSetup))]
public sealed class ResultServiceTest
{
    [Fake]
    public ResultClient _resultClient = null;

    void testListsAndArraysOfRecordsKeepTheirOrder()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(ResultFakeSetup), "lists", signal);
        _resultClient = new ResultClient(context);
        var service = construct<ResultService>();
        var items = service.items().GetAwaiter().GetResult();
        Assert.equals(2, items.Count);
        Assert.equals("fake first", items[0].Name);
        Assert.equals("fake second", items[1].Name);
        var array = service.array().GetAwaiter().GetResult();
        Assert.equals(1, array.Length);
        Assert.equals("fake array", array[0].Name);
        var numbers = service.numbers().GetAwaiter().GetResult();
        Assert.equals(3, numbers.Count);
        Assert.equals(3, numbers[0]);
        Assert.equals(2, numbers[2]);
    }

    void testPlainValuesAreRead()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(ResultFakeSetup), "values", signal);
        _resultClient = new ResultClient(context);
        var service = construct<ResultService>();
        Assert.equals(7, service.count().GetAwaiter().GetResult());
        Assert.equals("fake label", service.label().GetAwaiter().GetResult());
        Assert.isTrue(service.ready().GetAwaiter().GetResult());
        Assert.equals(0.5, service.ratio().GetAwaiter().GetResult());
    }

    void testNestedRecordAndValueTaskAreRead()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(ResultFakeSetup), "nested", signal);
        _resultClient = new ResultClient(context);
        var service = construct<ResultService>();
        var shelf = service.shelf().GetAwaiter().GetResult();
        Assert.equals("fake shelf", shelf.Label);
        Assert.equals("fake note", shelf.Detail.Note);
        Assert.equals(2, shelf.Detail.Count);
        Assert.equals("fake shelf: fake note", service.Summary);
        var first = service.first().GetAwaiter().GetResult();
        Assert.equals("fake value task", first.Name);
    }

    void testWrongShapesAreRefused()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(ResultFakeSetup), "wrong", signal);
        _resultClient = new ResultClient(context);
        var service = construct<ResultService>();
        Assert.isTrue(_refused(() => service.wrongItems()));
        Assert.isTrue(_refused(() => service.wrongCount()));
        Assert.isTrue(_refused(() => service.wrongShelf()));
        Assert.equals("", service.Summary);
    }

    private static bool _refused(System.Func<System.Threading.Tasks.Task> pending)
    {
        try { pending().GetAwaiter().GetResult(); return false; }
        catch (BehaviorError error) { return error.Code == "HTTP_RESPONSE"; }
    }
}
