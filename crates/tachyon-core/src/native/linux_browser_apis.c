typedef void (*TachyonNotificationSend)(const char *id, const char *title, const char *body);

static gboolean tachyon_browser_api_granted(const char *api) {
  (void)api;
  __GRANTED_BROWSER_APIS__
  return FALSE;
}

static const char *tachyon_permission_api(WebKitPermissionRequest *request) {
  if (G_TYPE_CHECK_INSTANCE_TYPE(request, g_type_from_name("WebKitClipboardPermissionRequest"))) return "clipboard";
  if (WEBKIT_IS_GEOLOCATION_PERMISSION_REQUEST(request)) return "geolocation";
  if (WEBKIT_IS_NOTIFICATION_PERMISSION_REQUEST(request)) return "notifications";
  return NULL;
}

static gboolean tachyon_media_granted(void) {
  return tachyon_browser_api_granted("camera") || tachyon_browser_api_granted("microphone") || tachyon_browser_api_granted("screen");
}

static const gint TACHYON_PORTAL_TIMEOUT_MS = 2000;
static const gchar *TACHYON_SCREEN_SHARE_HIDDEN = "if (globalThis.MediaDevices) delete MediaDevices.prototype.getDisplayMedia;";

static gboolean tachyon_screen_cast_portal_answers(GDBusConnection *bus) {
  GVariant *reply = g_dbus_connection_call_sync(
      bus, "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.DBus.Properties", "Get",
      g_variant_new("(ss)", "org.freedesktop.portal.ScreenCast", "version"), G_VARIANT_TYPE("(v)"),
      G_DBUS_CALL_FLAGS_NONE, TACHYON_PORTAL_TIMEOUT_MS, NULL, NULL);
  if (reply == NULL) return FALSE;
  g_variant_unref(reply);
  return TRUE;
}

static gboolean tachyon_screen_cast_ready(void) {
  GDBusConnection *bus = g_bus_get_sync(G_BUS_TYPE_SESSION, NULL, NULL);
  if (bus == NULL) return FALSE;
  gboolean ready = tachyon_screen_cast_portal_answers(bus);
  g_object_unref(bus);
  return ready;
}

static const gchar *tachyon_screen_share_page(void) {
  return tachyon_browser_api_granted("screen") && tachyon_screen_cast_ready() ? "" : TACHYON_SCREEN_SHARE_HIDDEN;
}

typedef gpointer (*TachyonFeatureList)(void);
typedef gsize (*TachyonFeatureCount)(gpointer);
typedef gpointer (*TachyonFeatureAt)(gpointer, gsize);
typedef const char *(*TachyonFeatureName)(gpointer);
typedef void (*TachyonFeatureEnable)(WebKitSettings *, gpointer, gboolean);
typedef void (*TachyonFeatureListFree)(gpointer);

typedef struct {
  TachyonFeatureList all;
  TachyonFeatureCount count;
  TachyonFeatureAt at;
  TachyonFeatureName name;
  TachyonFeatureEnable enable;
  TachyonFeatureListFree free;
} TachyonFeatureCalls;

static const char *const TACHYON_SPEAKER_FEATURES[] = {"ExposeSpeakers", "PerElementSpeakerSelection"};

static gboolean tachyon_feature_calls(TachyonFeatureCalls *calls) {
  GModule *self = g_module_open(NULL, G_MODULE_BIND_LAZY);
  gboolean found = self != NULL &&
                   g_module_symbol(self, "webkit_settings_get_all_features", (gpointer *)&calls->all) &&
                   g_module_symbol(self, "webkit_feature_list_get_length", (gpointer *)&calls->count) &&
                   g_module_symbol(self, "webkit_feature_list_get", (gpointer *)&calls->at) &&
                   g_module_symbol(self, "webkit_feature_get_identifier", (gpointer *)&calls->name) &&
                   g_module_symbol(self, "webkit_settings_set_feature_enabled", (gpointer *)&calls->enable) &&
                   g_module_symbol(self, "webkit_feature_list_unref", (gpointer *)&calls->free);
  if (self != NULL) g_module_close(self);
  return found;
}

static gboolean tachyon_speaker_feature(const char *name) {
  for (gsize index = 0; index < G_N_ELEMENTS(TACHYON_SPEAKER_FEATURES); index++) {
    if (g_strcmp0(name, TACHYON_SPEAKER_FEATURES[index]) == 0) return TRUE;
  }
  return FALSE;
}

static void tachyon_speaker_choice(WebKitSettings *settings) {
  TachyonFeatureCalls calls = {NULL, NULL, NULL, NULL, NULL, NULL};
  if (!tachyon_media_granted() || !tachyon_feature_calls(&calls)) return;
  gpointer features = calls.all();
  for (gsize index = 0; index < calls.count(features); index++) {
    gpointer feature = calls.at(features, index);
    if (tachyon_speaker_feature(calls.name(feature))) calls.enable(settings, feature, TRUE);
  }
  calls.free(features);
}

static gboolean tachyon_media_kind_allowed(gboolean asked, const char *api) {
  return !asked || tachyon_browser_api_granted(api);
}

static gboolean tachyon_user_media_granted(WebKitUserMediaPermissionRequest *request) {
  return tachyon_media_kind_allowed(webkit_user_media_permission_is_for_audio_device(request), "microphone") &&
         tachyon_media_kind_allowed(webkit_user_media_permission_is_for_video_device(request), "camera") &&
         tachyon_media_kind_allowed(webkit_user_media_permission_is_for_display_device(request), "screen");
}

static gboolean tachyon_permission_granted(WebKitPermissionRequest *request) {
  if (WEBKIT_IS_USER_MEDIA_PERMISSION_REQUEST(request)) return tachyon_user_media_granted(WEBKIT_USER_MEDIA_PERMISSION_REQUEST(request));
  if (WEBKIT_IS_DEVICE_INFO_PERMISSION_REQUEST(request)) return tachyon_media_granted();
  const char *api = tachyon_permission_api(request);
  return api != NULL && tachyon_browser_api_granted(api);
}

static gboolean tachyon_permission_request(WebKitWebView *view, WebKitPermissionRequest *request, gpointer data) {
  (void)data;
  if (tachyon_trusted_view(view) && tachyon_permission_granted(request)) webkit_permission_request_allow(request);
  else webkit_permission_request_deny(request);
  return TRUE;
}

static gchar *tachyon_payload_text(JSCValue *values, const char *name) {
  JSCValue *value = jsc_value_object_get_property(values, name);
  gchar *text = jsc_value_is_string(value) ? jsc_value_to_string(value) : NULL;
  g_object_unref(value);
  if (text != NULL && text[0] == '\0') { g_free(text); text = NULL; }
  return text;
}

static gchar *tachyon_share_open(const char *payload) {
  if (!tachyon_browser_api_granted("share")) return g_strdup("{\"error\":\"tac.browserApis does not grant share.\"}");
  JSCContext *context = jsc_context_new();
  JSCValue *values = jsc_value_new_from_json(context, payload);
  gchar *shared = NULL;
  const char *names[] = {"url", "text", "title"};
  for (gsize index = 0; values != NULL && jsc_value_is_object(values) && shared == NULL && index < G_N_ELEMENTS(names); index++) {
    shared = tachyon_payload_text(values, names[index]);
  }
  if (values != NULL) g_object_unref(values);
  g_object_unref(context);
  if (shared == NULL) return g_strdup("{\"error\":\"Share needs a title, text or url.\"}");
  gdk_clipboard_set_text(gdk_display_get_clipboard(gdk_display_get_default()), shared);
  g_free(shared);
  return g_strdup("{\"value\":\"clipboard\"}");
}

static gchar *tachyon_storage_estimate(void) {
  GFile *folder = g_file_new_for_path(g_get_user_data_dir());
  GFileInfo *info = g_file_query_filesystem_info(folder, G_FILE_ATTRIBUTE_FILESYSTEM_FREE, NULL, NULL);
  guint64 quota = info != NULL ? g_file_info_get_attribute_uint64(info, G_FILE_ATTRIBUTE_FILESYSTEM_FREE) : 0;
  g_clear_object(&info);
  g_object_unref(folder);
  return g_strdup_printf("{\"value\":{\"quota\":%" G_GUINT64_FORMAT ",\"persisted\":true}}", quota);
}

static gchar *tachyon_storage_persist(void) {
  return g_strdup(tachyon_browser_api_granted("persistentStorage") ? "{\"value\":true}" : "{\"error\":\"tac.browserApis does not grant persistentStorage.\"}");
}

static void tachyon_send_notification(const char *id, const char *title, const char *body) {
  GNotification *notification = g_notification_new(title);
  if (body != NULL) g_notification_set_body(notification, body);
  g_application_send_notification(g_application_get_default(), id, notification);
  g_object_unref(notification);
}

static TachyonNotificationSend g_notification_send = tachyon_send_notification;
static guint64 g_notification_count = 0;

static gchar *tachyon_notification_permission(void) {
  return g_strdup(tachyon_browser_api_granted("notifications") ? "{\"value\":\"granted\"}" : "{\"value\":\"denied\"}");
}

static gchar *tachyon_notification_show(const char *payload) {
  if (!tachyon_browser_api_granted("notifications")) return g_strdup("{\"error\":\"tac.browserApis does not grant notifications.\"}");
  JSCContext *context = jsc_context_new();
  JSCValue *values = jsc_value_new_from_json(context, payload);
  gchar *title = values != NULL && jsc_value_is_object(values) ? tachyon_payload_text(values, "title") : NULL;
  gchar *body = values != NULL && jsc_value_is_object(values) ? tachyon_payload_text(values, "body") : NULL;
  if (values != NULL) g_object_unref(values);
  g_object_unref(context);
  if (title == NULL) { g_free(body); return g_strdup("{\"error\":\"A notification needs a title.\"}"); }
  g_notification_count += 1;
  gchar *id = g_strdup_printf("tachyon-%" G_GUINT64_FORMAT, g_notification_count);
  g_notification_send(id, title, body);
  g_free(id); g_free(title); g_free(body);
  return g_strdup("{\"value\":null}");
}
