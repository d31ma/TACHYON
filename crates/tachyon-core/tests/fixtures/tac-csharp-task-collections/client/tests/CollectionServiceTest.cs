[Test(typeof(CollectionTestSetup))]
public sealed class CollectionServiceTest
{
    [Fake]
    public CollectionClient _collectionClient = null;

    void testTypedCollectionFakeKeepsOrderedItemsAndIndependentOwners()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CollectionFakeSetup), "ordered", signal);
        _collectionClient = new CollectionClient(context);
        var first = construct<CollectionService>();
        var second = construct<CollectionService>();
        var answer = first.ask().GetAwaiter().GetResult();
        Assert.equals(2, answer.Items.Count);
        Assert.equals("first", answer.Items[0].Message);
        Assert.equals("second", answer.Items[1].Message);
        Assert.equals("first", first.First);
        Assert.equals("", second.First);
        context.Dispose();
        Assert.isTrue(_cancelled(first));
        Assert.equals("first", first.First);
    }

    void testEmptyRequiredListAndAbsentOptionalListAreValid()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CollectionFakeSetup), "empty", signal);
        var answer = new CollectionClient(context).empty().GetAwaiter().GetResult();
        Assert.equals(0, answer.Items.Count);
        Assert.equals(null, answer.OptionalItems);
    }

    void testDeclaredNullableElementsPreserveNull()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CollectionFakeSetup), "nullable", signal);
        var answer = new CollectionClient(context).nullable().GetAwaiter().GetResult();
        Assert.equals(2, answer.OptionalItems.Count);
        Assert.equals(null, answer.OptionalItems[0]);
        Assert.equals("nullable item", answer.OptionalItems[1].Message);
    }

    void testPresentNullOptionalListIsValid()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CollectionFakeSetup), "null-list", signal);
        var answer = new CollectionClient(context).nullList().GetAwaiter().GetResult();
        Assert.equals(0, answer.Items.Count);
        Assert.equals(null, answer.OptionalItems);
    }

    void testNullableLeafScalarPreservesAbsentNullAndTextButRefusesWrongType()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CollectionFakeSetup), "nullable-leaf", signal);
        var client = new CollectionClient(context);
        var answer = client.nullableLeaf().GetAwaiter().GetResult();
        Assert.equals(3, answer.Items.Count);
        Assert.equals(null, answer.Items[0].Note);
        Assert.equals(null, answer.Items[1].Note);
        Assert.equals("present", answer.Items[2].Note);
        Assert.isTrue(_refused(client.wrongNullableLeaf));
    }

    void testMissingRequiredListIsRefused()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CollectionFakeSetup), "missing", signal);
        Assert.isTrue(_refused(new CollectionClient(context).missing));
    }

    void testObjectCannotReplaceList()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CollectionFakeSetup), "wrong-array", signal);
        Assert.isTrue(_refused(new CollectionClient(context).wrongArray));
    }

    void testMissingRequiredLeafMemberIsRefused()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CollectionFakeSetup), "missing-member", signal);
        Assert.isTrue(_refused(new CollectionClient(context).missingMember));
    }

    void testWrongLeafTypeIsRefused()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CollectionFakeSetup), "wrong-member", signal);
        Assert.isTrue(_refused(new CollectionClient(context).wrongMember));
    }

    void testClosedLeafExtraMemberIsRefused()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CollectionFakeSetup), "extra-member", signal);
        Assert.isTrue(_refused(new CollectionClient(context).extraMember));
    }

    void testNonnullableLeafCannotBeNull()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CollectionFakeSetup), "null-member", signal);
        Assert.isTrue(_refused(new CollectionClient(context).nullMember));
    }

    private static bool _refused(System.Func<System.Threading.Tasks.Task<Models.MessagePage>> pending)
    {
        try { pending().GetAwaiter().GetResult(); return false; }
        catch (BehaviorError error) { return error.Code == "HTTP_RESPONSE"; }
    }

    private static bool _cancelled(CollectionService service)
    {
        try { service.ask().GetAwaiter().GetResult(); return false; }
        catch (BehaviorError error) { return error.Code == "CANCELLED"; }
    }
}
