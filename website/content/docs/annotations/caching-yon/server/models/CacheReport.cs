using System.Text.Json.Nodes;

public record CacheReport(JsonNode First, JsonNode Repeat, JsonNode Written, JsonNode Refreshed);
