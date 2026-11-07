open System.Text.Json.Nodes

type LedgerReport = { Settled: JsonNode; Commands: JsonNode }
