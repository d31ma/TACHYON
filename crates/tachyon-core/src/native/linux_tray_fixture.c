#include <stdlib.h>

static const gchar TACHYON_FIXTURE_WATCHER[] =
    "<node><interface name='org.kde.StatusNotifierWatcher'>"
    "<method name='RegisterStatusNotifierItem'><arg type='s' direction='in'/></method>"
    "</interface></node>";

static GDBusConnection *g_fixture_bus = NULL;
static gchar *g_fixture_item = NULL;
static GVariant *g_fixture_reply = NULL;
static gboolean g_fixture_waiting = FALSE;
static int g_fixture_step = 0;
static gint64 g_fixture_started = 0;

typedef gint64 (*TachyonFixtureCount)(const char *);
static TachyonFixtureCount g_fixture_count = NULL;

static void tachyon_fixture_fail(const char *message) {
  gchar *reply = g_fixture_reply != NULL ? g_variant_print(g_fixture_reply, TRUE) : g_strdup("none");
  fprintf(stderr, "linux tray fixture failed at step %d after %.1f s: %s (item %s, reply %s)\n", g_fixture_step,
          (double)(g_get_monotonic_time() - g_fixture_started) / G_USEC_PER_SEC, message,
          g_fixture_item != NULL ? g_fixture_item : "none", reply);
  g_free(reply);
  exit(1);
}

static void tachyon_fixture_require(gboolean valid, const char *message) {
  if (!valid) tachyon_fixture_fail(message);
}

static gint64 tachyon_fixture_count(const char *event) {
  if (g_fixture_count == NULL) {
    gchar *root = tachyon_resource_root();
    gchar *bundle = g_path_get_dirname(root);
    gchar *path = g_build_filename(bundle, "bin", "libtachyoncompanion.so", NULL);
    GModule *module = g_module_open(path, G_MODULE_BIND_LAZY);
    if (module == NULL || !g_module_symbol(module, "tac_fixture_menu_count", (gpointer *)&g_fixture_count)) tachyon_fixture_fail("probe symbol is missing");
    g_free(path); g_free(bundle); g_free(root);
  }
  return g_fixture_count(event);
}

static void tachyon_fixture_watcher_call(GDBusConnection *connection, const gchar *sender, const gchar *path,
                                         const gchar *interface, const gchar *method, GVariant *parameters,
                                         GDBusMethodInvocation *invocation, gpointer data) {
  (void)connection;
  (void)sender;
  (void)path;
  (void)interface;
  (void)method;
  (void)data;
  g_free(g_fixture_item);
  g_variant_get(parameters, "(s)", &g_fixture_item);
  g_dbus_method_invocation_return_value(invocation, NULL);
}

static const GDBusInterfaceVTable TACHYON_FIXTURE_WATCHER_CALLS = {tachyon_fixture_watcher_call, NULL, NULL, {0}};

static void tachyon_fixture_watcher(void) {
  GError *error = NULL;
  gchar *address = g_dbus_address_get_for_bus_sync(G_BUS_TYPE_SESSION, NULL, &error);
  if (address == NULL) tachyon_fixture_fail(error->message);
  g_fixture_bus = g_dbus_connection_new_for_address_sync(
      address, G_DBUS_CONNECTION_FLAGS_AUTHENTICATION_CLIENT | G_DBUS_CONNECTION_FLAGS_MESSAGE_BUS_CONNECTION, NULL, NULL, &error);
  g_free(address);
  if (g_fixture_bus == NULL) tachyon_fixture_fail(error->message);
  GDBusNodeInfo *node = g_dbus_node_info_new_for_xml(TACHYON_FIXTURE_WATCHER, NULL);
  g_dbus_connection_register_object(g_fixture_bus, "/StatusNotifierWatcher", node->interfaces[0], &TACHYON_FIXTURE_WATCHER_CALLS, NULL, NULL, NULL);
  GVariant *owned = g_dbus_connection_call_sync(g_fixture_bus, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus",
                                                "RequestName", g_variant_new("(su)", "org.kde.StatusNotifierWatcher", 4u),
                                                G_VARIANT_TYPE("(u)"), G_DBUS_CALL_FLAGS_NONE, -1, NULL, &error);
  if (owned == NULL) tachyon_fixture_fail(error->message);
  g_variant_unref(owned);
  g_dbus_node_info_unref(node);
}

static void tachyon_fixture_answered(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  g_clear_pointer(&g_fixture_reply, g_variant_unref);
  g_fixture_reply = g_dbus_connection_call_finish(G_DBUS_CONNECTION(source), result, &error);
  if (g_fixture_reply == NULL) tachyon_fixture_fail(error->message);
  g_fixture_waiting = FALSE;
}

static void tachyon_fixture_call(const char *path, const char *interface, const char *method, GVariant *parameters) {
  g_fixture_waiting = TRUE;
  g_dbus_connection_call(g_fixture_bus, g_fixture_item, path, interface, method, parameters, NULL,
                         G_DBUS_CALL_FLAGS_NONE, 5000, NULL, tachyon_fixture_answered, NULL);
}

static void tachyon_fixture_property(const char *name) {
  tachyon_fixture_call(TACHYON_TRAY_ITEM_PATH, "org.freedesktop.DBus.Properties", "Get",
                       g_variant_new("(ss)", "org.kde.StatusNotifierItem", name));
}

static void tachyon_fixture_click(int id) {
  tachyon_fixture_call(TACHYON_TRAY_MENU_PATH, "com.canonical.dbusmenu", "Event",
                       g_variant_new("(isvu)", id, "clicked", g_variant_new_int32(0), 0u));
}

static int tachyon_fixture_item_id(const char *id) {
  for (int index = 0; index < TACHYON_MENU_ITEM_COUNT; index += 1) {
    if (g_strcmp0(TACHYON_MENU_ITEMS[index].id, id) == 0) return TACHYON_TRAY_FIRST_ID + index;
  }
  tachyon_fixture_fail("tray item is missing from the table");
  return -1;
}

static GVariant *tachyon_fixture_value(void) {
  GVariant *boxed = g_variant_get_child_value(g_fixture_reply, 0);
  GVariant *value = g_variant_get_variant(boxed);
  g_variant_unref(boxed);
  return value;
}

static void tachyon_fixture_check_pixmap(void) {
  GVariant *pixmaps = tachyon_fixture_value();
  tachyon_fixture_require(g_variant_n_children(pixmaps) == 1, "the icon has no pixmap");
  gint32 width = 0, height = 0;
  GVariant *pixels = NULL;
  g_variant_get_child(pixmaps, 0, "(ii@ay)", &width, &height, &pixels);
  tachyon_fixture_require(width == 192 && height == 192, "the pixmap does not have the PNG's size");
  tachyon_fixture_require(g_variant_n_children(pixels) == 192u * 192u * 4u, "the pixmap is not ARGB32");
  g_variant_unref(pixels);
  g_variant_unref(pixmaps);
}

static void tachyon_fixture_check_tooltip(void) {
  GVariant *tooltip = tachyon_fixture_value();
  const gchar *title = NULL;
  g_variant_get(tooltip, "(&s@a(iiay)&s&s)", NULL, NULL, &title, NULL);
  tachyon_fixture_require(g_strcmp0(title, "Tray tooltip é") == 0, "the tooltip is not the declared one");
  g_variant_unref(tooltip);
}

static void tachyon_fixture_check_layout(void) {
  GVariant *layout = g_variant_get_child_value(g_fixture_reply, 1);
  GVariant *children = g_variant_get_child_value(layout, 2);
  GString *labels = g_string_new(NULL);
  for (gsize index = 0; index < g_variant_n_children(children); index += 1) {
    GVariant *boxed = g_variant_get_child_value(children, index);
    GVariant *entry = g_variant_get_variant(boxed);
    GVariant *properties = g_variant_get_child_value(entry, 1);
    const gchar *label = NULL;
    const gchar *type = NULL;
    gboolean enabled = TRUE;
    g_variant_lookup(properties, "label", "&s", &label);
    g_variant_lookup(properties, "type", "&s", &type);
    g_variant_lookup(properties, "enabled", "b", &enabled);
    g_string_append_printf(labels, "%s%s|", label != NULL ? label : type, enabled ? "" : " (disabled)");
    g_variant_unref(properties);
    g_variant_unref(entry);
    g_variant_unref(boxed);
  }
  gboolean expected = g_strcmp0(labels->str, "Open Linux Host Check|separator|Dashboard|Refresh|separator|Quit Linux Host Check|") == 0;
  if (!expected) fprintf(stderr, "tray layout: %s\n", labels->str);
  g_string_free(labels, TRUE);
  g_variant_unref(children);
  g_variant_unref(layout);
  tachyon_fixture_require(expected, "the tray menu does not hold Open, the declared items and Quit");
}

static gboolean tachyon_fixture_window_visible(void) {
  return g_native_window != NULL && gtk_widget_get_visible(GTK_WIDGET(g_native_window));
}

static gboolean tachyon_fixture_tick(gpointer data) {
  (void)data;
  if (g_fixture_waiting) return G_SOURCE_CONTINUE;
  switch (g_fixture_step) {
    case 0: {
      const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
      if (g_strcmp0(uri, "tachyon://app/") != 0 || webkit_web_view_is_loading(g_view) || !g_tray_shown) return G_SOURCE_CONTINUE;
      tachyon_fixture_require(g_str_has_prefix(g_fixture_item, "org.kde.StatusNotifierItem-"), "the item registered an unexpected name");
      tachyon_fixture_property("IconPixmap");
      break;
    }
    case 1: tachyon_fixture_check_pixmap(); tachyon_fixture_property("ToolTip"); break;
    case 2:
      tachyon_fixture_check_tooltip();
      tachyon_fixture_call(TACHYON_TRAY_MENU_PATH, "com.canonical.dbusmenu", "GetLayout", g_variant_new("(ii@as)", 0, -1, g_variant_new_strv(NULL, 0)));
      break;
    case 3: tachyon_fixture_check_layout(); tachyon_fixture_click(tachyon_fixture_item_id("dashboard")); break;
    case 4: {
      const char *uri = webkit_web_view_get_uri(g_view);
      if (g_strcmp0(uri, "tachyon://app/help/") != 0 || webkit_web_view_is_loading(g_view)) return G_SOURCE_CONTINUE;
      tachyon_fixture_click(tachyon_fixture_item_id("tray-refresh"));
      break;
    }
    case 5:
      tachyon_fixture_require(tachyon_fixture_count("refreshed") == 1, "the tray action did not call the application service");
      gtk_window_close(g_native_window);
      break;
    case 6:
      tachyon_fixture_require(g_native_window != NULL && !tachyon_fixture_window_visible(), "closing the window did not hide it");
      tachyon_fixture_call(TACHYON_TRAY_ITEM_PATH, "org.kde.StatusNotifierItem", "Activate", g_variant_new("(ii)", 0, 0));
      break;
    case 7:
      if (!tachyon_fixture_window_visible()) return G_SOURCE_CONTINUE;
      tachyon_fixture_click(TACHYON_TRAY_QUIT_ID);
      break;
    default: return G_SOURCE_REMOVE;
  }
  g_fixture_step += 1;
  printf("tray step %d done after %.1f s\n", g_fixture_step, (double)(g_get_monotonic_time() - g_fixture_started) / G_USEC_PER_SEC);
  return G_SOURCE_CONTINUE;
}

static gboolean tachyon_fixture_deadline(gpointer data) {
  (void)data;
  tachyon_fixture_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_fixture_started = g_get_monotonic_time();
  tachyon_fixture_watcher();
  g_timeout_add(20, tachyon_fixture_tick, NULL);
  g_timeout_add_seconds(110, tachyon_fixture_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  tachyon_fixture_require(status == 0 && g_fixture_step == 8, "the host did not finish its run");
  tachyon_fixture_require(tachyon_fixture_count("stopped") == 1, "Quit did not stop the application service once");
  printf("linux tray passed: the StatusNotifierItem registered with the watcher, sent a 192x192 ARGB32 icon and the tooltip, its menu held Open, the declared items and Quit, a route item opened /help, an action item called the application service, closing hid the window, Activate showed it again and Quit stopped the app once\n");
  return 0;
}
