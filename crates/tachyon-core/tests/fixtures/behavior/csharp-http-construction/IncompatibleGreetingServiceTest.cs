[Test(typeof(GreetingTestSetup))]
public sealed class IncompatibleGreetingServiceTest {
    [Fake]
    public string greetingClient = "wrong type";

    void testIncompatibleFakeIsRefused() {
        var refused = false;
        try { construct<GreetingService>(); }
        catch (System.InvalidOperationException error) { refused = error.Message.Contains("matching typed Fake named greetingClient"); }
        if (!refused) throw new System.Exception("Incompatible Fake was not refused.");
    }
}

