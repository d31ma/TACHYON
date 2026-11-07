using System.Text.Json.Nodes;

public record ProductReport(JsonNode Active, JsonNode Count, JsonNode Others, JsonNode First);
