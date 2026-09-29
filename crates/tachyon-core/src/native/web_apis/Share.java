final class Share {
    void open(String title, String text, String url) {
        TacWebHost.call("share", "open", title, text, url);
    }
}
