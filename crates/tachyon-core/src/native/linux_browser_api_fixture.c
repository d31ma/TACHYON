#include <stdlib.h>

typedef void *(*TachyonOpenDisplay)(const char *);
typedef int (*TachyonWarpPointer)(void *, unsigned long, unsigned long, int, int, unsigned int, unsigned int, int, int);
typedef int (*TachyonFakeButton)(void *, unsigned int, int, unsigned long);
typedef int (*TachyonDisplayCall)(void *);
typedef int (*TachyonRaiseWindow)(void *, unsigned long);
typedef unsigned long (*TachyonSurfaceWindow)(GdkSurface *);

static gboolean g_browser_waiting = FALSE;
static gchar *g_browser_reply = NULL;
static gchar *g_browser_clipboard = NULL;
static GString *g_browser_notified = NULL;

static void tachyon_browser_fail(const char *message) {
  fprintf(stderr, "linux browser API fixture failed: %s (reply %s, notified %s, clipboard %s)\n", message,
          g_browser_reply != NULL ? g_browser_reply : "none", g_browser_notified->str,
          g_browser_clipboard != NULL ? g_browser_clipboard : "none");
  exit(1);
}

static void tachyon_browser_notified(const char *id, const char *title, const char *body) {
  if (g_browser_notified->len > 0) g_string_append_c(g_browser_notified, ' ');
  g_string_append_printf(g_browser_notified, "%s|%s", title, body != NULL ? body : "");
  tachyon_send_notification(id, title, body);
}

static gboolean tachyon_browser_position(WebKitGeolocationManager *manager, gpointer data) {
  (void)data;
  WebKitGeolocationPosition *position = webkit_geolocation_position_new(51.5, -0.125, 10);
  webkit_geolocation_manager_update_position(manager, position);
  webkit_geolocation_position_free(position);
  return TRUE;
}

static gpointer tachyon_browser_symbol(GModule *module, const char *name) {
  gpointer symbol = NULL;
  if (module == NULL || !g_module_symbol(module, name, &symbol)) tachyon_browser_fail("a real click needs libX11, libXtst and the X11 GDK backend");
  return symbol;
}

static gboolean tachyon_browser_click(gpointer data) {
  (void)data;
  if (!g_browser_waiting) return G_SOURCE_REMOVE;
  GModule *x11 = g_module_open("libX11.so.6", G_MODULE_BIND_LAZY);
  GModule *xtest = g_module_open("libXtst.so.6", G_MODULE_BIND_LAZY);
  GModule *program = g_module_open(NULL, G_MODULE_BIND_LAZY);
  TachyonOpenDisplay open_display = tachyon_browser_symbol(x11, "XOpenDisplay");
  TachyonWarpPointer warp_pointer = tachyon_browser_symbol(x11, "XWarpPointer");
  TachyonDisplayCall flush = tachyon_browser_symbol(x11, "XFlush");
  TachyonDisplayCall close_display = tachyon_browser_symbol(x11, "XCloseDisplay");
  TachyonRaiseWindow raise_window = tachyon_browser_symbol(x11, "XRaiseWindow");
  TachyonFakeButton fake_button = tachyon_browser_symbol(xtest, "XTestFakeButtonEvent");
  TachyonSurfaceWindow surface_window = tachyon_browser_symbol(program, "gdk_x11_surface_get_xid");
  void *display = open_display(NULL);
  if (display == NULL) tachyon_browser_fail("no X11 display");
  GtkWidget *window = GTK_WIDGET(g_native_window);
  unsigned long native = surface_window(gtk_native_get_surface(GTK_NATIVE(window)));
  raise_window(display, native);
  warp_pointer(display, 0, native, 0, 0, 0, 0, gtk_widget_get_width(window) / 2, gtk_widget_get_height(window) / 2);
  fake_button(display, 1, 1, 0);
  fake_button(display, 1, 0, 0);
  flush(display);
  close_display(display);
  return G_SOURCE_CONTINUE;
}

static void tachyon_browser_finish(void) {
  printf("linux browser APIs: %s; notified %s; clipboard %s\n", g_browser_reply, g_browser_notified->str, g_browser_clipboard);
  if (g_strcmp0(g_browser_reply, "__BROWSER_EXPECTED__") != 0) tachyon_browser_fail("unexpected browser API summary");
  if (g_strcmp0(g_browser_notified->str, "__BROWSER_NOTIFIED__") != 0) tachyon_browser_fail("unexpected notifications");
  if (g_strcmp0(g_browser_clipboard, "__BROWSER_CLIPBOARD__") != 0) tachyon_browser_fail("unexpected clipboard text");
  g_application_quit(g_application_get_default());
}

static void tachyon_browser_clipboard_read(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  g_browser_clipboard = gdk_clipboard_read_text_finish(GDK_CLIPBOARD(source), result, &error);
  if (g_browser_clipboard == NULL) g_browser_clipboard = g_strdup(error != NULL ? error->message : "empty");
  g_clear_error(&error);
  tachyon_browser_finish();
}

static void tachyon_browser_replied(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  JSCValue *value = webkit_web_view_call_async_javascript_function_finish(WEBKIT_WEB_VIEW(source), result, &error);
  if (value == NULL) tachyon_browser_fail(error != NULL ? error->message : "script failed");
  g_browser_reply = jsc_value_to_string(value);
  g_object_unref(value);
  g_browser_waiting = FALSE;
  gdk_clipboard_read_text_async(gdk_display_get_clipboard(gdk_display_get_default()), NULL, tachyon_browser_clipboard_read, NULL);
}

static gboolean tachyon_browser_tick(gpointer data) {
  (void)data;
  const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
  if (g_strcmp0(uri, "tachyon://app/") != 0 || webkit_web_view_is_loading(g_view)) return G_SOURCE_CONTINUE;
  g_browser_waiting = TRUE;
  g_notification_send = tachyon_browser_notified;
  WebKitGeolocationManager *geolocation = webkit_web_context_get_geolocation_manager(webkit_web_view_get_context(g_view));
  g_signal_connect(geolocation, "start", G_CALLBACK(tachyon_browser_position), NULL);
  gdk_clipboard_set_text(gdk_display_get_clipboard(gdk_display_get_default()), "from another app");
  webkit_web_view_call_async_javascript_function(g_view, "__BROWSER_SCRIPT__", -1, NULL, NULL, NULL, NULL, tachyon_browser_replied, NULL);
  g_timeout_add(400, tachyon_browser_click, NULL);
  return G_SOURCE_REMOVE;
}

static gboolean tachyon_browser_deadline(gpointer data) {
  (void)data;
  tachyon_browser_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_setenv("GDK_BACKEND", "x11", TRUE);
  g_browser_notified = g_string_new(NULL);
  g_timeout_add(20, tachyon_browser_tick, NULL);
  g_timeout_add_seconds(110, tachyon_browser_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  if (status != 0 || g_browser_clipboard == NULL) tachyon_browser_fail("the host did not finish its run");
  printf("linux browser APIs passed\n");
  return 0;
}
