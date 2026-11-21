using System;
using System.Collections.Generic;
using System.Net.Http;
using Models;

[IdentityProvider]
class AppIdentityProvider {
    private const string BrokenToken = "broken-token";

    private static readonly Dictionary<string, Identity> Sessions = new() {
        ["ada-token"] = new Identity("ada", new List<string> { "Admin" }),
        ["bob-token"] = new Identity("bob", new List<string>()),
    };

    public Identity? Identify(HttpRequestMessage request) {
        var token = Convert.ToString(request.Headers.Authorization);
        RefuseBroken(token);
        return Find(token);
    }

    private static void RefuseBroken(string? token) {
        if (token == BrokenToken) {
            throw new InvalidOperationException("the session store is down");
        }
    }

    private static Identity? Find(string? token) {
        return Sessions.GetValueOrDefault(token ?? "");
    }
}
