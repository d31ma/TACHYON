[Test(typeof(GreetingTestSetup))]
public sealed class MissingGreetingServiceTest {
    void testMissingFakeIsRefused() {
        var refused = false;
        try { construct<GreetingService>(); }
        catch (System.InvalidOperationException error) { refused = error.Message.Contains("matching typed Fake named greetingClient"); }
        if (!refused) throw new System.Exception("Missing Fake was not refused.");
    }
}

