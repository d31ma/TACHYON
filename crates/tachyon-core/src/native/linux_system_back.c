static void tachyon_system_back_go(gboolean forward) {
  if (!tachyon_trusted_view(g_view)) return;
  if (forward ? !webkit_web_view_can_go_forward(g_view) : !webkit_web_view_can_go_back(g_view)) return;
  tachyon_record(forward ? "navigation.forward" : "navigation.back");
  if (forward) webkit_web_view_go_forward(g_view);
  else webkit_web_view_go_back(g_view);
}

static gboolean tachyon_system_back_key(GtkWidget *widget, GVariant *arguments, gpointer forward) {
  (void)widget;
  (void)arguments;
  tachyon_system_back_go(GPOINTER_TO_INT(forward));
  return TRUE;
}

static gboolean tachyon_system_back_button(GtkEventControllerLegacy *controller, GdkEvent *event, gpointer data) {
  (void)controller;
  (void)data;
  GdkEventType type = gdk_event_get_event_type(event);
  if (type != GDK_BUTTON_PRESS && type != GDK_BUTTON_RELEASE) return FALSE;
  guint button = gdk_button_event_get_button(event);
  if (button != 8 && button != 9) return FALSE;
  if (type == GDK_BUTTON_PRESS) tachyon_system_back_go(button == 9);
  return TRUE;
}

static void tachyon_system_back_shortcut(GtkShortcutController *keys, guint key, GdkModifierType modifiers, gboolean forward) {
  gtk_shortcut_controller_add_shortcut(keys, gtk_shortcut_new(
      gtk_keyval_trigger_new(key, modifiers),
      gtk_callback_action_new(tachyon_system_back_key, GINT_TO_POINTER(forward), NULL)));
}

static void tachyon_system_back_install(GtkWindow *window) {
  GtkEventController *keys = gtk_shortcut_controller_new();
  gtk_event_controller_set_propagation_phase(keys, GTK_PHASE_CAPTURE);
  tachyon_system_back_shortcut(GTK_SHORTCUT_CONTROLLER(keys), GDK_KEY_Left, GDK_ALT_MASK, FALSE);
  tachyon_system_back_shortcut(GTK_SHORTCUT_CONTROLLER(keys), GDK_KEY_Right, GDK_ALT_MASK, TRUE);
  tachyon_system_back_shortcut(GTK_SHORTCUT_CONTROLLER(keys), GDK_KEY_Back, 0, FALSE);
  tachyon_system_back_shortcut(GTK_SHORTCUT_CONTROLLER(keys), GDK_KEY_Forward, 0, TRUE);
  gtk_widget_add_controller(GTK_WIDGET(window), keys);
  GtkEventController *buttons = gtk_event_controller_legacy_new();
  gtk_event_controller_set_propagation_phase(buttons, GTK_PHASE_CAPTURE);
  g_signal_connect(buttons, "event", G_CALLBACK(tachyon_system_back_button), NULL);
  gtk_widget_add_controller(GTK_WIDGET(window), buttons);
}
