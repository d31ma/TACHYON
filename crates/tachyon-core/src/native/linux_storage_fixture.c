#include <stdlib.h>

static gboolean g_storage_waiting = FALSE;
static gboolean g_storage_started = FALSE;
static gchar *g_storage_reply = NULL;

static void tachyon_storage_fail(const char *message) {
  fprintf(stderr, "linux storage fixture failed: %s (reply %s)\n", message, g_storage_reply != NULL ? g_storage_reply : "none");
  exit(1);
}

static void tachyon_storage_replied(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  JSCValue *value = webkit_web_view_call_async_javascript_function_finish(WEBKIT_WEB_VIEW(source), result, &error);
  if (value == NULL) tachyon_storage_fail(error != NULL ? error->message : "script failed");
  g_storage_reply = jsc_value_to_string(value);
  g_object_unref(value);
  g_storage_waiting = FALSE;
}

static gboolean tachyon_storage_tick(gpointer data) {
  (void)data;
  if (g_storage_waiting) return G_SOURCE_CONTINUE;
  if (!g_storage_started) {
    const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
    if (g_strcmp0(uri, "tachyon://app/") != 0 || webkit_web_view_is_loading(g_view)) return G_SOURCE_CONTINUE;
    g_storage_started = TRUE;
    g_storage_waiting = TRUE;
    webkit_web_view_call_async_javascript_function(g_view, "__STORAGE_SCRIPT__", -1, NULL, NULL, NULL, NULL, tachyon_storage_replied, NULL);
    return G_SOURCE_CONTINUE;
  }
  printf("linux storage: %s\n", g_storage_reply);
  g_application_quit(g_application_get_default());
  return G_SOURCE_REMOVE;
}

static gboolean tachyon_storage_deadline(gpointer data) {
  (void)data;
  tachyon_storage_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_timeout_add(20, tachyon_storage_tick, NULL);
  g_timeout_add_seconds(110, tachyon_storage_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  if (status != 0 || g_storage_reply == NULL) tachyon_storage_fail("the host did not finish its run");
  printf("linux storage run finished\n");
  return 0;
}
