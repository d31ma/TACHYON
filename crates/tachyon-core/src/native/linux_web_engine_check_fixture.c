static gboolean tachyon_fixture_check(gpointer data) {
  (void)data;
  GApplication *application = g_application_get_default();
  GtkWindow *shown = application != NULL
      ? gtk_application_get_active_window(GTK_APPLICATION(application)) : NULL;
  GtkWidget *child = shown != NULL ? gtk_window_get_child(shown) : NULL;
  gboolean passed = g_web_engine_too_old && g_view == NULL && g_application_start == NULL &&
                    child != NULL && GTK_IS_LABEL(child);
  if (passed) {
    printf("old web engine message shown without a page: %s\n", gtk_label_get_text(GTK_LABEL(child)));
  } else {
    fprintf(stderr, "linux web engine fixture failed: too old %d, view %s, start %s, label %s\n",
            g_web_engine_too_old, g_view != NULL ? "present" : "missing",
            g_application_start != NULL ? "loaded" : "missing", child != NULL ? "present" : "missing");
  }
  fflush(stdout);
  if (application != NULL) g_application_quit(application);
  if (!passed) exit(1);
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_timeout_add(3000, tachyon_fixture_check, NULL);
  return tachyon_host_main(argc, argv);
}
