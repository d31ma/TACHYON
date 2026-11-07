  private static final String[] BROWSER_API_GRANTS = __BROWSER_API_GRANTS__;
  private static final String[] LOCATION_PERMISSIONS = {
      "android.permission.ACCESS_FINE_LOCATION", "android.permission.ACCESS_COARSE_LOCATION"};
  private static final String NOTIFICATION_PERMISSION = "android.permission.POST_NOTIFICATIONS";
  private static final String NOTIFICATION_CHANNEL = "tachyon";
  private static final int NOTIFICATION_PERMISSION_REQUEST = 7301;
  private static final int LOCATION_PERMISSION_REQUEST = 7302;

  private final java.util.HashMap<Integer, java.util.ArrayList<Runnable>> permissionWaiters =
      new java.util.HashMap<>();
  private androidx.webkit.ScriptHandler notificationPermissionScript;
  private int shownNotifications = (int) ((System.currentTimeMillis() / 1000) & 0x3fffffff);

  private interface BrowserApiAnswer {
    void send(String result);
  }

  private final class BrowserApiChromeClient extends android.webkit.WebChromeClient {
    @Override
    public void onGeolocationPermissionsShowPrompt(
        String origin, android.webkit.GeolocationPermissions.Callback callback) {
      if (origin == null || !isAssetUrl(Uri.parse(origin)) || !browserApiGranted("geolocation")) {
        callback.invoke(origin, false, false);
        return;
      }
      if (holdsLocation()) {
        callback.invoke(origin, true, false);
        return;
      }
      askAndroidPermission(LOCATION_PERMISSION_REQUEST, LOCATION_PERMISSIONS, "geolocation",
          () -> callback.invoke(origin, holdsLocation(), false));
    }
  }

  private void installBrowserApis() {
    view.getSettings().setGeolocationEnabled(browserApiGranted("geolocation"));
    view.setWebChromeClient(new BrowserApiChromeClient());
    publishNotificationPermission();
  }

  private static boolean browserApiGranted(String api) {
    for (String granted : BROWSER_API_GRANTS) {
      if (granted.equals(api)) return true;
    }
    return false;
  }

  private static long browserApiPatience(String capability) {
    return "browserApi.notifications.requestPermission".equals(capability) ? 120000 : 10000;
  }

  private void answerBrowserApi(WebView web, BridgeRequest request, JavaScriptReplyProxy replyProxy) {
    final long generation = documentGeneration;
    final String ownerRoute = route;
    final long deadline =
        android.os.SystemClock.uptimeMillis() + browserApiPatience(request.capability);
    browserApiCall(request.capability, request.payload, result -> {
      String bounded = withinByteLimit(result, 65536) ? result
          : browserApiError("Native result exceeds the size limit.");
      String reply = "{\"id\":" + org.json.JSONObject.quote(request.id)
          + ",\"result\":" + org.json.JSONObject.quote(bounded) + "}";
      web.post(() -> {
        if (currentDocument(web, generation, ownerRoute)
            && android.os.SystemClock.uptimeMillis() <= deadline) replyProxy.postMessage(reply);
      });
    });
  }

  private void browserApiCall(String capability, String payload, BrowserApiAnswer answer) {
    String[] parts = capability.split("\\.");
    if (parts.length != 3 || !browserApiGranted(parts[1])) {
      answer.send(browserApiError("The Android app does not grant " + safeName(capability)
          + ". Add the API to tac.browserApis in client/manifest.json."));
      return;
    }
    try {
      org.json.JSONObject request = new org.json.JSONObject(payload);
      switch (capability) {
        case "browserApi.clipboard.writeText": answer.send(writeClipboard(request)); return;
        case "browserApi.clipboard.readText": answer.send(readClipboard()); return;
        case "browserApi.share.open": answer.send(openShareSheet(request)); return;
        case "browserApi.notifications.requestPermission": requestNotificationPermission(answer); return;
        case "browserApi.notifications.show": answer.send(showNotification(request)); return;
        default: answer.send(browserApiError("The Android app does not answer " + safeName(capability) + "."));
      }
    } catch (org.json.JSONException | RuntimeException error) {
      answer.send(browserApiError("The Android app could not run " + safeName(capability) + "."));
    }
  }

  private static String browserApiValue(String json) {
    return "{\"value\":" + json + "}";
  }

  private static String browserApiError(String message) {
    return "{\"error\":" + org.json.JSONObject.quote(message) + "}";
  }

  private static String optionalText(org.json.JSONObject request, String name)
      throws org.json.JSONException {
    if (!request.has(name) || request.isNull(name)) return "";
    Object value = request.get(name);
    if (!(value instanceof String)) throw new org.json.JSONException(name + " must be text.");
    return (String) value;
  }

  private android.content.ClipboardManager clipboardManager() {
    return (android.content.ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
  }

  private String writeClipboard(org.json.JSONObject request) throws org.json.JSONException {
    android.content.ClipboardManager clipboard = clipboardManager();
    if (clipboard == null) return browserApiError("The Android clipboard is unavailable.");
    clipboard.setPrimaryClip(android.content.ClipData.newPlainText(
        getString(R.string.app_name), optionalText(request, "text")));
    return browserApiValue("null");
  }

  private String readClipboard() {
    android.content.ClipboardManager clipboard = clipboardManager();
    if (clipboard == null) return browserApiError("The Android clipboard is unavailable.");
    android.content.ClipData clip = clipboard.getPrimaryClip();
    CharSequence text = clip == null || clip.getItemCount() == 0 ? "" : clip.getItemAt(0).coerceToText(this);
    return browserApiValue(org.json.JSONObject.quote(String.valueOf(text)));
  }

  private String openShareSheet(org.json.JSONObject request) throws org.json.JSONException {
    String title = optionalText(request, "title");
    String text = optionalText(request, "text");
    String url = optionalText(request, "url");
    String shared = text.isEmpty() ? url : url.isEmpty() ? text : text + "\n" + url;
    if (shared.isEmpty()) shared = title;
    if (shared.isEmpty()) return browserApiError("Share needs a title, text or a url.");
    Intent send = new Intent(Intent.ACTION_SEND).setType("text/plain")
        .putExtra(Intent.EXTRA_TEXT, shared);
    if (!title.isEmpty()) send.putExtra(Intent.EXTRA_SUBJECT, title).putExtra(Intent.EXTRA_TITLE, title);
    try {
      startActivity(Intent.createChooser(send, title.isEmpty() ? null : title));
    } catch (android.content.ActivityNotFoundException error) {
      return browserApiError("No Android app can share this.");
    }
    return browserApiValue("null");
  }

  private android.content.SharedPreferences browserApiState() {
    return getSharedPreferences("tachyon_browser_apis", MODE_PRIVATE);
  }

  private String notificationPermission() {
    if (androidx.core.app.NotificationManagerCompat.from(this).areNotificationsEnabled()) return "granted";
    boolean undecided = android.os.Build.VERSION.SDK_INT >= 33
        && checkSelfPermission(NOTIFICATION_PERMISSION) != android.content.pm.PackageManager.PERMISSION_GRANTED
        && !browserApiState().getBoolean("notifications.asked", false);
    return undecided ? "default" : "denied";
  }

  private void requestNotificationPermission(BrowserApiAnswer answer) {
    Runnable reply = () -> answer.send(browserApiValue(org.json.JSONObject.quote(notificationPermission())));
    if (android.os.Build.VERSION.SDK_INT < 33 || "granted".equals(notificationPermission())) {
      reply.run();
      return;
    }
    askAndroidPermission(NOTIFICATION_PERMISSION_REQUEST, new String[] {NOTIFICATION_PERMISSION},
        "notifications", reply);
  }

  private String showNotification(org.json.JSONObject request) throws org.json.JSONException {
    String refused = "Notifications are not allowed. Call requestPermission first.";
    if (!"granted".equals(notificationPermission())) return browserApiError(refused);
    String reason = permissionReason("notifications");
    android.app.NotificationManager system = getSystemService(android.app.NotificationManager.class);
    system.createNotificationChannel(new android.app.NotificationChannel(NOTIFICATION_CHANNEL,
        reason == null ? getString(R.string.app_name) : reason,
        android.app.NotificationManager.IMPORTANCE_DEFAULT));
    Intent open = new Intent(this, MainActivity.class)
        .addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP | Intent.FLAG_ACTIVITY_CLEAR_TOP);
    android.app.PendingIntent tap = android.app.PendingIntent.getActivity(
        this, 0, open, android.app.PendingIntent.FLAG_IMMUTABLE);
    int icon = getApplicationInfo().icon != 0 ? getApplicationInfo().icon : android.R.drawable.ic_dialog_info;
    String body = optionalText(request, "body");
    android.app.Notification shown =
        new androidx.core.app.NotificationCompat.Builder(this, NOTIFICATION_CHANNEL)
            .setSmallIcon(icon)
            .setContentTitle(optionalText(request, "title"))
            .setContentText(body)
            .setStyle(new androidx.core.app.NotificationCompat.BigTextStyle().bigText(body))
            .setContentIntent(tap)
            .setAutoCancel(true)
            .build();
    try {
      androidx.core.app.NotificationManagerCompat.from(this).notify(++shownNotifications, shown);
    } catch (SecurityException error) {
      return browserApiError(refused);
    }
    return browserApiValue("null");
  }

  private void publishNotificationPermission() {
    if (view == null || !browserApiGranted("notifications")
        || !WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) return;
    if (notificationPermissionScript != null) notificationPermissionScript.remove();
    notificationPermissionScript = WebViewCompat.addDocumentStartJavaScript(view,
        "globalThis.__tachyonNotificationPermission = "
            + org.json.JSONObject.quote(notificationPermission()) + ";",
        java.util.Collections.singleton("https://appassets.tachyon.local"));
  }

  private boolean holdsLocation() {
    for (String permission : LOCATION_PERMISSIONS) {
      if (checkSelfPermission(permission) == android.content.pm.PackageManager.PERMISSION_GRANTED) return true;
    }
    return false;
  }

  private String permissionReason(String api) {
    int id = getResources().getIdentifier("tachyon_permission_" + api, "string", getPackageName());
    return id == 0 ? null : getString(id);
  }

  private void askAndroidPermission(int requestCode, String[] permissions, String api, Runnable after) {
    java.util.ArrayList<Runnable> waiting =
        permissionWaiters.computeIfAbsent(requestCode, code -> new java.util.ArrayList<>());
    waiting.add(after);
    if (waiting.size() > 1) return;
    String reason = permissionReason(api);
    if (reason == null || !shouldShowRequestPermissionRationale(permissions[0])) {
      requestPermissions(permissions, requestCode);
      return;
    }
    new android.app.AlertDialog.Builder(this)
        .setMessage(reason)
        .setPositiveButton(android.R.string.ok, (dialog, button) -> requestPermissions(permissions, requestCode))
        .setNegativeButton(android.R.string.cancel, (dialog, button) -> dialog.cancel())
        .setOnCancelListener(dialog -> finishPermissionWait(requestCode))
        .show();
  }

  private void finishPermissionWait(int requestCode) {
    java.util.ArrayList<Runnable> waiting = permissionWaiters.remove(requestCode);
    if (waiting == null) return;
    for (Runnable after : waiting) after.run();
  }

  @Override
  public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] results) {
    super.onRequestPermissionsResult(requestCode, permissions, results);
    if (requestCode == NOTIFICATION_PERMISSION_REQUEST) {
      browserApiState().edit().putBoolean("notifications.asked", true).apply();
      publishNotificationPermission();
    }
    finishPermissionWait(requestCode);
  }
