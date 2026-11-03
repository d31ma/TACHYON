public static class EmittedBehaviorTests {
    public static void Main() {
        var unit = new GreetingServiceTest();
        var inactiveRefused = false;
        try { unit.construct<GreetingService>(); }
        catch (System.InvalidOperationException error) { inactiveRefused = error.Message.Contains("active test setup"); }
        if (!inactiveRefused) throw new System.Exception("Inactive Test construction was accepted.");
        var reports = new string[] { unit.POST(), new MissingGreetingServiceTest().POST(), new IncompatibleGreetingServiceTest().POST() };
        foreach (var report in reports) {
            var cases = BehaviorJSON.Parse(report).Get("cases").Items;
            if (cases.Length == 0) throw new System.Exception("No authored Test executed.");
            foreach (var result in cases) {
                if (!result.Get("passed").Boolean) throw new System.Exception(result.ToString());
            }
        }
        inactiveRefused = false;
        try { unit.construct<GreetingService>(); }
        catch (System.InvalidOperationException error) { inactiveRefused = error.Message.Contains("active test setup"); }
        if (!inactiveRefused) throw new System.Exception("Stopped Test construction was accepted.");
        System.Console.WriteLine("PASS C# HTTPClient typed Test construction");
    }
}
