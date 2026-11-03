class Geolocation {
    fun currentPosition() {
        TacWebHost.call("geolocation", "currentPosition")
    }
}
