#define TACHYON_WEBKITGTK_MAJOR __WEBKITGTK_MAJOR__u
#define TACHYON_WEBKITGTK_MINOR __WEBKITGTK_MINOR__u

static gboolean g_web_engine_too_old = FALSE;

static gboolean tachyon_web_engine_ready(void) {
  guint installed = webkit_get_major_version() * 10000u + webkit_get_minor_version();
  return installed >= TACHYON_WEBKITGTK_MAJOR * 10000u + TACHYON_WEBKITGTK_MINOR;
}

static void tachyon_show_web_engine_message(GtkApplication *app) {
  GtkWindow *shown = gtk_application_get_active_window(app);
  if (shown != NULL) { gtk_window_present(shown); return; }
  gchar *message = g_strdup_printf(
      "%s needs WebKitGTK %u.%u or newer, and this system has %u.%u.\n"
      "Update the webkitgtk-6.0 package, then open %s again.",
      TACHYON_APP_NAME, TACHYON_WEBKITGTK_MAJOR, TACHYON_WEBKITGTK_MINOR,
      webkit_get_major_version(), webkit_get_minor_version(), TACHYON_APP_NAME);
  g_printerr("%s\n", message);
  GtkWidget *window = gtk_application_window_new(app);
  gtk_window_set_title(GTK_WINDOW(window), TACHYON_APP_NAME);
  gtk_window_set_default_size(GTK_WINDOW(window), 480, 160);
  GtkWidget *label = gtk_label_new(message);
  gtk_label_set_wrap(GTK_LABEL(label), TRUE);
  gtk_label_set_selectable(GTK_LABEL(label), TRUE);
  gtk_widget_set_margin_top(label, 24);
  gtk_widget_set_margin_bottom(label, 24);
  gtk_widget_set_margin_start(label, 24);
  gtk_widget_set_margin_end(label, 24);
  gtk_window_set_child(GTK_WINDOW(window), label);
  gtk_window_present(GTK_WINDOW(window));
  g_free(message);
}
