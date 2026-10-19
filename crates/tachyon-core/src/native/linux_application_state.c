static const gchar *g_application_state = "foreground";
static guint g_application_state_check = 0;

static void tachyon_application_state_post(void) {
  if (!tachyon_trusted_view(g_view)) return;
  gchar *script = g_strdup_printf("globalThis.__tachyonApplicationState?.('%s')", g_application_state);
  webkit_web_view_evaluate_javascript(g_view, script, -1, NULL, NULL, NULL, NULL, NULL);
  g_free(script);
}

static void tachyon_application_state_send(const gchar *state) {
  if (g_strcmp0(state, g_application_state) == 0) return;
  g_application_state = state;
  tachyon_record(g_strcmp0(state, "background") == 0 ? "application.background" : "application.foreground");
  tachyon_application_state_post();
}

static gboolean tachyon_application_state_check(gpointer data) {
  (void)data;
  g_application_state_check = 0;
  gboolean foreground = FALSE;
  GList *windows = gtk_window_list_toplevels();
  for (GList *item = windows; item != NULL; item = item->next) {
    GtkWindow *window = GTK_WINDOW(item->data);
    foreground = foreground || (gtk_window_is_active(window) && !gtk_window_is_suspended(window));
  }
  g_list_free(windows);
  tachyon_application_state_send(foreground ? "foreground" : "background");
  return G_SOURCE_REMOVE;
}

static void tachyon_application_state_changed(GObject *window, GParamSpec *property, gpointer data) {
  (void)window;
  (void)property;
  (void)data;
  if (g_application_state_check == 0) g_application_state_check = g_idle_add(tachyon_application_state_check, NULL);
}

static void tachyon_application_state_watch(GtkWindow *window) {
  g_signal_connect(window, "notify::is-active", G_CALLBACK(tachyon_application_state_changed), NULL);
  g_signal_connect(window, "notify::suspended", G_CALLBACK(tachyon_application_state_changed), NULL);
}
