using System;
using System.Collections.Generic;
using System.Net.Http;
using Models;

[IdentityProvider]
class AppIdentityProvider {
    private static readonly Dictionary<string, Identity> Sessions = new() {
        ["ada-token"] = new Identity("ada", new List<string> { "Admin" }),
        ["bob-token"] = new Identity("bob", new List<string>()),
    };

    public Identity? Identify(HttpRequestMessage request) {
        return Find(Convert.ToString(request.Headers.Authorization));
    }

    private static Identity? Find(string? token) {
        return Sessions.GetValueOrDefault(token ?? "");
    }
}
