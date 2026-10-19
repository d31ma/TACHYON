#include <stdlib.h>

static int g_grant_step = 0;
static gboolean g_grant_waiting = FALSE;
static gchar *g_grant_reply = NULL;
static gint64 g_grant_started = 0;

static void tachyon_grant_fail(const char *message) {
  fprintf(stderr, "linux window grant fixture failed at step %d after %.1f s: %s (reply %s)\n", g_grant_step,
          (double)(g_get_monotonic_time() - g_grant_started) / G_USEC_PER_SEC, message, g_grant_reply != NULL ? g_grant_reply : "none");
  exit(1);
}

static void tachyon_grant_require(gboolean valid, const char *message) {
  if (!valid) tachyon_grant_fail(message);
}

static void tachyon_grant_replied(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  JSCValue *value = webkit_web_view_call_async_javascript_function_finish(WEBKIT_WEB_VIEW(source), result, &error);
  if (value == NULL) tachyon_grant_fail(error != NULL ? error->message : "script failed");
  g_free(g_grant_reply);
  g_grant_reply = jsc_value_to_string(value);
  g_object_unref(value);
  g_grant_waiting = FALSE;
}

static void tachyon_grant_closing(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  JSCValue *value = webkit_web_view_call_async_javascript_function_finish(WEBKIT_WEB_VIEW(source), result, NULL);
  if (value != NULL) g_object_unref(value);
}

static void tachyon_grant_call(GAsyncReadyCallback replied) {
  g_grant_waiting = replied == tachyon_grant_replied;
  webkit_web_view_call_async_javascript_function(g_view, "__GRANT_SCRIPT__", -1, NULL, NULL, NULL, NULL, replied, NULL);
}

static void tachyon_grant_expect(const char *reply) {
  tachyon_grant_require(g_strcmp0(g_grant_reply, reply) == 0, "a granted window control was refused");
}

static gboolean tachyon_grant_tick(gpointer data) {
  (void)data;
  if (g_grant_waiting) return G_SOURCE_CONTINUE;
  switch (g_grant_step) {
    case 0: {
      const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
      if (g_strcmp0(uri, "tachyon://app/") != 0 || webkit_web_view_is_loading(g_view)) return G_SOURCE_CONTINUE;
      tachyon_grant_require(!gtk_window_is_maximized(g_native_window) && !gtk_window_is_fullscreen(g_native_window), "the window started maximized or full screen");
      tachyon_grant_call(tachyon_grant_replied);
      break;
    }
    case 1:
      tachyon_grant_expect("maximize=");
      if (!gtk_window_is_maximized(g_native_window)) return G_SOURCE_CONTINUE;
      tachyon_grant_call(tachyon_grant_replied);
      break;
    case 2:
      tachyon_grant_expect("fullscreen=");
      if (!gtk_window_is_fullscreen(g_native_window)) return G_SOURCE_CONTINUE;
      tachyon_grant_call(tachyon_grant_replied);
      break;
    case 3:
      tachyon_grant_expect("fullscreen=");
      if (gtk_window_is_fullscreen(g_native_window)) return G_SOURCE_CONTINUE;
      tachyon_grant_call(tachyon_grant_closing);
      break;
    default: return G_SOURCE_REMOVE;
  }
  g_grant_step += 1;
  printf("step %d done after %.1f s\n", g_grant_step, (double)(g_get_monotonic_time() - g_grant_started) / G_USEC_PER_SEC);
  return G_SOURCE_CONTINUE;
}

static gboolean tachyon_grant_deadline(gpointer data) {
  (void)data;
  tachyon_grant_fail("deadline: the granted close did not end the application");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_grant_started = g_get_monotonic_time();
  g_timeout_add(20, tachyon_grant_tick, NULL);
  g_timeout_add_seconds(110, tachyon_grant_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  tachyon_grant_require(status == 0 && g_grant_step == 4, "the host did not finish its run");
  printf("linux window grants passed: a Rust service maximized the window, entered and left full screen, and closed it, which ended the application\n");
  return 0;
}
