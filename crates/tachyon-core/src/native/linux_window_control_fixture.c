#include <stdlib.h>

static int g_window_step = 0;
static gboolean g_window_waiting = FALSE;
static gchar *g_window_reply = NULL;
static gint64 g_window_started = 0;

static void tachyon_window_fail(const char *message) {
  int width = g_native_window != NULL ? gtk_widget_get_width(GTK_WIDGET(g_native_window)) : -1;
  int height = g_native_window != NULL ? gtk_widget_get_height(GTK_WIDGET(g_native_window)) : -1;
  fprintf(stderr, "linux window control fixture failed at step %d after %.1f s: %s (reply %s, window %dx%d)\n", g_window_step,
          (double)(g_get_monotonic_time() - g_window_started) / G_USEC_PER_SEC, message,
          g_window_reply != NULL ? g_window_reply : "none", width, height);
  exit(1);
}

static void tachyon_window_require(gboolean valid, const char *message) {
  if (!valid) tachyon_window_fail(message);
}

static void tachyon_window_replied(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  JSCValue *value = webkit_web_view_call_async_javascript_function_finish(WEBKIT_WEB_VIEW(source), result, &error);
  if (value == NULL) tachyon_window_fail(error != NULL ? error->message : "script failed");
  g_free(g_window_reply);
  g_window_reply = jsc_value_to_string(value);
  g_object_unref(value);
  g_window_waiting = FALSE;
}

static void tachyon_window_run(const char *script) {
  g_window_waiting = TRUE;
  webkit_web_view_call_async_javascript_function(g_view, script, -1, NULL, NULL, NULL, NULL, tachyon_window_replied, NULL);
}

static void tachyon_window_script(void) {
  tachyon_window_run("__WINDOW_SCRIPT__");
}

static void tachyon_window_page_script(void) {
  tachyon_window_run("__PAGE_SCRIPT__");
}

static gboolean tachyon_window_host_refuses(const char *operation) {
  size_t length = 0;
  unsigned char *answer = tachyon_capability_call(operation, "{}", &length);
  gboolean refused = length == strlen("{\"error\":\"denied\"}") && memcmp(answer, "{\"error\":\"denied\"}", length) == 0;
  tachyon_capability_free(answer, length);
  return refused;
}

static void tachyon_window_check_page_calls(void) {
  tachyon_window_require(g_strcmp0(g_window_reply, "__PAGE_EXPECTED__") == 0, "the page runtime did not report the host's refusal");
  tachyon_window_require(g_native_window != NULL && gtk_widget_get_visible(GTK_WIDGET(g_native_window)), "a page close without its grant closed the window");
  tachyon_window_require(g_strcmp0(gtk_window_get_title(g_native_window), "Page title é漢") == 0, "the page title did not reach the GTK window");
}

static gboolean tachyon_window_has_size(int width, int height) {
  return gtk_widget_get_width(GTK_WIDGET(g_view)) == width && gtk_widget_get_height(GTK_WIDGET(g_view)) == height;
}

static void tachyon_window_check_calls(void) {
  tachyon_window_require(g_strcmp0(g_window_reply, "__WINDOW_EXPECTED__") == 0, "unexpected window control summary");
  tachyon_window_require(g_native_window != NULL && gtk_widget_get_visible(GTK_WIDGET(g_native_window)), "a denied close closed the window");
  tachyon_window_require(g_strcmp0(gtk_window_get_title(g_native_window), "Quarterly report é漢😀") == 0, "the granted title did not reach the GTK window");
  tachyon_window_require(tachyon_window_host_refuses("window.close"), "the host closed the window without the close grant");
  tachyon_window_require(tachyon_window_host_refuses("window.fullscreen"), "the host entered fullscreen without the fullscreen grant");
}

static gboolean tachyon_window_minimized(void) {
  GdkSurface *surface = gtk_native_get_surface(GTK_NATIVE(g_native_window));
  return surface != NULL && GDK_IS_TOPLEVEL(surface) &&
         (gdk_toplevel_get_state(GDK_TOPLEVEL(surface)) & GDK_TOPLEVEL_STATE_MINIMIZED) != 0;
}

static gboolean tachyon_window_tick(gpointer data) {
  (void)data;
  if (g_window_waiting) return G_SOURCE_CONTINUE;
  switch (g_window_step) {
    case 0: {
      const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
      if (g_strcmp0(uri, "tachyon://app/") != 0 || webkit_web_view_is_loading(g_view)) return G_SOURCE_CONTINUE;
      tachyon_window_require(!tachyon_window_minimized(), "the window started minimized");
      tachyon_window_page_script();
      break;
    }
    case 1: tachyon_window_check_page_calls(); break;
    case 2: if (!tachyon_window_has_size(700, 500)) return G_SOURCE_CONTINUE; tachyon_window_script(); break;
    case 3: tachyon_window_check_calls(); break;
    case 4: if (!tachyon_window_has_size(800, 600)) return G_SOURCE_CONTINUE; tachyon_window_script(); break;
    case 5: tachyon_window_require(g_strcmp0(g_window_reply, "minimize=") == 0, "the granted minimize failed"); break;
    case 6: if (!tachyon_window_minimized()) return G_SOURCE_CONTINUE; g_application_quit(g_application_get_default()); break;
    default: return G_SOURCE_REMOVE;
  }
  g_window_step += 1;
  printf("step %d done after %.1f s\n", g_window_step, (double)(g_get_monotonic_time() - g_window_started) / G_USEC_PER_SEC);
  return G_SOURCE_CONTINUE;
}

static gboolean tachyon_window_deadline(gpointer data) {
  (void)data;
  tachyon_window_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_window_started = g_get_monotonic_time();
  g_timeout_add(20, tachyon_window_tick, NULL);
  g_timeout_add_seconds(110, tachyon_window_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  tachyon_window_require(status == 0 && g_window_step == 7, "the host did not finish its run");
  printf("linux window controls passed: a JavaScript page caller set the title and resized to 700x500 and heard the host refuse close; a Rust service set the title, resized to 800x600 and minimized; close and fullscreen were refused\n");
  return 0;
}
