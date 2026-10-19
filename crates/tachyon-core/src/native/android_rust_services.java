  private static final java.util.Set<String> RUST_SERVICES =
      new java.util.HashSet<>(java.util.Arrays.asList(__RUST_SERVICES__));
  private final java.util.concurrent.atomic.AtomicBoolean rustResumePosted =
      new java.util.concurrent.atomic.AtomicBoolean();

  private static String rustOrCompanion(String request) {
    String rust = rustCompanionInvoke(request);
    return rust != null ? rust : __OTHER_COMPANION_CALL__;
  }

  private static String rustCompanionInvoke(String request) {
    String operation;
    String service;
    try {
      org.json.JSONObject parsed = new org.json.JSONObject(request);
      operation = parsed.optString("op");
      service = parsed.optString("service");
    } catch (org.json.JSONException error) {
      return null;
    }
    if ("host-dispose".equals(operation)) {
      dev.tachyon.host.TachyonRust.invoke(request);
      return null;
    }
    boolean owned = "host-resume".equals(operation) || RUST_SERVICES.contains(service);
    return owned ? dev.tachyon.host.TachyonRust.invoke(request) : null;
  }

  private void installRustCompanion() {
    String folder = new File(getFilesDir(), "tachyon-rust").getPath();
    boolean loaded = dev.tachyon.host.TachyonRust.attach(new dev.tachyon.host.TachyonRust.Host() {
      @Override
      public void publish(String payload) { relayPublish(payload); }

      @Override
      public void dispatch() { postRustResume(); }

      @Override
      public String capability(String capability, String payload) {
        return nativeCapability(capability, payload);
      }
    }, folder);
    record(loaded ? "companion.loaded" : "companion.unavailable");
  }

  private void postRustResume() {
    if (!rustResumePosted.compareAndSet(false, true)) return;
    try {
      bridgeWorker.execute(() -> {
        rustResumePosted.set(false);
        companionInvoke("{\"op\":\"host-resume\"}");
      });
    } catch (java.util.concurrent.RejectedExecutionException ignored) {
      rustResumePosted.set(false);
    }
  }
