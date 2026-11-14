#include <stdlib.h>

static const gchar *TACHYON_FAKE_PORTAL_XML =
    "<node><interface name='org.freedesktop.portal.ScreenCast'><property name='version' type='u' access='read'/></interface></node>";
static gboolean g_media_waiting = FALSE;
static gint g_fake_portal_ready = 0;
static const guint32 TACHYON_NAME_DO_NOT_QUEUE = 4;
static const guint32 TACHYON_NAME_PRIMARY_OWNER = 1;
static gboolean g_media_started = FALSE;
static gchar *g_media_reply = NULL;

static void tachyon_media_fail(const char *message) {
  fprintf(stderr, "linux media permission fixture failed: %s (reply %s)\n", message, g_media_reply != NULL ? g_media_reply : "none");
  exit(1);
}

static GVariant *tachyon_fake_portal_property(GDBusConnection *bus, const gchar *sender, const gchar *path,
                                              const gchar *interface, const gchar *name, GError **error, gpointer data) {
  (void)bus; (void)sender; (void)path; (void)interface; (void)name; (void)error; (void)data;
  return g_variant_new_uint32(5);
}

static gpointer tachyon_fake_portal_run(gpointer data) {
  (void)data;
  static const GDBusInterfaceVTable vtable = {NULL, tachyon_fake_portal_property, NULL, {0}};
  GMainContext *context = g_main_context_new();
  g_main_context_push_thread_default(context);
  GDBusConnection *bus = g_bus_get_sync(G_BUS_TYPE_SESSION, NULL, NULL);
  GDBusNodeInfo *node = g_dbus_node_info_new_for_xml(TACHYON_FAKE_PORTAL_XML, NULL);
  if (bus == NULL || node == NULL) tachyon_media_fail("no session bus for the fake ScreenCast portal");
  g_dbus_connection_register_object(bus, "/org/freedesktop/portal/desktop", node->interfaces[0], &vtable, NULL, NULL, NULL);
  GVariant *owned = g_dbus_connection_call_sync(bus, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus",
                                                "RequestName", g_variant_new("(su)", "org.freedesktop.portal.Desktop", TACHYON_NAME_DO_NOT_QUEUE),
                                                G_VARIANT_TYPE("(u)"), G_DBUS_CALL_FLAGS_NONE, -1, NULL, NULL);
  guint32 answer = 0;
  if (owned != NULL) g_variant_get(owned, "(u)", &answer);
  if (answer != TACHYON_NAME_PRIMARY_OWNER) tachyon_media_fail("the fake ScreenCast portal could not take its name; another portal owns it");
  g_variant_unref(owned);
  g_atomic_int_set(&g_fake_portal_ready, 1);
  GMainLoop *loop = g_main_loop_new(context, FALSE);
  g_main_loop_run(loop);
  return NULL;
}

static void tachyon_fake_portal_start(void) {
  g_thread_new("fake-screen-cast-portal", tachyon_fake_portal_run, NULL);
  while (!g_atomic_int_get(&g_fake_portal_ready)) g_usleep(10000);
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
  if (__MEDIA_FAKE_PORTAL__) tachyon_fake_portal_start();
  g_timeout_add(20, tachyon_media_tick, NULL);
  g_timeout_add_seconds(110, tachyon_media_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  if (status != 0 || g_media_reply == NULL) tachyon_media_fail("the host did not finish its run");
  printf("linux media permissions passed\n");
  return 0;
}
