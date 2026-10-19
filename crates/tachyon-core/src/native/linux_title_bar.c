#include <math.h>

static const gboolean TACHYON_TITLE_BAR_INTEGRATED = __TITLE_BAR_INTEGRATED__;
static const int TACHYON_TITLE_BAR_TOP = __TITLE_BAR_TOP__;
static const int TACHYON_TITLE_BAR_SIDE = __TITLE_BAR_SIDE__;
static const guint TACHYON_TITLE_BAR_REGION_LIMIT = 256;
static const double TACHYON_TITLE_BAR_LARGEST_EDGE = 100000;
static const gchar *TACHYON_TITLE_BAR_PAGE_SCRIPT = "__TITLE_BAR_PAGE_SCRIPT__";

static GArray *g_title_bar_drag = NULL;
static GArray *g_title_bar_exclude = NULL;
static GtkWidget *g_title_bar_start = NULL;
static GtkWidget *g_title_bar_end = NULL;
static WebKitUserContentManager *g_title_bar_manager = NULL;
static WebKitUserScript *g_title_bar_script = NULL;
static gchar *g_title_bar_published = NULL;
static int g_title_bar_frames = 0;

static int tachyon_title_bar_round_up(double value) {
  int whole = (int)value;
  return whole + (whole < value);
}

static const char *tachyon_title_bar_title(const char *requested) {
  return TACHYON_TITLE_BAR_INTEGRATED && requested[0] == '\0' ? TACHYON_APP_NAME : requested;
}

static gboolean tachyon_title_bar_rectangle(JSCValue *item, graphene_rect_t *rectangle) {
  if (!jsc_value_is_array(item)) return FALSE;
  JSCValue *length = jsc_value_object_get_property(item, "length");
  gboolean four = jsc_value_is_number(length) && jsc_value_to_int32(length) == 4;
  g_object_unref(length);
  if (!four) return FALSE;
  double values[4];
  for (guint index = 0; index < 4; index += 1) {
    JSCValue *value = jsc_value_object_get_property_at_index(item, index);
    values[index] = jsc_value_is_number(value) ? jsc_value_to_double(value) : NAN;
    g_object_unref(value);
    if (!isfinite(values[index]) || values[index] > TACHYON_TITLE_BAR_LARGEST_EDGE || values[index] < -TACHYON_TITLE_BAR_LARGEST_EDGE) return FALSE;
  }
  if (values[2] < 0 || values[3] < 0) return FALSE;
  graphene_rect_init(rectangle, (float)values[0], (float)values[1], (float)values[2], (float)values[3]);
  return TRUE;
}

static GArray *tachyon_title_bar_rectangles(JSCValue *request, const char *name) {
  JSCValue *list = jsc_value_object_get_property(request, name);
  JSCValue *length = jsc_value_is_array(list) ? jsc_value_object_get_property(list, "length") : NULL;
  int count = length != NULL && jsc_value_is_number(length) ? jsc_value_to_int32(length) : -1;
  GArray *rectangles = count >= 0 && (guint)count <= TACHYON_TITLE_BAR_REGION_LIMIT ? g_array_new(FALSE, FALSE, sizeof(graphene_rect_t)) : NULL;
  for (int index = 0; rectangles != NULL && index < count; index += 1) {
    JSCValue *item = jsc_value_object_get_property_at_index(list, (guint)index);
    graphene_rect_t rectangle;
    if (tachyon_title_bar_rectangle(item, &rectangle)) g_array_append_val(rectangles, rectangle);
    else g_clear_pointer(&rectangles, g_array_unref);
    g_object_unref(item);
  }
  if (length != NULL) g_object_unref(length);
  g_object_unref(list);
  return rectangles;
}

static gchar *tachyon_title_bar_regions(const gchar *payload) {
  if (!TACHYON_TITLE_BAR_INTEGRATED) return g_strdup("{\"error\":\"Native title bar unavailable.\"}");
  JSCContext *context = jsc_context_new();
  JSCValue *request = jsc_value_new_from_json(context, payload);
  gboolean object = request != NULL && jsc_value_is_object(request) && !jsc_value_is_array(request);
  GArray *drag = object ? tachyon_title_bar_rectangles(request, "drag") : NULL;
  GArray *exclude = object ? tachyon_title_bar_rectangles(request, "exclude") : NULL;
  if (request != NULL) g_object_unref(request);
  g_object_unref(context);
  if (drag == NULL || exclude == NULL) {
    if (drag != NULL) g_array_unref(drag);
    if (exclude != NULL) g_array_unref(exclude);
    return g_strdup("{\"error\":\"Drag regions must be at most 256 rectangles of four numbers.\"}");
  }
  if (g_title_bar_drag != NULL) g_array_unref(g_title_bar_drag);
  if (g_title_bar_exclude != NULL) g_array_unref(g_title_bar_exclude);
  g_title_bar_drag = drag;
  g_title_bar_exclude = exclude;
  return g_strdup("{\"ok\":true}");
}

static gboolean tachyon_title_bar_inside(GArray *rectangles, const graphene_point_t *point) {
  for (guint index = 0; rectangles != NULL && index < rectangles->len; index += 1) {
    if (graphene_rect_contains_point(&g_array_index(rectangles, graphene_rect_t, index), point)) return TRUE;
  }
  return FALSE;
}

static gboolean tachyon_title_bar_drags(GtkWidget *view, double x, double y) {
  double zoom = MAX(webkit_web_view_get_zoom_level(WEBKIT_WEB_VIEW(view)), 0.1);
  graphene_point_t point = GRAPHENE_POINT_INIT((float)(x / zoom), (float)(y / zoom));
  return tachyon_title_bar_inside(g_title_bar_drag, &point) && !tachyon_title_bar_inside(g_title_bar_exclude, &point);
}

static GdkToplevel *tachyon_title_bar_toplevel(GtkWidget *widget) {
  GtkNative *native = gtk_widget_get_native(widget);
  GdkSurface *surface = native != NULL ? gtk_native_get_surface(native) : NULL;
  return surface != NULL && GDK_IS_TOPLEVEL(surface) ? GDK_TOPLEVEL(surface) : NULL;
}

static void tachyon_title_bar_double_click(GtkGesture *gesture) {
  GtkWidget *view = gtk_event_controller_get_widget(GTK_EVENT_CONTROLLER(gesture));
  GdkToplevel *toplevel = tachyon_title_bar_toplevel(view);
  if (g_native_window == NULL || toplevel == NULL) return;
  gchar *action = NULL;
  g_object_get(gtk_widget_get_settings(view), "gtk-titlebar-double-click", &action, NULL);
  if (g_strcmp0(action, "minimize") == 0) gtk_window_minimize(g_native_window);
  else if (g_strcmp0(action, "lower") == 0) gdk_toplevel_lower(toplevel);
  else if (g_strcmp0(action, "menu") == 0) gdk_toplevel_show_window_menu(toplevel, gtk_event_controller_get_current_event(GTK_EVENT_CONTROLLER(gesture)));
  else if (g_strcmp0(action, "none") != 0 && gtk_window_is_maximized(g_native_window)) gtk_window_unmaximize(g_native_window);
  else if (g_strcmp0(action, "none") != 0) gtk_window_maximize(g_native_window);
  g_free(action);
}

static void tachyon_title_bar_pressed(GtkGestureClick *gesture, int presses, double x, double y, gpointer data) {
  (void)data;
  GtkWidget *view = gtk_event_controller_get_widget(GTK_EVENT_CONTROLLER(gesture));
  if (!tachyon_title_bar_drags(view, x, y)) {
    gtk_gesture_set_state(GTK_GESTURE(gesture), GTK_EVENT_SEQUENCE_DENIED);
    return;
  }
  gtk_gesture_set_state(GTK_GESTURE(gesture), GTK_EVENT_SEQUENCE_CLAIMED);
  if (presses == 2) tachyon_title_bar_double_click(GTK_GESTURE(gesture));
}

static void tachyon_title_bar_dragged(GtkGestureDrag *gesture, double offset_x, double offset_y, gpointer data) {
  (void)data;
  GtkGesture *base = GTK_GESTURE(gesture);
  GtkWidget *view = gtk_event_controller_get_widget(GTK_EVENT_CONTROLLER(gesture));
  double x = 0, y = 0;
  gtk_gesture_drag_get_start_point(gesture, &x, &y);
  if (gtk_gesture_get_sequence_state(base, gtk_gesture_get_last_updated_sequence(base)) != GTK_EVENT_SEQUENCE_CLAIMED) return;
  if (!gtk_drag_check_threshold(view, (int)x, (int)y, (int)(x + offset_x), (int)(y + offset_y))) return;
  GtkNative *native = gtk_widget_get_native(view);
  GdkToplevel *toplevel = tachyon_title_bar_toplevel(view);
  graphene_point_t point;
  if (toplevel == NULL || !gtk_widget_compute_point(view, GTK_WIDGET(native), &GRAPHENE_POINT_INIT((float)x, (float)y), &point)) return;
  double surface_x = 0, surface_y = 0;
  gtk_native_get_surface_transform(native, &surface_x, &surface_y);
  gdk_toplevel_begin_move(toplevel, gtk_gesture_get_device(base), (int)gtk_gesture_single_get_current_button(GTK_GESTURE_SINGLE(gesture)),
                          point.x + surface_x, point.y + surface_y, gtk_event_controller_get_current_event_time(GTK_EVENT_CONTROLLER(gesture)));
  tachyon_record("window.move.started");
  gtk_event_controller_reset(GTK_EVENT_CONTROLLER(gesture));
}

static int tachyon_title_bar_extent(GtkWidget *group, GtkWidget *content, gboolean leading, int *height) {
  graphene_rect_t bounds;
  if (!gtk_widget_get_visible(group) || !gtk_widget_compute_bounds(group, content, &bounds) || bounds.size.width <= 0) return 0;
  gboolean left = (gtk_widget_get_direction(content) == GTK_TEXT_DIR_RTL) != leading;
  double offset = left ? bounds.origin.x : gtk_widget_get_width(content) - bounds.origin.x - bounds.size.width;
  *height = MAX(*height, tachyon_title_bar_round_up(bounds.origin.y * 2 + bounds.size.height));
  return tachyon_title_bar_round_up(offset * 2 + bounds.size.width);
}

static void tachyon_title_bar_page_script(const gchar *call) {
  gchar *source = g_strdup_printf("if (location.protocol === 'tachyon:' && location.host === 'app' && window === window.top) {\n%s\n%s\n}\n",
                                  TACHYON_TITLE_BAR_PAGE_SCRIPT, call);
  WebKitUserScript *script = webkit_user_script_new(source, WEBKIT_USER_CONTENT_INJECT_TOP_FRAME, WEBKIT_USER_SCRIPT_INJECT_AT_DOCUMENT_START, NULL, NULL);
  if (g_title_bar_script != NULL) {
    webkit_user_content_manager_remove_script(g_title_bar_manager, g_title_bar_script);
    webkit_user_script_unref(g_title_bar_script);
  }
  webkit_user_content_manager_add_script(g_title_bar_manager, script);
  g_title_bar_script = script;
  g_free(source);
}

static void tachyon_title_bar_publish(GtkWidget *content) {
  gboolean full = g_native_window == NULL || gtk_window_is_fullscreen(g_native_window);
  int height = 0;
  int leading = full ? 0 : tachyon_title_bar_extent(g_title_bar_start, content, TRUE, &height);
  int trailing = full ? 0 : tachyon_title_bar_extent(g_title_bar_end, content, FALSE, &height);
  gchar *call = g_strdup_printf("globalThis.__tachyonTitleBar?.apply({\"height\":%d,\"leading\":%d,\"trailing\":%d})", height, leading, trailing);
  if (g_strcmp0(call, g_title_bar_published) == 0) { g_free(call); return; }
  g_free(g_title_bar_published);
  g_title_bar_published = call;
  tachyon_title_bar_page_script(call);
  if (tachyon_trusted_view(g_view)) webkit_web_view_evaluate_javascript(g_view, call, -1, NULL, NULL, NULL, NULL, NULL);
}

static gboolean tachyon_title_bar_laid_out(GtkWidget *content, GdkFrameClock *clock, gpointer data) {
  (void)clock;
  (void)data;
  g_title_bar_frames -= 1;
  if (g_title_bar_frames > 0) return G_SOURCE_CONTINUE;
  tachyon_title_bar_publish(content);
  return G_SOURCE_REMOVE;
}

static void tachyon_title_bar_changed(GObject *source, GParamSpec *property, gpointer content) {
  (void)source;
  (void)property;
  gboolean full = g_native_window != NULL && gtk_window_is_fullscreen(g_native_window);
  gtk_widget_set_visible(g_title_bar_start, !full);
  gtk_widget_set_visible(g_title_bar_end, !full);
  if (g_title_bar_frames <= 0) gtk_widget_add_tick_callback(GTK_WIDGET(content), tachyon_title_bar_laid_out, NULL, NULL);
  g_title_bar_frames = 2;
}

static void tachyon_title_bar_mapped(GtkWidget *content, gpointer data) {
  (void)data;
  tachyon_title_bar_changed(G_OBJECT(content), NULL, content);
}

static GtkWidget *tachyon_title_bar_group(GtkWidget *group, GtkAlign side) {
  gtk_widget_set_halign(group, side);
  gtk_widget_set_valign(group, GTK_ALIGN_START);
  gtk_widget_set_margin_top(group, TACHYON_TITLE_BAR_TOP);
  gtk_widget_set_margin_start(group, TACHYON_TITLE_BAR_SIDE);
  gtk_widget_set_margin_end(group, TACHYON_TITLE_BAR_SIDE);
  return group;
}

static GtkWidget *tachyon_title_bar_menu(GtkWindow *window) {
  GtkWidget *button = gtk_menu_button_new();
  GMenuModel *menus = gtk_application_get_menubar(gtk_window_get_application(window));
  gtk_menu_button_set_icon_name(GTK_MENU_BUTTON(button), "open-menu-symbolic");
  gtk_menu_button_set_menu_model(GTK_MENU_BUTTON(button), menus);
  gtk_menu_button_set_primary(GTK_MENU_BUTTON(button), TRUE);
  gtk_menu_button_set_has_frame(GTK_MENU_BUTTON(button), FALSE);
  gtk_widget_set_valign(button, GTK_ALIGN_CENTER);
  gtk_widget_set_visible(button, menus != NULL);
  gtk_widget_set_tooltip_text(button, "Main Menu");
  gtk_accessible_update_property(GTK_ACCESSIBLE(button), GTK_ACCESSIBLE_PROPERTY_LABEL, "Main Menu", -1);
  return button;
}

static void tachyon_title_bar_gestures(GtkWidget *view) {
  GtkGesture *click = gtk_gesture_click_new();
  GtkGesture *drag = gtk_gesture_drag_new();
  gtk_gesture_single_set_button(GTK_GESTURE_SINGLE(click), GDK_BUTTON_PRIMARY);
  gtk_gesture_single_set_button(GTK_GESTURE_SINGLE(drag), GDK_BUTTON_PRIMARY);
  gtk_event_controller_set_propagation_phase(GTK_EVENT_CONTROLLER(click), GTK_PHASE_CAPTURE);
  gtk_event_controller_set_propagation_phase(GTK_EVENT_CONTROLLER(drag), GTK_PHASE_CAPTURE);
  g_signal_connect(click, "pressed", G_CALLBACK(tachyon_title_bar_pressed), NULL);
  g_signal_connect(drag, "drag-update", G_CALLBACK(tachyon_title_bar_dragged), NULL);
  gtk_widget_add_controller(view, GTK_EVENT_CONTROLLER(click));
  gtk_widget_add_controller(view, GTK_EVENT_CONTROLLER(drag));
  gtk_gesture_group(click, drag);
}

static GtkWidget *tachyon_title_bar_content(GtkWindow *window, WebKitWebView *view, WebKitUserContentManager *manager) {
  if (!TACHYON_TITLE_BAR_INTEGRATED) return GTK_WIDGET(view);
  GtkWidget *nothing = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
  gtk_widget_set_visible(nothing, FALSE);
  gtk_window_set_titlebar(window, nothing);
  gtk_application_window_set_show_menubar(GTK_APPLICATION_WINDOW(window), FALSE);
  g_title_bar_manager = manager;
  tachyon_title_bar_page_script("");
  GtkWidget *overlay = gtk_overlay_new();
  gtk_overlay_set_child(GTK_OVERLAY(overlay), GTK_WIDGET(view));
  g_title_bar_start = tachyon_title_bar_group(gtk_window_controls_new(GTK_PACK_START), GTK_ALIGN_START);
  g_title_bar_end = tachyon_title_bar_group(gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6), GTK_ALIGN_END);
  gtk_box_append(GTK_BOX(g_title_bar_end), tachyon_title_bar_menu(window));
  gtk_box_append(GTK_BOX(g_title_bar_end), gtk_window_controls_new(GTK_PACK_END));
  gtk_overlay_add_overlay(GTK_OVERLAY(overlay), g_title_bar_start);
  gtk_overlay_add_overlay(GTK_OVERLAY(overlay), g_title_bar_end);
  tachyon_title_bar_gestures(GTK_WIDGET(view));
  g_signal_connect(overlay, "map", G_CALLBACK(tachyon_title_bar_mapped), NULL);
  const char *changes[] = {"notify::fullscreened", "notify::maximized"};
  for (gsize index = 0; index < G_N_ELEMENTS(changes); index += 1) {
    g_signal_connect_object(window, changes[index], G_CALLBACK(tachyon_title_bar_changed), overlay, 0);
  }
  g_signal_connect_object(g_title_bar_start, "notify::empty", G_CALLBACK(tachyon_title_bar_changed), overlay, 0);
  g_signal_connect_object(gtk_widget_get_settings(GTK_WIDGET(window)), "notify::gtk-decoration-layout", G_CALLBACK(tachyon_title_bar_changed), overlay, 0);
  return overlay;
}
