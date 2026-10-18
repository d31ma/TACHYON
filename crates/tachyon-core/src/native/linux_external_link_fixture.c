#include <stdlib.h>

static gboolean g_link_waiting = FALSE;
static gboolean g_link_started = FALSE;
static gchar *g_link_reply = NULL;
static GString *g_link_opened = NULL;

static void tachyon_link_fail(const char *message) {
  fprintf(stderr, "linux external link fixture failed: %s (reply %s, opened %s)\n", message,
          g_link_reply != NULL ? g_link_reply : "none", g_link_opened != NULL ? g_link_opened->str : "none");
  exit(1);
}

static void tachyon_link_record(const char *uri) {
  if (g_link_opened->len > 0) g_string_append_c(g_link_opened, ' ');
  g_string_append(g_link_opened, uri);
}

static void tachyon_link_replied(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  JSCValue *value = webkit_web_view_call_async_javascript_function_finish(WEBKIT_WEB_VIEW(source), result, &error);
  if (value == NULL) tachyon_link_fail(error != NULL ? error->message : "script failed");
  g_link_reply = jsc_value_to_string(value);
  g_object_unref(value);
  g_link_waiting = FALSE;
}

static gboolean tachyon_link_tick(gpointer data) {
  (void)data;
  if (g_link_waiting) return G_SOURCE_CONTINUE;
  if (!g_link_started) {
    const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
    if (g_strcmp0(uri, "tachyon://app/") != 0 || webkit_web_view_is_loading(g_view)) return G_SOURCE_CONTINUE;
    g_link_started = TRUE;
    g_link_waiting = TRUE;
    g_external_launch = tachyon_link_record;
    webkit_settings_set_javascript_can_open_windows_automatically(webkit_web_view_get_settings(g_view), TRUE);
    webkit_web_view_call_async_javascript_function(g_view, "__LINK_SCRIPT__", -1, NULL, NULL, NULL, NULL, tachyon_link_replied, NULL);
    return G_SOURCE_CONTINUE;
  }
  printf("linux external links: page %s; opened %s\n", g_link_reply, g_link_opened->str);
  if (g_strcmp0(g_link_reply, "tachyon://app/") != 0) tachyon_link_fail("the page left the application");
  if (g_strcmp0(g_link_opened->str, "__LINK_EXPECTED__") != 0) tachyon_link_fail("unexpected external links");
  g_application_quit(g_application_get_default());
  return G_SOURCE_REMOVE;
}

static gboolean tachyon_link_deadline(gpointer data) {
  (void)data;
  tachyon_link_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_link_opened = g_string_new(NULL);
  g_timeout_add(20, tachyon_link_tick, NULL);
  g_timeout_add_seconds(110, tachyon_link_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  if (status != 0 || g_link_reply == NULL) tachyon_link_fail("the host did not finish its run");
  printf("linux external links passed\n");
  return 0;
}
