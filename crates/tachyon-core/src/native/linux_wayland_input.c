#include <stdlib.h>

static const int TACHYON_WAYLAND_LEFT_BUTTON = 0x110;
static const guint32 TACHYON_WAYLAND_SHIFT = 0xffe1;

static GDBusConnection *g_wayland_input_bus = NULL;
static gchar *g_wayland_input_session = NULL;
static int g_wayland_pointer_x = 0, g_wayland_pointer_y = 0;

G_GNUC_UNUSED static gboolean tachyon_wayland_input_wanted(void) {
  GdkDisplay *display = gdk_display_get_default();
  return display != NULL && g_strcmp0(G_OBJECT_TYPE_NAME(display), "GdkWaylandDisplay") == 0;
}

static GVariant *tachyon_wayland_input_call(const char *path, const char *interface, const char *method, GVariant *arguments) {
  GError *error = NULL;
  GVariant *answer = g_dbus_connection_call_sync(g_wayland_input_bus, "org.gnome.Mutter.RemoteDesktop", path, interface, method, arguments, NULL,
                                                 G_DBUS_CALL_FLAGS_NONE, -1, NULL, &error);
  if (answer == NULL) {
    fprintf(stderr, "Mutter RemoteDesktop %s failed: %s\n", method, error != NULL ? error->message : "no answer");
    exit(1);
  }
  return answer;
}

G_GNUC_UNUSED static void tachyon_wayland_input_session(const char *method, GVariant *arguments) {
  g_variant_unref(tachyon_wayland_input_call(g_wayland_input_session, "org.gnome.Mutter.RemoteDesktop.Session", method, arguments));
}

G_GNUC_UNUSED static void tachyon_wayland_input_start(void) {
  if (g_wayland_input_session != NULL) return;
  g_wayland_input_bus = g_bus_get_sync(G_BUS_TYPE_SESSION, NULL, NULL);
  if (g_wayland_input_bus == NULL) {
    fprintf(stderr, "Wayland input needs the session bus of a headless mutter compositor\n");
    exit(1);
  }
  GVariant *created = tachyon_wayland_input_call("/org/gnome/Mutter/RemoteDesktop", "org.gnome.Mutter.RemoteDesktop", "CreateSession", NULL);
  g_variant_get(created, "(o)", &g_wayland_input_session);
  g_variant_unref(created);
  tachyon_wayland_input_session("Start", NULL);
  tachyon_wayland_input_session("NotifyKeyboardKeysym", g_variant_new("(ub)", TACHYON_WAYLAND_SHIFT, TRUE));
  tachyon_wayland_input_session("NotifyKeyboardKeysym", g_variant_new("(ub)", TACHYON_WAYLAND_SHIFT, FALSE));
  tachyon_wayland_input_session("NotifyPointerMotionRelative", g_variant_new("(dd)", -10000.0, -10000.0));
}

G_GNUC_UNUSED static void tachyon_wayland_pointer_to(int x, int y) {
  tachyon_wayland_input_start();
  tachyon_wayland_input_session("NotifyPointerMotionRelative", g_variant_new("(dd)", (double)(x - g_wayland_pointer_x), (double)(y - g_wayland_pointer_y)));
  g_wayland_pointer_x = x;
  g_wayland_pointer_y = y;
}

G_GNUC_UNUSED static void tachyon_wayland_click(void) {
  tachyon_wayland_input_start();
  tachyon_wayland_input_session("NotifyPointerButton", g_variant_new("(ib)", TACHYON_WAYLAND_LEFT_BUTTON, TRUE));
  tachyon_wayland_input_session("NotifyPointerButton", g_variant_new("(ib)", TACHYON_WAYLAND_LEFT_BUTTON, FALSE));
}

G_GNUC_UNUSED static void tachyon_wayland_key(unsigned long keysym, gboolean down) {
  tachyon_wayland_input_start();
  tachyon_wayland_input_session("NotifyKeyboardKeysym", g_variant_new("(ub)", (guint32)keysym, down));
}
