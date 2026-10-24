  private boolean nativeWindowGranted(String control) {
    switch (control) {
__WINDOW_GRANTS__
      default: return false;
    }
  }

  private String nativeCapability(String capability, String payload) {
    if (capability == null || payload == null || !withinByteLimit(payload, 65536)) return "{\"status\":\"invalid\"}";
    if (capability.startsWith("filePicker.")) return filePickerCall(capability.substring(11), payload);
    if (!capability.startsWith("window.")) return "{\"status\":\"unsupported\"}";
    try {
      org.json.JSONObject request = new org.json.JSONObject(payload);
      if ("window.granted".equals(capability)) return nativeWindowGranted(request.optString("control")) ? "{\"granted\":true}" : "{\"granted\":false}";
      String control = capability.substring(7);
      if (!nativeWindowGranted(control)) return "{\"ok\":false,\"error\":\"Window control is not granted.\"}";
      if (android.os.Looper.myLooper() == android.os.Looper.getMainLooper()) return nativeWindowExecute(control, request);
      long generation = documentGeneration;
      java.util.concurrent.atomic.AtomicReference<String> result = new java.util.concurrent.atomic.AtomicReference<>("{\"ok\":false,\"error\":\"Native window is unavailable.\"}");
      java.util.concurrent.CountDownLatch ready = new java.util.concurrent.CountDownLatch(1);
      java.util.concurrent.atomic.AtomicBoolean pending = new java.util.concurrent.atomic.AtomicBoolean(true);
      runOnUiThread(() -> {
        try {
          if (pending.getAndSet(false) && generation == documentGeneration && !isFinishing() && !isDestroyed()) result.set(nativeWindowExecute(control, request));
        } finally { ready.countDown(); }
      });
      if (!ready.await(2, java.util.concurrent.TimeUnit.SECONDS)) pending.set(false);
      return result.get();
    } catch (org.json.JSONException | InterruptedException error) {
      if (error instanceof InterruptedException) Thread.currentThread().interrupt();
      return "{\"ok\":false,\"error\":\"Invalid native window request.\"}";
    }
  }

  private String nativeWindowExecute(String control, org.json.JSONObject request) {
    try {
      switch (control) {
        case "minimize": moveTaskToBack(true); break;
        case "maximize": getWindow().setLayout(android.view.ViewGroup.LayoutParams.MATCH_PARENT, android.view.ViewGroup.LayoutParams.MATCH_PARENT); break;
        case "fullscreen":
          android.view.View decor = getWindow().getDecorView();
          int mask = android.view.View.SYSTEM_UI_FLAG_FULLSCREEN | android.view.View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | android.view.View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY;
          decor.setSystemUiVisibility((decor.getSystemUiVisibility() & mask) == mask ? decor.getSystemUiVisibility() & ~mask : decor.getSystemUiVisibility() | mask);
          break;
        case "close": finish(); break;
        case "resize":
          Object width = request.get("width"), height = request.get("height");
          if (!(width instanceof Integer) || !(height instanceof Integer) || (Integer)width < 64 || (Integer)width > 16384 || (Integer)height < 64 || (Integer)height > 16384) return "{\"ok\":false,\"error\":\"Invalid window size.\"}";
          float scale = getResources().getDisplayMetrics().density;
          getWindow().setLayout(Math.round((Integer)width * scale), Math.round((Integer)height * scale));
          break;
        case "title":
          Object title = request.get("title");
          if (!(title instanceof String) || ((String)title).length() > 512 || ((String)title).indexOf('\0') >= 0) return "{\"ok\":false,\"error\":\"Invalid window title.\"}";
          setTitle((String)title);
          break;
        default: return "{\"ok\":false,\"error\":\"Native window control is unavailable.\"}";
      }
      return "{\"ok\":true}";
    } catch (org.json.JSONException | RuntimeException error) { return "{\"ok\":false,\"error\":\"Native window control failed.\"}"; }
  }
