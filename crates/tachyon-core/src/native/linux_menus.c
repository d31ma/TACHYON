typedef bool (*TachyonMenuCall)(const char *);

static TachyonMenuCall g_menu_perform = NULL;
static TachyonMenuCall g_menu_ready = NULL;
static GtkApplication *g_menu_application = NULL;

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

static void tachyon_menus_install(GtkApplication *application) {
  if (TACHYON_MENU_ITEM_COUNT == 0) return;
  g_menu_application = application;
  GMenu *groups[] = {g_menu_new(), g_menu_new(), g_menu_new()};
  for (int index = 0; index < TACHYON_MENU_ITEM_COUNT && index < TACHYON_MENU_LIMIT; index += 1) {
    tachyon_menu_item(groups[TACHYON_MENU_ITEMS[index].group], index);
  }
  tachyon_menu_action("close-window", "<Primary>w", G_CALLBACK(tachyon_menu_close_window), NULL);
  tachyon_menu_action("quit", "<Primary>q", G_CALLBACK(tachyon_menu_quit), NULL);
  GMenu *window = g_menu_new();
  g_menu_append(window, "Close Window", "app.close-window");
  g_menu_append(window, "Quit", "app.quit");
  GMenu *file = g_menu_new();
  g_menu_append_section(file, NULL, G_MENU_MODEL(groups[TACHYON_MENU_FILE]));
  g_menu_append_section(file, NULL, G_MENU_MODEL(window));
  GMenu *bar = g_menu_new();
  g_menu_append_submenu(bar, "_File", G_MENU_MODEL(file));
  tachyon_menu_group(bar, "_View", groups[TACHYON_MENU_VIEW]);
  tachyon_menu_group(bar, "_Help", groups[TACHYON_MENU_HELP]);
  gtk_application_set_menubar(application, G_MENU_MODEL(bar));
  g_object_unref(bar);
  g_object_unref(file);
  g_object_unref(window);
  for (gsize group = 0; group < G_N_ELEMENTS(groups); group += 1) g_object_unref(groups[group]);
  tachyon_menus_refresh();
}
