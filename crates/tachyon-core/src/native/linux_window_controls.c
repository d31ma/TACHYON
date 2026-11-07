static GtkWindow *g_native_window = NULL;

typedef unsigned char *(*TachyonCapabilityCall)(const char *, const char *, size_t *);
typedef void (*TachyonCapabilityFree)(unsigned char *, size_t);
typedef void (*TachyonCapabilitySetAdapter)(TachyonCapabilityCall, TachyonCapabilityFree);

static unsigned char *tachyon_capability_answer(const char *json, size_t *length) {
  *length = strlen(json);
  unsigned char *answer = g_malloc(*length);
  memcpy(answer, json, *length);
  return answer;
}

static void tachyon_capability_free(unsigned char *answer, size_t length) {
  (void)length;
  g_free(answer);
}

static gboolean tachyon_window_granted(const char *control) {
  if (control == NULL || g_native_window == NULL) return FALSE;
  __PERMITTED_OPERATIONS__
  return FALSE;
}

static gchar *tachyon_capability_string(JSCValue *request, const char *name) {
  JSCValue *value = jsc_value_object_get_property(request, name);
  gchar *text = jsc_value_is_string(value) ? jsc_value_to_string(value) : NULL;
  g_object_unref(value);
  return text;
}

static gboolean tachyon_capability_edge(JSCValue *request, const char *name, int *edge) {
  JSCValue *value = jsc_value_object_get_property(request, name);
  gboolean numeric = jsc_value_is_number(value);
  double number = numeric ? jsc_value_to_double(value) : 0;
  g_object_unref(value);
  if (!numeric || !(number >= 64 && number <= 16384)) return FALSE;
  *edge = (int)number;
  return number == *edge;
}

static gboolean tachyon_close_owned_window(gpointer window) {
  gtk_window_close(GTK_WINDOW(window));
  return G_SOURCE_REMOVE;
}

static const char *tachyon_window_action(const char *control, JSCValue *request) {
  if (!tachyon_window_granted(control)) return "{\"error\":\"denied\"}";
  if (strcmp(control, "minimize") == 0) gtk_window_minimize(g_native_window);
  else if (strcmp(control, "maximize") == 0) gtk_window_maximize(g_native_window);
  else if (strcmp(control, "fullscreen") == 0) {
    if (gtk_window_is_fullscreen(g_native_window)) gtk_window_unfullscreen(g_native_window);
    else gtk_window_fullscreen(g_native_window);
  } else if (strcmp(control, "close") == 0) {
    GtkWindow *window = g_native_window;
    g_object_remove_weak_pointer(G_OBJECT(window), (gpointer *)&g_native_window);
    g_native_window = NULL;
    g_idle_add_full(G_PRIORITY_DEFAULT, tachyon_close_owned_window,
                    g_object_ref(window), g_object_unref);
  } else if (strcmp(control, "resize") == 0) {
    int width = 0, height = 0;
    if (!tachyon_capability_edge(request, "width", &width) ||
        !tachyon_capability_edge(request, "height", &height)) return "{\"error\":\"invalid\"}";
    tachyon_window_content_size(g_native_window, width, height);
  } else if (strcmp(control, "title") == 0) {
    gchar *title = tachyon_capability_string(request, "title");
    gboolean valid = title != NULL && strlen(title) <= 1024 && g_utf8_validate(title, -1, NULL);
    if (valid) gtk_window_set_title(g_native_window, tachyon_title_bar_title(title));
    g_free(title);
    if (!valid) return "{\"error\":\"invalid\"}";
  } else return "{\"error\":\"unsupported\"}";
  return "{\"ok\":true}";
}

static unsigned char *tachyon_capability_call(const char *operation, const char *json, size_t *length) {
  if (length == NULL) return NULL;
  *length = 0;
  if (operation == NULL || json == NULL || strnlen(operation, 65) > 64 ||
      strnlen(json, 65537) > 65536 || !g_main_context_is_owner(g_main_context_default())) {
    return tachyon_capability_answer("{\"error\":\"unavailable\"}", length);
  }
  JSCContext *context = jsc_context_new();
  JSCValue *request = jsc_value_new_from_json(context, json);
  const char *answer = "{\"error\":\"invalid\"}";
  gchar *picked = NULL;
  if (request != NULL && jsc_value_is_object(request) && !jsc_value_is_array(request)) {
    if (strcmp(operation, "window.granted") == 0) {
      gchar *control = tachyon_capability_string(request, "control");
      answer = tachyon_window_granted(control) ? "{\"granted\":true}" : "{\"granted\":false}";
      g_free(control);
    } else if (g_str_has_prefix(operation, "window.")) {
      answer = tachyon_window_action(operation + strlen("window."), request);
    } else if (g_str_has_prefix(operation, "filePicker.")) {
      picked = tachyon_file_picker_call(operation + strlen("filePicker."), request);
      answer = picked;
    } else answer = "{\"error\":\"unsupported\"}";
  }
  unsigned char *reply = tachyon_capability_answer(answer, length);
  g_free(picked);
  if (request != NULL) g_object_unref(request);
  g_object_unref(context);
  return reply;
}

static gchar *tachyon_capability_page_call(const gchar *operation, const gchar *payload) {
  size_t length = 0;
  unsigned char *answer = tachyon_capability_call(operation, payload, &length);
  gchar *text = g_strndup((const gchar *)answer, length);
  tachyon_capability_free(answer, length);
  return text;
}
