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

static gboolean tachyon_permission_request(WebKitWebView *view, WebKitPermissionRequest *request, gpointer data) {
  (void)data;
  const char *api = tachyon_permission_api(request);
  if (api != NULL && tachyon_trusted_view(view) && tachyon_browser_api_granted(api)) webkit_permission_request_allow(request);
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
