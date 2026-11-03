public record Pulse(int Amount);

public record Relay(int Amount);

public record Cycle(int Amount);

public record Isolation(int Value);

public record Ordered(string Status, int Total);

public record Other(string Status, int Total);

public record Echo(int Amount);
