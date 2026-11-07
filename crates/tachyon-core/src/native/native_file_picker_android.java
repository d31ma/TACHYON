  private static final int FILE_PICKER_REQUEST = 31601;
  private static final int FILE_PICKER_BYTE_LIMIT = 65536;
  private static final int FILE_PICKER_HANDLE_LIMIT = 128;
  private static final int FILE_PICKER_OWNER_LIMIT = 8;
  private static final long FILE_PICKER_LIFETIME = 300000L;
  private static final long FILE_PICKER_DIALOG_MILLISECONDS = 30000L;
  private static final String FILE_PICKER_ROUTE = "/__tachyon/capability/";
  private static final boolean FILE_PICKER_GRANTED = __FILE_PICKER_GRANTED__;
  private static final java.util.regex.Pattern FILE_PICKER_OWNER =
      java.util.regex.Pattern.compile("[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}");

  private static final class PickedFile {
    final String owner;
    final Uri uri;
    final long expiry;

    PickedFile(String owner, Uri uri, long expiry) {
      this.owner = owner;
      this.uri = uri;
      this.expiry = expiry;
    }
  }

  private static final class FilePickerChoice {
    final String owner;
    final java.util.concurrent.CountDownLatch done = new java.util.concurrent.CountDownLatch(1);
    volatile Uri uri;

    FilePickerChoice(String owner) { this.owner = owner; }

    synchronized void finish(Uri chosen) {
      if (done.getCount() == 0) return;
      uri = chosen;
      done.countDown();
    }
  }

  private final java.util.Map<String, PickedFile> pickedFiles = new java.util.HashMap<>();
  private FilePickerChoice filePickerChoice;

  private static String filePickerStatus(String status) {
    return "{\"status\":\"" + status + "\"}";
  }

  private String filePickerCall(String operation, String payload) {
    org.json.JSONObject request;
    try { request = new org.json.JSONObject(payload); }
    catch (org.json.JSONException error) { return filePickerStatus("invalid"); }
    String owner = request.optString("owner", "");
    if (!FILE_PICKER_OWNER.matcher(owner).matches()) return filePickerStatus("invalid");
    synchronized (pickedFiles) {
      long now = android.os.SystemClock.elapsedRealtime();
      pickedFiles.values().removeIf(file -> file.expiry <= now);
      if ("dispose".equals(operation)) { disposeFilePicker(owner); return filePickerStatus("disposed"); }
      if (!FILE_PICKER_GRANTED) { disposeFilePicker(owner); return filePickerStatus("denied"); }
      if ("available".equals(operation)) return filePickerStatus("available");
      if ("release".equals(operation)) return releasePickedFile(owner, request.optString("handle", ""));
    }
    if ("open".equals(operation)) return openFilePicker(owner, request.optJSONArray("types"));
    if ("readText".equals(operation)) return readPickedFile(owner, request.optString("handle", ""));
    return filePickerStatus("invalid");
  }

  private String openFilePicker(String owner, org.json.JSONArray types) {
    if (android.os.Looper.myLooper() == android.os.Looper.getMainLooper()) return filePickerStatus("unavailable");
    if (types == null || types.length() > 2) return filePickerStatus("invalid");
    String[] mimeTypes = new String[types.length()];
    for (int index = 0; index < types.length(); index++) {
      String type = types.optString(index, "");
      if (!"text/plain".equals(type) && !"application/json".equals(type)) return filePickerStatus("invalid");
      mimeTypes[index] = type;
    }
    FilePickerChoice choice = new FilePickerChoice(owner);
    synchronized (pickedFiles) {
      int owned = 0;
      for (PickedFile file : pickedFiles.values()) if (file.owner.equals(owner)) owned++;
      if (filePickerChoice != null || pickedFiles.size() >= FILE_PICKER_HANDLE_LIMIT || owned >= FILE_PICKER_OWNER_LIMIT) {
        return filePickerStatus("unavailable");
      }
      filePickerChoice = choice;
    }
    Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE)
        .setType(mimeTypes.length == 1 ? mimeTypes[0] : "*/*")
        .putExtra(Intent.EXTRA_ALLOW_MULTIPLE, false);
    if (mimeTypes.length > 1) intent.putExtra(Intent.EXTRA_MIME_TYPES, mimeTypes);
    runOnUiThread(() -> {
      if (choice.done.getCount() == 0 || isFinishing() || isDestroyed()) { choice.finish(null); return; }
      try { startActivityForResult(intent, FILE_PICKER_REQUEST); }
      catch (android.content.ActivityNotFoundException error) { choice.finish(null); }
    });
    boolean answered = false;
    try { answered = choice.done.await(FILE_PICKER_DIALOG_MILLISECONDS, java.util.concurrent.TimeUnit.MILLISECONDS); }
    catch (InterruptedException error) { Thread.currentThread().interrupt(); }
    if (!answered) {
      choice.finish(null);
      runOnUiThread(() -> finishActivity(FILE_PICKER_REQUEST));
    }
    Uri chosen = choice.uri;
    String name = chosen == null ? null : pickedFileName(chosen);
    synchronized (pickedFiles) {
      if (filePickerChoice == choice) filePickerChoice = null;
      if (chosen == null) return filePickerStatus("cancelled");
      String handle = java.util.UUID.randomUUID().toString();
      try {
        String answer = new org.json.JSONObject().put("status", "selected").put("handle", handle).put("name", name).toString();
        pickedFiles.put(handle, new PickedFile(owner, chosen, android.os.SystemClock.elapsedRealtime() + FILE_PICKER_LIFETIME));
        return answer;
      } catch (org.json.JSONException error) {
        return filePickerStatus("unavailable");
      }
    }
  }

  private String pickedFileName(Uri uri) {
    String name = null;
    try (android.database.Cursor cursor = getContentResolver().query(uri,
        new String[] {android.provider.OpenableColumns.DISPLAY_NAME}, null, null, null)) {
      if (cursor != null && cursor.moveToFirst() && !cursor.isNull(0)) name = cursor.getString(0);
    } catch (RuntimeException ignored) {
    }
    String safe = name == null || name.isEmpty() ? "file" : name.replaceAll("[/\\\\\\u0000]", "_");
    return safe.length() > 255 ? safe.substring(0, 255) : safe;
  }

  private String readPickedFile(String owner, String handle) {
    PickedFile file;
    synchronized (pickedFiles) { file = pickedFiles.get(handle); }
    if (file == null || !file.owner.equals(owner)) return filePickerStatus("invalid");
    byte[] buffer = new byte[FILE_PICKER_BYTE_LIMIT + 1];
    int count = 0;
    try (InputStream stream = getContentResolver().openInputStream(file.uri)) {
      if (stream == null) return filePickerStatus("unavailable");
      for (int read = 0; read >= 0 && count < buffer.length; count += Math.max(read, 0)) {
        read = stream.read(buffer, count, buffer.length - count);
      }
    } catch (java.io.IOException | RuntimeException error) {
      return filePickerStatus("unavailable");
    }
    if (count > FILE_PICKER_BYTE_LIMIT) return filePickerStatus("tooLarge");
    try {
      String text = StandardCharsets.UTF_8.newDecoder()
          .onMalformedInput(java.nio.charset.CodingErrorAction.REPORT)
          .onUnmappableCharacter(java.nio.charset.CodingErrorAction.REPORT)
          .decode(java.nio.ByteBuffer.wrap(buffer, 0, count)).toString();
      return new org.json.JSONObject().put("status", "read").put("text", text).toString();
    } catch (java.nio.charset.CharacterCodingException | org.json.JSONException error) {
      return filePickerStatus("invalid");
    }
  }

  private String releasePickedFile(String owner, String handle) {
    PickedFile file = pickedFiles.get(handle);
    if (file == null || !file.owner.equals(owner)) return filePickerStatus("invalid");
    pickedFiles.remove(handle);
    return filePickerStatus("released");
  }

  private void disposeFilePicker(String owner) {
    FilePickerChoice choice = filePickerChoice;
    if (choice != null && choice.owner.equals(owner)) closeFilePicker(choice);
    pickedFiles.values().removeIf(file -> file.owner.equals(owner));
  }

  private void disposeFilePickers() {
    synchronized (pickedFiles) {
      FilePickerChoice choice = filePickerChoice;
      if (choice != null) closeFilePicker(choice);
      pickedFiles.clear();
    }
  }

  private void closeFilePicker(FilePickerChoice choice) {
    choice.finish(null);
    runOnUiThread(() -> finishActivity(FILE_PICKER_REQUEST));
  }

  @Override
  protected void onActivityResult(int requestCode, int resultCode, Intent data) {
    super.onActivityResult(requestCode, resultCode, data);
    if (requestCode != FILE_PICKER_REQUEST) return;
    synchronized (pickedFiles) {
      FilePickerChoice choice = filePickerChoice;
      if (choice != null) choice.finish(resultCode == RESULT_OK && data != null ? data.getData() : null);
    }
  }

  private WebResourceResponse capabilityResponse(WebResourceRequest request) {
    Uri target = request.getUrl();
    String path = target.getPath();
    if (!isAssetUrl(target) || path == null || !path.startsWith(FILE_PICKER_ROUTE)) return null;
    String capability = path.substring(FILE_PICKER_ROUTE.length());
    String named = null;
    String encoded = null;
    for (java.util.Map.Entry<String, String> header : request.getRequestHeaders().entrySet()) {
      if ("X-Tachyon-Capability".equalsIgnoreCase(header.getKey())) named = header.getValue();
      if ("X-Tachyon-Capability-Request".equalsIgnoreCase(header.getKey())) encoded = header.getValue();
    }
    String payload = null;
    try { payload = encoded == null || encoded.length() > 196608 ? null : java.net.URLDecoder.decode(encoded, "UTF-8"); }
    catch (java.io.UnsupportedEncodingException | IllegalArgumentException ignored) { }
    java.util.Map<String, String> headers = new java.util.HashMap<>();
    headers.put("Cache-Control", "no-store");
    headers.put("X-Content-Type-Options", "nosniff");
    if (!"POST".equals(request.getMethod()) || !capability.startsWith("filePicker.") || capability.length() > 64
        || !capability.equals(named) || payload == null || !boundedJson(payload, 65536)) {
      return new WebResourceResponse("application/json", "UTF-8", 400, "Bad Request", headers,
          new ByteArrayInputStream(new byte[0]));
    }
    String answer = filePickerCall(capability.substring(11), payload);
    return new WebResourceResponse("application/json", "UTF-8", 200, "OK", headers,
        new ByteArrayInputStream(answer.getBytes(StandardCharsets.UTF_8)));
  }
