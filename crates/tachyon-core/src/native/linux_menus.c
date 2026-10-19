typedef bool (*TachyonMenuCall)(const char *);

typedef struct {
  const char *name;
  const char *label;
  const char *command;
  const char *accelerator;
} TachyonMenuStandard;

static TachyonMenuCall g_menu_perform = NULL;
static TachyonMenuCall g_menu_ready = NULL;
static GtkApplication *g_menu_application = NULL;
static void tachyon_tray_refresh(void);

__NATIVE_MENU_ITEMS__

static void tachyon_menus_bind(GModule *module) {
  g_module_symbol(module, "tac_native_menu_action", (gpointer *)&g_menu_perform);
  g_module_symbol(module, "tac_native_menu_ready", (gpointer *)&g_menu_ready);
}

static gboolean tachyon_menu_enabled(const TachyonMenuItem *item) {
  if (item->route != NULL) return tachyon_menu_window_open();
  if (g_menu_perform == NULL) return FALSE;
  if (item->event[0] == '\0') return TRUE;
  return g_menu_ready != NULL && tachyon_menu_page_shown() && g_menu_ready(item->event);
}

static void tachyon_menus_refresh(void) {
  if (g_menu_application == NULL) return;
  for (int index = 0; index < TACHYON_MENU_ITEM_COUNT; index += 1) {
    gchar *name = g_strdup_printf("menu-%d", index);
    GAction *action = g_action_map_lookup_action(G_ACTION_MAP(g_menu_application), name);
    if (action != NULL) g_simple_action_set_enabled(G_SIMPLE_ACTION(action), tachyon_menu_enabled(&TACHYON_MENU_ITEMS[index]));
    g_free(name);
  }
  tachyon_tray_refresh();
}

static void tachyon_menu_refuse(void) {
  GdkDisplay *display = gdk_display_get_default();
  if (display != NULL) gdk_display_beep(display);
}

static void tachyon_menu_activate(GSimpleAction *action, GVariant *parameter, gpointer data) {
  (void)action;
  (void)parameter;
  const TachyonMenuItem *item = data;
  if (!tachyon_menu_enabled(item)) tachyon_menu_refuse();
  else if (item->route != NULL) tachyon_menu_open_route(item->route);
  else if (!g_menu_perform(item->id)) tachyon_menu_refuse();
  tachyon_menus_refresh();
}

static void tachyon_menu_close_window(GSimpleAction *action, GVariant *parameter, gpointer data) {
  (void)action;
  (void)parameter;
  (void)data;
  GtkWindow *window = gtk_application_get_active_window(g_menu_application);
  if (window != NULL) gtk_window_close(window);
}

static void tachyon_menu_quit(GSimpleAction *action, GVariant *parameter, gpointer data) {
  (void)action;
  (void)parameter;
  (void)data;
  g_application_quit(G_APPLICATION(g_menu_application));
}

static void tachyon_menu_edit_activate(GSimpleAction *action, GVariant *parameter, gpointer data) {
  (void)action;
  (void)parameter;
  if (!tachyon_menu_edit(data)) tachyon_menu_refuse();
}

static void tachyon_menu_minimize(GSimpleAction *action, GVariant *parameter, gpointer data) {
  (void)action;
  (void)parameter;
  (void)data;
  GtkWindow *window = gtk_application_get_active_window(g_menu_application);
  if (window != NULL) gtk_window_minimize(window);
}

static void tachyon_menu_maximize(GSimpleAction *action, GVariant *parameter, gpointer data) {
  (void)action;
  (void)parameter;
  (void)data;
  GtkWindow *window = gtk_application_get_active_window(g_menu_application);
  if (window == NULL) return;
  if (gtk_window_is_maximized(window)) gtk_window_unmaximize(window);
  else gtk_window_maximize(window);
}

static void tachyon_menu_action(const char *name, const char *accelerator, GCallback activate, gpointer data) {
  GSimpleAction *action = g_simple_action_new(name, NULL);
  g_signal_connect(action, "activate", activate, data);
  g_action_map_add_action(G_ACTION_MAP(g_menu_application), G_ACTION(action));
  g_object_unref(action);
  if (accelerator == NULL) return;
  gchar *detailed = g_strdup_printf("app.%s", name);
  const gchar *accelerators[] = {accelerator, NULL};
  gtk_application_set_accels_for_action(g_menu_application, detailed, accelerators);
  g_free(detailed);
}

static void tachyon_menu_item(GMenu *menu, int index) {
  const TachyonMenuItem *item = &TACHYON_MENU_ITEMS[index];
  gchar *name = g_strdup_printf("menu-%d", index);
  gchar *accelerator = item->key == 0 ? NULL : g_strdup_printf("%s%s%s%c",
      (item->modifiers & TACHYON_MENU_PRIMARY) ? "<Primary>" : "",
      (item->modifiers & TACHYON_MENU_ALT) ? "<Alt>" : "",
      (item->modifiers & TACHYON_MENU_SHIFT) ? "<Shift>" : "", item->key);
  tachyon_menu_action(name, accelerator, G_CALLBACK(tachyon_menu_activate), (gpointer)item);
  gchar *detailed = g_strdup_printf("app.%s", name);
  g_menu_append(menu, item->label, detailed);
  g_free(detailed);
  g_free(accelerator);
  g_free(name);
}

static void tachyon_menu_group(GMenu *bar, const char *title, GMenu *items) {
  if (g_menu_model_get_n_items(G_MENU_MODEL(items)) > 0) g_menu_append_submenu(bar, title, G_MENU_MODEL(items));
}

static void tachyon_menu_standard(GMenu *menu, const TachyonMenuStandard *item) {
  tachyon_menu_action(item->name, item->accelerator, G_CALLBACK(tachyon_menu_edit_activate), (gpointer)item->command);
  gchar *detailed = g_strdup_printf("app.%s", item->name);
  g_menu_append(menu, item->label, detailed);
  g_free(detailed);
}

static GMenu *tachyon_menus_edit(void) {
  static const TachyonMenuStandard history[] = {
    {"edit-undo", "_Undo", "Undo", "<Primary>z"},
    {"edit-redo", "_Redo", "Redo", "<Primary><Shift>z"},
  };
  static const TachyonMenuStandard clipboard[] = {
    {"edit-cut", "Cu_t", "Cut", "<Primary>x"},
    {"edit-copy", "_Copy", "Copy", "<Primary>c"},
    {"edit-paste", "_Paste", "Paste", "<Primary>v"},
  };
  static const TachyonMenuStandard selection[] = {
    {"edit-select-all", "Select _All", "SelectAll", "<Primary>a"},
  };
  GMenu *sections[] = {g_menu_new(), g_menu_new(), g_menu_new()};
  for (gsize index = 0; index < G_N_ELEMENTS(history); index += 1) tachyon_menu_standard(sections[0], &history[index]);
  for (gsize index = 0; index < G_N_ELEMENTS(clipboard); index += 1) tachyon_menu_standard(sections[1], &clipboard[index]);
  for (gsize index = 0; index < G_N_ELEMENTS(selection); index += 1) tachyon_menu_standard(sections[2], &selection[index]);
  GMenu *edit = g_menu_new();
  for (gsize section = 0; section < G_N_ELEMENTS(sections); section += 1) {
    g_menu_append_section(edit, NULL, G_MENU_MODEL(sections[section]));
    g_object_unref(sections[section]);
  }
  return edit;
}

static GMenu *tachyon_menus_window(void) {
  tachyon_menu_action("minimize", NULL, G_CALLBACK(tachyon_menu_minimize), NULL);
  tachyon_menu_action("maximize", NULL, G_CALLBACK(tachyon_menu_maximize), NULL);
  GMenu *window = g_menu_new();
  g_menu_append(window, "Mi_nimize", "app.minimize");
  g_menu_append(window, "Ma_ximize", "app.maximize");
  return window;
}

static gchar *tachyon_menu_spoken_text(const char *text) {
  GString *spoken = g_string_new(NULL);
  for (const char *cursor = text; *cursor != '\0'; cursor += 1) {
    if (*cursor == '_' && cursor[1] != '\0') cursor += 1;
    g_string_append_c(spoken, *cursor);
  }
  return g_string_free(spoken, FALSE);
}

static void tachyon_menu_name_item(GtkWidget *widget) {
  if (gtk_accessible_get_accessible_role(GTK_ACCESSIBLE(widget)) != GTK_ACCESSIBLE_ROLE_MENU_ITEM) return;
  if (g_object_class_find_property(G_OBJECT_GET_CLASS(widget), "text") == NULL) return;
  gchar *text = NULL;
  g_object_get(widget, "text", &text, NULL);
  if (text != NULL && text[0] != '\0') {
    gchar *spoken = tachyon_menu_spoken_text(text);
    gtk_accessible_reset_relation(GTK_ACCESSIBLE(widget), GTK_ACCESSIBLE_RELATION_LABELLED_BY);
    gtk_accessible_update_property(GTK_ACCESSIBLE(widget), GTK_ACCESSIBLE_PROPERTY_LABEL, spoken, -1);
    g_free(spoken);
  }
  g_free(text);
}

static void tachyon_menus_name_tree(GtkWidget *widget);

static void tachyon_menus_mapped(GtkWidget *widget, gpointer data) {
  (void)data;
  tachyon_menus_name_tree(widget);
}

static void tachyon_menus_name_tree(GtkWidget *widget) {
  if (GTK_IS_POPOVER(widget)) {
    g_signal_handlers_disconnect_by_func(widget, G_CALLBACK(tachyon_menus_mapped), NULL);
    g_signal_connect(widget, "map", G_CALLBACK(tachyon_menus_mapped), NULL);
  }
  tachyon_menu_name_item(widget);
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child != NULL; child = gtk_widget_get_next_sibling(child)) {
    tachyon_menus_name_tree(child);
  }
}

static void tachyon_menus_install(GtkApplication *application) {
  g_menu_application = application;
  GMenu *groups[] = {g_menu_new(), g_menu_new(), g_menu_new()};
  for (int index = 0; index < TACHYON_MENU_ITEM_COUNT && index < TACHYON_MENU_LIMIT; index += 1) {
    if (TACHYON_MENU_ITEMS[index].group != TACHYON_MENU_TRAY) tachyon_menu_item(groups[TACHYON_MENU_ITEMS[index].group], index);
  }
  tachyon_menu_action("close-window", "<Primary>w", G_CALLBACK(tachyon_menu_close_window), NULL);
  tachyon_menu_action("quit", "<Primary>q", G_CALLBACK(tachyon_menu_quit), NULL);
  GMenu *closing = g_menu_new();
  g_menu_append(closing, "Close Window", "app.close-window");
  g_menu_append(closing, "Quit", "app.quit");
  GMenu *file = g_menu_new();
  g_menu_append_section(file, NULL, G_MENU_MODEL(groups[TACHYON_MENU_FILE]));
  g_menu_append_section(file, NULL, G_MENU_MODEL(closing));
  GMenu *edit = tachyon_menus_edit();
  GMenu *window = tachyon_menus_window();
  GMenu *bar = g_menu_new();
  g_menu_append_submenu(bar, "_File", G_MENU_MODEL(file));
  g_menu_append_submenu(bar, "_Edit", G_MENU_MODEL(edit));
  tachyon_menu_group(bar, "_View", groups[TACHYON_MENU_VIEW]);
  g_menu_append_submenu(bar, "_Window", G_MENU_MODEL(window));
  tachyon_menu_group(bar, "_Help", groups[TACHYON_MENU_HELP]);
  gtk_application_set_menubar(application, G_MENU_MODEL(bar));
  g_object_unref(bar);
  g_object_unref(window);
  g_object_unref(edit);
  g_object_unref(file);
  g_object_unref(closing);
  for (gsize group = 0; group < G_N_ELEMENTS(groups); group += 1) g_object_unref(groups[group]);
  tachyon_menus_refresh();
}
