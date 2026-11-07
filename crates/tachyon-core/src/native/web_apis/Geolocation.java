final class Geolocation {
    void currentPosition() {
        TacWebHost.call("geolocation", "currentPosition");
    }
}
