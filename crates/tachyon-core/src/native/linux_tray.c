__NATIVE_TRAY_SETTINGS__

#if TACHYON_TRAY
#define TACHYON_TRAY_ITEM_PATH "/StatusNotifierItem"
#define TACHYON_TRAY_MENU_PATH "/StatusNotifierItem/Menu"
#define TACHYON_TRAY_WATCHER "org.kde.StatusNotifierWatcher"
#define TACHYON_TRAY_OPEN_ID 1
#define TACHYON_TRAY_QUIT_ID 2
#define TACHYON_TRAY_TOP_SEPARATOR_ID 3
#define TACHYON_TRAY_BOTTOM_SEPARATOR_ID 4
#define TACHYON_TRAY_FIRST_ID 100

static const gchar TACHYON_TRAY_INTERFACES[] =
    "<node>"
    "<interface name='org.kde.StatusNotifierItem'>"
    "<property name='Category' type='s' access='read'/>"
    "<property name='Id' type='s' access='read'/>"
    "<property name='Title' type='s' access='read'/>"
    "<property name='Status' type='s' access='read'/>"
    "<property name='IconName' type='s' access='read'/>"
    "<property name='IconPixmap' type='a(iiay)' access='read'/>"
    "<property name='ToolTip' type='(sa(iiay)ss)' access='read'/>"
    "<property name='ItemIsMenu' type='b' access='read'/>"
    "<property name='Menu' type='o' access='read'/>"
    "<method name='Activate'><arg type='i' direction='in'/><arg type='i' direction='in'/></method>"
    "<method name='SecondaryActivate'><arg type='i' direction='in'/><arg type='i' direction='in'/></method>"
    "<method name='ContextMenu'><arg type='i' direction='in'/><arg type='i' direction='in'/></method>"
    "<method name='Scroll'><arg type='i' direction='in'/><arg type='s' direction='in'/></method>"
    "<signal name='NewIcon'/>"
    "<signal name='NewToolTip'/>"
    "<signal name='NewStatus'><arg type='s'/></signal>"
    "</interface>"
    "<interface name='com.canonical.dbusmenu'>"
    "<property name='Version' type='u' access='read'/>"
    "<property name='TextDirection' type='s' access='read'/>"
    "<property name='Status' type='s' access='read'/>"
    "<property name='IconThemePath' type='as' access='read'/>"
    "<method name='GetLayout'><arg type='i' direction='in'/><arg type='i' direction='in'/><arg type='as' direction='in'/>"
    "<arg type='u' direction='out'/><arg type='(ia{sv}av)' direction='out'/></method>"
    "<method name='GetGroupProperties'><arg type='ai' direction='in'/><arg type='as' direction='in'/><arg type='a(ia{sv})' direction='out'/></method>"
    "<method name='GetProperty'><arg type='i' direction='in'/><arg type='s' direction='in'/><arg type='v' direction='out'/></method>"
    "<method name='Event'><arg type='i' direction='in'/><arg type='s' direction='in'/><arg type='v' direction='in'/><arg type='u' direction='in'/></method>"
    "<method name='EventGroup'><arg type='a(isvu)' direction='in'/><arg type='ai' direction='out'/></method>"
    "<method name='AboutToShow'><arg type='i' direction='in'/><arg type='b' direction='out'/></method>"
    "<method name='AboutToShowGroup'><arg type='ai' direction='in'/><arg type='ai' direction='out'/><arg type='ai' direction='out'/></method>"
    "<signal name='ItemsPropertiesUpdated'><arg type='a(ia{sv})'/><arg type='a(ias)'/></signal>"
    "<signal name='LayoutUpdated'><arg type='u'/><arg type='i'/></signal>"
    "<signal name='ItemActivationRequested'><arg type='i'/><arg type='u'/></signal>"
    "</interface>"
    "</node>";

static GDBusConnection *g_tray_bus = NULL;
static GDBusNodeInfo *g_tray_node = NULL;
static GVariant *g_tray_pixmap = NULL;
static gchar *g_tray_service = NULL;
static guint g_tray_objects[2] = {0, 0};
static guint g_tray_owner = 0;
static guint g_tray_watch = 0;
static guint g_tray_revision = 1;
static guint64 g_tray_enabled = 0;
static gboolean g_tray_shown = FALSE;
static gboolean g_tray_quitting = FALSE;

static gchar *tachyon_resource_root(void);

static GVariant *tachyon_tray_pixmap(void) {
  gchar *resources = tachyon_resource_root();
  gchar *path = g_build_filename(resources, "WebBundle", TACHYON_TRAY_ICON, NULL);
  GFile *file = g_file_new_for_path(path);
  GdkTexture *texture = gdk_texture_new_from_file(file, NULL);
  GVariantBuilder pixmaps;
  g_variant_builder_init(&pixmaps, G_VARIANT_TYPE("a(iiay)"));
  if (texture != NULL) {
    int width = gdk_texture_get_width(texture);
    int height = gdk_texture_get_height(texture);
    GdkTextureDownloader *downloader = gdk_texture_downloader_new(texture);
    gdk_texture_downloader_set_format(downloader, GDK_MEMORY_A8R8G8B8);
    gsize stride = 0;
    GBytes *pixels = gdk_texture_downloader_download_bytes(downloader, &stride);
    if (stride == (gsize)width * 4) {
      GVariant *bytes = g_variant_new_from_bytes(G_VARIANT_TYPE("ay"), pixels, TRUE);
      g_variant_builder_add(&pixmaps, "(ii@ay)", width, height, bytes);
    }
    g_bytes_unref(pixels);
    gdk_texture_downloader_free(downloader);
    g_object_unref(texture);
  }
  g_object_unref(file);
  g_free(path);
  g_free(resources);
  return g_variant_ref_sink(g_variant_builder_end(&pixmaps));
}

static guint64 tachyon_tray_enabled_items(void) {
  guint64 enabled = 0;
  for (int index = 0, bit = 0; index < TACHYON_MENU_ITEM_COUNT && index < TACHYON_MENU_LIMIT && bit < 64; index += 1) {
    if (TACHYON_MENU_ITEMS[index].group != TACHYON_MENU_TRAY) continue;
    if (tachyon_menu_enabled(&TACHYON_MENU_ITEMS[index])) enabled |= G_GUINT64_CONSTANT(1) << bit;
    bit += 1;
  }
  return enabled;
}

static gboolean tachyon_tray_has_items(void) {
  for (int index = 0; index < TACHYON_MENU_ITEM_COUNT && index < TACHYON_MENU_LIMIT; index += 1) {
    if (TACHYON_MENU_ITEMS[index].group == TACHYON_MENU_TRAY) return TRUE;
  }
  return FALSE;
}

static GVariant *tachyon_tray_entry(int id, const char *label, gboolean enabled, gboolean separator) {
  GVariantBuilder properties;
  g_variant_builder_init(&properties, G_VARIANT_TYPE("a{sv}"));
  if (separator) g_variant_builder_add(&properties, "{sv}", "type", g_variant_new_string("separator"));
  else g_variant_builder_add(&properties, "{sv}", "label", g_variant_new_string(label));
  g_variant_builder_add(&properties, "{sv}", "enabled", g_variant_new_boolean(enabled));
  g_variant_builder_add(&properties, "{sv}", "visible", g_variant_new_boolean(TRUE));
  return g_variant_new("(i@a{sv}@av)", id, g_variant_builder_end(&properties), g_variant_new_array(G_VARIANT_TYPE_VARIANT, NULL, 0));
}

static GVariant *tachyon_tray_layout(void) {
  GVariantBuilder children;
  g_variant_builder_init(&children, G_VARIANT_TYPE("av"));
  g_variant_builder_add(&children, "v", tachyon_tray_entry(TACHYON_TRAY_OPEN_ID, TACHYON_TRAY_OPEN_LABEL, TRUE, FALSE));
  if (tachyon_tray_has_items()) g_variant_builder_add(&children, "v", tachyon_tray_entry(TACHYON_TRAY_TOP_SEPARATOR_ID, NULL, TRUE, TRUE));
  for (int index = 0; index < TACHYON_MENU_ITEM_COUNT && index < TACHYON_MENU_LIMIT; index += 1) {
    const TachyonMenuItem *item = &TACHYON_MENU_ITEMS[index];
    if (item->group != TACHYON_MENU_TRAY) continue;
    g_variant_builder_add(&children, "v", tachyon_tray_entry(TACHYON_TRAY_FIRST_ID + index, item->label, tachyon_menu_enabled(item), FALSE));
  }
  g_variant_builder_add(&children, "v", tachyon_tray_entry(TACHYON_TRAY_BOTTOM_SEPARATOR_ID, NULL, TRUE, TRUE));
  g_variant_builder_add(&children, "v", tachyon_tray_entry(TACHYON_TRAY_QUIT_ID, TACHYON_TRAY_QUIT_LABEL, TRUE, FALSE));
  GVariantBuilder root;
  g_variant_builder_init(&root, G_VARIANT_TYPE("a{sv}"));
  g_variant_builder_add(&root, "{sv}", "children-display", g_variant_new_string("submenu"));
  return g_variant_new("(i@a{sv}@av)", 0, g_variant_builder_end(&root), g_variant_builder_end(&children));
}

static void tachyon_tray_show_window(void) {
  if (g_native_window == NULL) {
    if (g_menu_application != NULL) g_application_activate(G_APPLICATION(g_menu_application));
    return;
  }
  gtk_widget_set_visible(GTK_WIDGET(g_native_window), TRUE);
  gtk_window_present(g_native_window);
}

static void tachyon_tray_choose(int id) {
  if (id == TACHYON_TRAY_OPEN_ID) { tachyon_tray_show_window(); return; }
  if (id == TACHYON_TRAY_QUIT_ID) {
    g_tray_quitting = TRUE;
    if (g_menu_application != NULL) g_application_quit(G_APPLICATION(g_menu_application));
    return;
  }
  int index = id - TACHYON_TRAY_FIRST_ID;
  if (index < 0 || index >= TACHYON_MENU_ITEM_COUNT || TACHYON_MENU_ITEMS[index].group != TACHYON_MENU_TRAY) return;
  const TachyonMenuItem *item = &TACHYON_MENU_ITEMS[index];
  if (!tachyon_menu_enabled(item)) tachyon_menu_refuse();
  else if (item->route != NULL) { tachyon_tray_show_window(); tachyon_menu_open_route(item->route); }
  else if (!g_menu_perform(item->id)) tachyon_menu_refuse();
  tachyon_menus_refresh();
}

static void tachyon_tray_refresh(void) {
  guint64 enabled = tachyon_tray_enabled_items();
  if (g_tray_bus == NULL || enabled == g_tray_enabled) return;
  g_tray_enabled = enabled;
  g_tray_revision += 1;
  g_dbus_connection_emit_signal(g_tray_bus, NULL, TACHYON_TRAY_MENU_PATH, "com.canonical.dbusmenu", "LayoutUpdated",
                                g_variant_new("(ui)", g_tray_revision, 0), NULL);
}

static GVariant *tachyon_tray_property(const gchar *name) {
  if (g_strcmp0(name, "Category") == 0) return g_variant_new_string("ApplicationStatus");
  if (g_strcmp0(name, "Id") == 0) return g_variant_new_string(TACHYON_BUNDLE_ID);
  if (g_strcmp0(name, "Title") == 0) return g_variant_new_string(TACHYON_APP_NAME);
  if (g_strcmp0(name, "Status") == 0) return g_variant_new_string("Active");
  if (g_strcmp0(name, "IconName") == 0) return g_variant_new_string("");
  if (g_strcmp0(name, "IconPixmap") == 0) return g_variant_ref(g_tray_pixmap);
  if (g_strcmp0(name, "ToolTip") == 0) return g_variant_new("(s@a(iiay)ss)", "", g_tray_pixmap, TACHYON_TRAY_TOOLTIP, "");
  if (g_strcmp0(name, "ItemIsMenu") == 0) return g_variant_new_boolean(FALSE);
  if (g_strcmp0(name, "Menu") == 0) return g_variant_new_object_path(TACHYON_TRAY_MENU_PATH);
  return NULL;
}

static GVariant *tachyon_tray_menu_property(const gchar *name) {
  if (g_strcmp0(name, "Version") == 0) return g_variant_new_uint32(3);
  if (g_strcmp0(name, "TextDirection") == 0) return g_variant_new_string("ltr");
  if (g_strcmp0(name, "Status") == 0) return g_variant_new_string("normal");
  if (g_strcmp0(name, "IconThemePath") == 0) return g_variant_new_strv(NULL, 0);
  return NULL;
}

static GVariant *tachyon_tray_get_property(GDBusConnection *connection, const gchar *sender, const gchar *path,
                                           const gchar *interface, const gchar *name, GError **error, gpointer data) {
  (void)connection;
  (void)sender;
  (void)path;
  (void)data;
  GVariant *value = g_strcmp0(interface, "com.canonical.dbusmenu") == 0 ? tachyon_tray_menu_property(name) : tachyon_tray_property(name);
  if (value == NULL) g_set_error(error, G_DBUS_ERROR, G_DBUS_ERROR_UNKNOWN_PROPERTY, "Unknown tray property.");
  return value;
}

static void tachyon_tray_item_call(GDBusConnection *connection, const gchar *sender, const gchar *path,
                                   const gchar *interface, const gchar *method, GVariant *parameters,
                                   GDBusMethodInvocation *invocation, gpointer data) {
  (void)connection;
  (void)sender;
  (void)path;
  (void)interface;
  (void)parameters;
  (void)data;
  if (g_strcmp0(method, "Activate") == 0) tachyon_tray_show_window();
  g_dbus_method_invocation_return_value(invocation, NULL);
}

static GVariant *tachyon_tray_group_properties(GVariant *ids) {
  GVariantBuilder groups;
  g_variant_builder_init(&groups, G_VARIANT_TYPE("a(ia{sv})"));
  GVariant *layout = g_variant_ref_sink(tachyon_tray_layout());
  GVariant *children = g_variant_get_child_value(layout, 2);
  for (gsize child = 0; child < g_variant_n_children(children); child += 1) {
    GVariant *boxed = g_variant_get_child_value(children, child);
    GVariant *entry = g_variant_get_variant(boxed);
    gint32 id = 0;
    GVariant *properties = NULL;
    g_variant_get(entry, "(i@a{sv}@av)", &id, &properties, NULL);
    gboolean wanted = g_variant_n_children(ids) == 0;
    for (gsize index = 0; index < g_variant_n_children(ids) && !wanted; index += 1) {
      gint32 requested = 0;
      g_variant_get_child(ids, index, "i", &requested);
      wanted = requested == id;
    }
    if (wanted) g_variant_builder_add(&groups, "(i@a{sv})", id, properties);
    g_variant_unref(properties);
    g_variant_unref(entry);
    g_variant_unref(boxed);
  }
  g_variant_unref(children);
  g_variant_unref(layout);
  return g_variant_builder_end(&groups);
}

static void tachyon_tray_menu_call(GDBusConnection *connection, const gchar *sender, const gchar *path,
                                   const gchar *interface, const gchar *method, GVariant *parameters,
                                   GDBusMethodInvocation *invocation, gpointer data) {
  (void)connection;
  (void)sender;
  (void)path;
  (void)interface;
  (void)data;
  if (g_strcmp0(method, "GetLayout") == 0) {
    g_tray_enabled = tachyon_tray_enabled_items();
    g_dbus_method_invocation_return_value(invocation, g_variant_new("(u@(ia{sv}av))", g_tray_revision, tachyon_tray_layout()));
  } else if (g_strcmp0(method, "GetGroupProperties") == 0) {
    GVariant *ids = g_variant_get_child_value(parameters, 0);
    g_dbus_method_invocation_return_value(invocation, g_variant_new("(@a(ia{sv}))", tachyon_tray_group_properties(ids)));
    g_variant_unref(ids);
  } else if (g_strcmp0(method, "GetProperty") == 0) {
    g_dbus_method_invocation_return_error_literal(invocation, G_DBUS_ERROR, G_DBUS_ERROR_NOT_SUPPORTED, "Read the layout instead.");
  } else if (g_strcmp0(method, "Event") == 0) {
    gint32 id = 0;
    const gchar *event = NULL;
    g_variant_get(parameters, "(i&svu)", &id, &event, NULL, NULL);
    g_dbus_method_invocation_return_value(invocation, NULL);
    if (g_strcmp0(event, "clicked") == 0) tachyon_tray_choose(id);
  } else if (g_strcmp0(method, "EventGroup") == 0) {
    GVariantIter *events = NULL;
    g_variant_get(parameters, "(a(isvu))", &events);
    gint32 id = 0;
    const gchar *event = NULL;
    GVariant *chosen = NULL;
    while (g_variant_iter_next(events, "(i&s@vu)", &id, &event, &chosen, NULL)) {
      if (g_strcmp0(event, "clicked") == 0) tachyon_tray_choose(id);
      g_variant_unref(chosen);
    }
    g_variant_iter_free(events);
    g_dbus_method_invocation_return_value(invocation, g_variant_new("(@ai)", g_variant_new_array(G_VARIANT_TYPE_INT32, NULL, 0)));
  } else if (g_strcmp0(method, "AboutToShow") == 0) {
    g_dbus_method_invocation_return_value(invocation, g_variant_new("(b)", tachyon_tray_enabled_items() != g_tray_enabled));
  } else if (g_strcmp0(method, "AboutToShowGroup") == 0) {
    GVariant *none = g_variant_new_array(G_VARIANT_TYPE_INT32, NULL, 0);
    g_dbus_method_invocation_return_value(invocation, g_variant_new("(@ai@ai)", none, g_variant_new_array(G_VARIANT_TYPE_INT32, NULL, 0)));
  } else {
    g_dbus_method_invocation_return_error_literal(invocation, G_DBUS_ERROR, G_DBUS_ERROR_UNKNOWN_METHOD, "Unknown tray menu method.");
  }
}

static const GDBusInterfaceVTable TACHYON_TRAY_ITEM_CALLS = {tachyon_tray_item_call, tachyon_tray_get_property, NULL, {0}};
static const GDBusInterfaceVTable TACHYON_TRAY_MENU_CALLS = {tachyon_tray_menu_call, tachyon_tray_get_property, NULL, {0}};

static void tachyon_tray_registered(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  GVariant *answer = g_dbus_connection_call_finish(G_DBUS_CONNECTION(source), result, &error);
  g_tray_shown = answer != NULL;
  if (answer != NULL) { g_variant_unref(answer); tachyon_record("tray.shown"); }
  else tachyon_record("tray.refused");
  g_clear_error(&error);
}

static void tachyon_tray_watcher_appeared(GDBusConnection *connection, const gchar *name, const gchar *owner, gpointer data) {
  (void)name;
  (void)owner;
  (void)data;
  g_dbus_connection_call(connection, TACHYON_TRAY_WATCHER, "/StatusNotifierWatcher", TACHYON_TRAY_WATCHER,
                         "RegisterStatusNotifierItem", g_variant_new("(s)", g_tray_service), NULL,
                         G_DBUS_CALL_FLAGS_NONE, -1, NULL, tachyon_tray_registered, NULL);
}

static void tachyon_tray_watcher_vanished(GDBusConnection *connection, const gchar *name, gpointer data) {
  (void)connection;
  (void)name;
  (void)data;
  g_tray_shown = FALSE;
  if (g_native_window != NULL) gtk_widget_set_visible(GTK_WIDGET(g_native_window), TRUE);
}

static void tachyon_tray_named(GDBusConnection *connection, const gchar *name, gpointer data) {
  (void)name;
  (void)data;
  g_tray_watch = g_bus_watch_name_on_connection(connection, TACHYON_TRAY_WATCHER, G_BUS_NAME_WATCHER_FLAGS_NONE,
                                                tachyon_tray_watcher_appeared, tachyon_tray_watcher_vanished, NULL, NULL);
}

static void tachyon_tray_install(void) {
  if (g_menu_application == NULL) return;
  g_tray_bus = g_application_get_dbus_connection(G_APPLICATION(g_menu_application));
  if (g_tray_bus == NULL) { tachyon_record("tray.unavailable"); return; }
  g_tray_node = g_dbus_node_info_new_for_xml(TACHYON_TRAY_INTERFACES, NULL);
  if (g_tray_node == NULL) { g_tray_bus = NULL; return; }
  g_tray_pixmap = tachyon_tray_pixmap();
  tachyon_record(g_variant_n_children(g_tray_pixmap) == 1 ? "tray.icon.loaded" : "tray.icon.missing");
  g_tray_enabled = tachyon_tray_enabled_items();
  g_tray_objects[0] = g_dbus_connection_register_object(g_tray_bus, TACHYON_TRAY_ITEM_PATH, g_tray_node->interfaces[0],
                                                        &TACHYON_TRAY_ITEM_CALLS, NULL, NULL, NULL);
  g_tray_objects[1] = g_dbus_connection_register_object(g_tray_bus, TACHYON_TRAY_MENU_PATH, g_tray_node->interfaces[1],
                                                        &TACHYON_TRAY_MENU_CALLS, NULL, NULL, NULL);
  g_tray_service = g_strdup_printf("org.kde.StatusNotifierItem-%d-1", (int)getpid());
  g_tray_owner = g_bus_own_name_on_connection(g_tray_bus, g_tray_service, G_BUS_NAME_OWNER_FLAGS_NONE,
                                              tachyon_tray_named, NULL, NULL, NULL);
}

static void tachyon_tray_remove(void) {
  if (g_tray_watch != 0) g_bus_unwatch_name(g_tray_watch);
  if (g_tray_owner != 0) g_bus_unown_name(g_tray_owner);
  for (gsize index = 0; index < G_N_ELEMENTS(g_tray_objects); index += 1) {
    if (g_tray_objects[index] != 0 && g_tray_bus != NULL) g_dbus_connection_unregister_object(g_tray_bus, g_tray_objects[index]);
    g_tray_objects[index] = 0;
  }
  g_tray_watch = 0;
  g_tray_owner = 0;
  g_tray_shown = FALSE;
  g_tray_bus = NULL;
  g_clear_pointer(&g_tray_pixmap, g_variant_unref);
  g_clear_pointer(&g_tray_node, g_dbus_node_info_unref);
  g_clear_pointer(&g_tray_service, g_free);
}

static gboolean tachyon_tray_close_request(GtkWindow *window, gpointer data) {
  (void)data;
#if TACHYON_TRAY_CLOSE_HIDES
  if (!g_tray_shown || g_tray_quitting) return FALSE;
  gtk_widget_set_visible(GTK_WIDGET(window), FALSE);
  tachyon_record("window.hidden");
  return TRUE;
#else
  (void)window;
  return FALSE;
#endif
}
#else
static void tachyon_tray_install(void) {}

static void tachyon_tray_remove(void) {}

static void tachyon_tray_refresh(void) {}

static gboolean tachyon_tray_close_request(GtkWindow *window, gpointer data) {
  (void)window;
  (void)data;
  return FALSE;
}
#endif
