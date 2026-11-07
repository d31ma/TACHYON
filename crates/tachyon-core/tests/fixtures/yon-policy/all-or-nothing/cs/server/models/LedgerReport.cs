using System.Text.Json.Nodes;

public record LedgerReport(JsonNode Settled, JsonNode Commands);
