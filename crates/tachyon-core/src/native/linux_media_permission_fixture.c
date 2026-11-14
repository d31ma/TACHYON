#include <stdlib.h>

static gboolean g_media_waiting = FALSE;
static gboolean g_media_started = FALSE;
static gchar *g_media_reply = NULL;

static void tachyon_media_fail(const char *message) {
  fprintf(stderr, "linux media permission fixture failed: %s (reply %s)\n", message, g_media_reply != NULL ? g_media_reply : "none");
  exit(1);
}

static void tachyon_media_replied(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  JSCValue *value = webkit_web_view_call_async_javascript_function_finish(WEBKIT_WEB_VIEW(source), result, &error);
  if (value == NULL) tachyon_media_fail(error != NULL ? error->message : "script failed");
  g_media_reply = jsc_value_to_string(value);
  g_object_unref(value);
  g_media_waiting = FALSE;
}

static gboolean tachyon_media_tick(gpointer data) {
  (void)data;
  if (g_media_waiting) return G_SOURCE_CONTINUE;
  if (!g_media_started) {
    const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
    if (g_strcmp0(uri, "tachyon://app/") != 0 || webkit_web_view_is_loading(g_view)) return G_SOURCE_CONTINUE;
    g_media_started = TRUE;
    g_media_waiting = TRUE;
    webkit_settings_set_enable_mock_capture_devices(webkit_web_view_get_settings(g_view), TRUE);
    webkit_web_view_call_async_javascript_function(g_view, "__MEDIA_SCRIPT__", -1, NULL, NULL, NULL, NULL, tachyon_media_replied, NULL);
    return G_SOURCE_CONTINUE;
  }
  printf("linux media permissions: %s\n", g_media_reply);
  if (g_strcmp0(g_media_reply, "__MEDIA_EXPECTED__") != 0) tachyon_media_fail("unexpected media summary");
  g_application_quit(g_application_get_default());
  return G_SOURCE_REMOVE;
}

static gboolean tachyon_media_deadline(gpointer data) {
  (void)data;
  tachyon_media_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_timeout_add(20, tachyon_media_tick, NULL);
  g_timeout_add_seconds(110, tachyon_media_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  if (status != 0 || g_media_reply == NULL) tachyon_media_fail("the host did not finish its run");
  printf("linux media permissions passed\n");
  return 0;
}
