import java.util.Map;

@Setup
class PageProgramSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        String identity = java.util.UUID.randomUUID().toString();
        int[] count = {0};
        Behaviors.HttpAdapter program = (request, running) -> {
            String path = String.valueOf(request.get("path"));
            if ("POST".equals(request.get("method")) && path.endsWith("/increment")) count[0] += 1;
            return Map.of("identity", identity, "count", count[0]);
        };
        return new Behaviors.Configuration(Map.of(), Map.of("pageprogram", program), Map.of(), null);
    }
}
