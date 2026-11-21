#include <gdk/x11/gdkx.h>

typedef int (*TachyonFakeKey)(Display *, unsigned int, int, unsigned long);
typedef unsigned char (*TachyonKeysymToKeycode)(Display *, unsigned long);
typedef int (*TachyonFlush)(Display *);

#define TACHYON_FIXTURE_CONTROL 0xffe3

static TachyonFakeKey g_fixture_key = NULL;
static TachyonKeysymToKeycode g_fixture_keycode = NULL;
static TachyonFlush g_fixture_flush = NULL;

static void tachyon_fixture_keys_require(gboolean valid, const char *message);

static void tachyon_fixture_load_x11(void) {
  GModule *test = g_module_open("libXtst.so.6", G_MODULE_BIND_LAZY);
  GModule *self = g_module_open(NULL, G_MODULE_BIND_LAZY);
  tachyon_fixture_keys_require(test != NULL && self != NULL, "libXtst.so.6 is not installed");
  tachyon_fixture_keys_require(g_module_symbol(test, "XTestFakeKeyEvent", (gpointer *)&g_fixture_key) &&
                         g_module_symbol(self, "XKeysymToKeycode", (gpointer *)&g_fixture_keycode) &&
                         g_module_symbol(self, "XFlush", (gpointer *)&g_fixture_flush),
                         "the X11 test functions are missing");
}

static unsigned int tachyon_fixture_keycode(Display *display, unsigned long keysym) {
  unsigned int keycode = g_fixture_keycode(display, keysym);
  tachyon_fixture_keys_require(keycode != 0, "the X11 keymap has no key the fixture needs");
  return keycode;
}

static void tachyon_fixture_wayland_press(unsigned long keysym, gboolean control) {
  if (control) tachyon_wayland_key(TACHYON_FIXTURE_CONTROL, TRUE);
  tachyon_wayland_key(keysym, TRUE);
  tachyon_wayland_key(keysym, FALSE);
  if (control) tachyon_wayland_key(TACHYON_FIXTURE_CONTROL, FALSE);
}

static void tachyon_fixture_press(unsigned long keysym, gboolean control) {
  if (tachyon_wayland_input_wanted()) {
    tachyon_fixture_wayland_press(keysym, control);
    return;
  }
  if (g_fixture_key == NULL) tachyon_fixture_load_x11();
  GdkDisplay *display = gdk_display_get_default();
  tachyon_fixture_keys_require(GDK_IS_X11_DISPLAY(display), "the fixture needs an X11 display (GDK_BACKEND=x11)");
  G_GNUC_BEGIN_IGNORE_DEPRECATIONS
  Display *xdisplay = gdk_x11_display_get_xdisplay(display);
  G_GNUC_END_IGNORE_DEPRECATIONS
  unsigned int keycode = tachyon_fixture_keycode(xdisplay, keysym);
  unsigned int modifier = tachyon_fixture_keycode(xdisplay, TACHYON_FIXTURE_CONTROL);
  if (control) g_fixture_key(xdisplay, modifier, True, 0);
  g_fixture_key(xdisplay, keycode, True, 0);
  g_fixture_key(xdisplay, keycode, False, 0);
  if (control) g_fixture_key(xdisplay, modifier, False, 0);
  g_fixture_flush(xdisplay);
}

static gboolean tachyon_fixture_wayland_focused(void) {
  if (!tachyon_wayland_input_wanted() || gtk_window_is_active(g_native_window)) return TRUE;
  if (!gtk_window_is_maximized(g_native_window)) {
    gtk_window_maximize(g_native_window);
    return FALSE;
  }
  GtkWidget *window = GTK_WIDGET(g_native_window);
  tachyon_wayland_pointer_to(gtk_widget_get_width(window) / 2, gtk_widget_get_height(window) / 2);
  tachyon_wayland_click();
  return FALSE;
}
