final class Connectivity {
    boolean online() {
        return TacWebHost.flag("connectivity", "online");
    }
}
