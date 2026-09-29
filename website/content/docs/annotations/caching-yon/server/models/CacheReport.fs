open System.Text.Json.Nodes

type CacheReport =
    { First: JsonNode
      Repeat: JsonNode
      Written: JsonNode
      Refreshed: JsonNode }
