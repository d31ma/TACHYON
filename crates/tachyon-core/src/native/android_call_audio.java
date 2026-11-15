  private static final String CALL_NOTICE = "__CALL_NOTICE__";
  private static final boolean CALL_BACKGROUND = __CALL_BACKGROUND__;
  private static final String CALL_CHANNEL = "tachyon-call";
  private static final int CALL_NOTIFICATION = 7310;
  private static final String CALL_TITLE = "title";
  private static final String CALL_CAMERA = "camera";
  private static final String EARPIECE = "earpiece";
  private static final String SPEAKER = "speaker";

  public static class CallService extends android.app.Service {
    @Override
    public android.os.IBinder onBind(Intent intent) {
      return null;
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
      String title = intent == null ? "" : intent.getStringExtra(CALL_TITLE);
      boolean camera = intent != null && intent.getBooleanExtra(CALL_CAMERA, false);
      android.app.Notification shown = callNotification(this, title == null || title.isEmpty() ? CALL_NOTICE : title);
      if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.Q) startForeground(CALL_NOTIFICATION, shown, callTypes(camera));
      else startForeground(CALL_NOTIFICATION, shown);
      return START_NOT_STICKY;
    }
  }

  private static android.app.Notification callNotification(android.content.Context context, String title) {
    android.app.NotificationManager system = context.getSystemService(android.app.NotificationManager.class);
    system.createNotificationChannel(new android.app.NotificationChannel(CALL_CHANNEL, title,
        android.app.NotificationManager.IMPORTANCE_LOW));
    Intent open = new Intent(context, MainActivity.class)
        .addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP | Intent.FLAG_ACTIVITY_CLEAR_TOP);
    android.app.PendingIntent tap = android.app.PendingIntent.getActivity(
        context, 0, open, android.app.PendingIntent.FLAG_IMMUTABLE);
    int icon = context.getApplicationInfo().icon != 0 ? context.getApplicationInfo().icon : android.R.drawable.ic_btn_speak_now;
    return new androidx.core.app.NotificationCompat.Builder(context, CALL_CHANNEL)
        .setSmallIcon(icon)
        .setContentTitle(title)
        .setOngoing(true)
        .setCategory(androidx.core.app.NotificationCompat.CATEGORY_CALL)
        .setContentIntent(tap)
        .build();
  }

  @androidx.annotation.RequiresApi(android.os.Build.VERSION_CODES.Q)
  private static int callTypes(boolean camera) {
    int microphone = android.content.pm.ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE;
    return camera ? microphone | android.content.pm.ServiceInfo.FOREGROUND_SERVICE_TYPE_CAMERA : microphone;
  }

  private String media(String capability, String payload) {
    if (!browserApiGranted("microphone") && !browserApiGranted("camera")) {
      return browserApiError("The media browser API is not granted."
          + " Add \"microphone\" or \"camera\" to tac.browserApis in client/manifest.json.");
    }
    try {
      org.json.JSONObject request = new org.json.JSONObject(payload);
      switch (capability) {
        case "browserApi.media.callActive": return callActive(request.optBoolean("on"), request.optString(CALL_TITLE));
        case "browserApi.media.speakers": return browserApiValue(speakers().toString());
        case "browserApi.media.chooseSpeaker": return chooseSpeaker(request.optString("device"));
        default: return browserApiError("The Android app does not answer " + safeName(capability) + ".");
      }
    } catch (org.json.JSONException | RuntimeException error) {
      return browserApiError("The Android app could not run " + safeName(capability) + ".");
    }
  }

  private String callActive(boolean on, String title) throws org.json.JSONException {
    android.media.AudioManager audio = getSystemService(android.media.AudioManager.class);
    audio.setMode(on ? android.media.AudioManager.MODE_IN_COMMUNICATION : android.media.AudioManager.MODE_NORMAL);
    String failed = on ? startCallService(title) : stopCallService();
    if (failed != null) return browserApiError(failed);
    return browserApiValue(new org.json.JSONObject().put("on", on).put("background", CALL_BACKGROUND).toString());
  }

  private String startCallService(String title) {
    if (!CALL_BACKGROUND) return null;
    boolean camera = browserApiGranted("camera")
        && checkSelfPermission(android.Manifest.permission.CAMERA) == android.content.pm.PackageManager.PERMISSION_GRANTED;
    Intent service = new Intent(this, CallService.class).putExtra(CALL_TITLE, title).putExtra(CALL_CAMERA, camera);
    try {
      startForegroundService(service);
      return null;
    } catch (SecurityException | IllegalStateException error) {
      return "Allow the microphone before a call runs in the background.";
    }
  }

  private String stopCallService() {
    stopService(new Intent(this, CallService.class));
    return null;
  }

  private org.json.JSONArray speakers() throws org.json.JSONException {
    android.media.AudioManager audio = getSystemService(android.media.AudioManager.class);
    if (android.os.Build.VERSION.SDK_INT < android.os.Build.VERSION_CODES.S) return builtInSpeakers(audio.isSpeakerphoneOn());
    return communicationSpeakers(audio);
  }

  @androidx.annotation.RequiresApi(android.os.Build.VERSION_CODES.S)
  private static org.json.JSONArray communicationSpeakers(android.media.AudioManager audio) throws org.json.JSONException {
    org.json.JSONArray list = new org.json.JSONArray();
    android.media.AudioDeviceInfo chosen = audio.getCommunicationDevice();
    for (android.media.AudioDeviceInfo device : audio.getAvailableCommunicationDevices()) {
      list.put(speaker(device, chosen != null && chosen.getId() == device.getId()));
    }
    return list;
  }

  private static org.json.JSONArray builtInSpeakers(boolean speakerOn) throws org.json.JSONException {
    return new org.json.JSONArray()
        .put(new org.json.JSONObject().put("id", EARPIECE).put("label", "Phone").put("chosen", !speakerOn))
        .put(new org.json.JSONObject().put("id", SPEAKER).put("label", "Speaker").put("chosen", speakerOn));
  }

  private static org.json.JSONObject speaker(android.media.AudioDeviceInfo device, boolean chosen) throws org.json.JSONException {
    return new org.json.JSONObject()
        .put("id", String.valueOf(device.getId()))
        .put("label", speakerLabel(device))
        .put("chosen", chosen);
  }

  private static String speakerLabel(android.media.AudioDeviceInfo device) {
    if (device.getType() == android.media.AudioDeviceInfo.TYPE_BUILTIN_EARPIECE) return "Phone";
    if (device.getType() == android.media.AudioDeviceInfo.TYPE_BUILTIN_SPEAKER) return "Speaker";
    return String.valueOf(device.getProductName());
  }

  private String chooseSpeaker(String device) throws org.json.JSONException {
    android.media.AudioManager audio = getSystemService(android.media.AudioManager.class);
    boolean chosen = android.os.Build.VERSION.SDK_INT < android.os.Build.VERSION_CODES.S
        ? chooseBuiltIn(audio, device)
        : chooseCommunicationDevice(audio, device);
    if (!chosen) return browserApiError("There is no such speaker (unknown).");
    return browserApiValue(new org.json.JSONObject().put("device", device).toString());
  }

  private static boolean chooseBuiltIn(android.media.AudioManager audio, String device) {
    if (!EARPIECE.equals(device) && !SPEAKER.equals(device)) return false;
    audio.setSpeakerphoneOn(SPEAKER.equals(device));
    return true;
  }

  @androidx.annotation.RequiresApi(android.os.Build.VERSION_CODES.S)
  private static boolean chooseCommunicationDevice(android.media.AudioManager audio, String device) {
    for (android.media.AudioDeviceInfo found : audio.getAvailableCommunicationDevices()) {
      if (String.valueOf(found.getId()).equals(device)) return audio.setCommunicationDevice(found);
    }
    return false;
  }
