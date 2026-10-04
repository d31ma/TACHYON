#include <stdlib.h>

static gboolean g_page_waiting = FALSE;
static gboolean g_page_started = FALSE;
static gchar *g_page_reply = NULL;

static void tachyon_page_fail(const char *message) {
  fprintf(stderr, "linux page call fixture failed: %s (reply %s)\n", message, g_page_reply != NULL ? g_page_reply : "none");
  exit(1);
}

static void tachyon_page_replied(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  JSCValue *value = webkit_web_view_call_async_javascript_function_finish(WEBKIT_WEB_VIEW(source), result, &error);
  if (value == NULL) tachyon_page_fail(error != NULL ? error->message : "script failed");
  g_page_reply = jsc_value_to_string(value);
  g_object_unref(value);
  g_page_waiting = FALSE;
}

static gboolean tachyon_page_tick(gpointer data) {
  (void)data;
  if (g_page_waiting) return G_SOURCE_CONTINUE;
  if (!g_page_started) {
    const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
    if (g_strcmp0(uri, "tachyon://app/") != 0 || webkit_web_view_is_loading(g_view)) return G_SOURCE_CONTINUE;
    g_page_started = TRUE;
    g_page_waiting = TRUE;
    webkit_web_view_call_async_javascript_function(g_view, "__PAGE_SCRIPT__", -1, NULL, NULL, NULL, NULL, tachyon_page_replied, NULL);
    return G_SOURCE_CONTINUE;
  }
  printf("linux page calls: %s\n", g_page_reply);
  if (g_strcmp0(g_page_reply, "__PAGE_EXPECTED__") != 0) tachyon_page_fail("unexpected page call summary");
  g_application_quit(g_application_get_default());
  return G_SOURCE_REMOVE;
}

static gboolean tachyon_page_deadline(gpointer data) {
  (void)data;
  tachyon_page_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_timeout_add(20, tachyon_page_tick, NULL);
  g_timeout_add_seconds(110, tachyon_page_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  if (status != 0 || g_page_reply == NULL) tachyon_page_fail("the host did not finish its run");
  printf("linux page calls passed\n");
  return 0;
}
