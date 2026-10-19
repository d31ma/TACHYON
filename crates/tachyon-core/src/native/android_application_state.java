  private String applicationState = "foreground";

  @Override
  protected void onStart() {
    super.onStart();
    sendApplicationState("foreground");
  }

  @Override
  protected void onStop() {
    super.onStop();
    sendApplicationState("background");
  }

  private void sendApplicationState(String state) {
    if (state.equals(applicationState)) return;
    applicationState = state;
    record("application." + state);
    postApplicationState();
  }

  private void postApplicationState() {
    WebView web = view;
    if (web == null) return;
    String script = "globalThis.__tachyonApplicationState?.('" + applicationState + "')";
    web.post(() -> web.evaluateJavascript(script, null));
  }
