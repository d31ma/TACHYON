#include <stdlib.h>
#include <gdk/x11/gdkx.h>

#ifndef TACHYON_BAR_DECLARED_TOP
#define TACHYON_BAR_DECLARED_TOP 6
#define TACHYON_BAR_DECLARED_SIDE 6
#endif

static const int TACHYON_BAR_LEFT_BUTTON = 0x110;

typedef int (*TachyonFakeMotion)(Display *, int, int, int, unsigned long);
typedef int (*TachyonFakeButton)(Display *, unsigned int, int, unsigned long);
typedef int (*TachyonFlush)(Display *);
typedef int (*TachyonTranslate)(Display *, Window, Window, int, int, int *, int *, Window *);

static const char *TACHYON_BAR_PAGE =
    "(() => { const place = (element, style) => Object.assign(element.style, style);"
    "const header = document.createElement('header'); header.dataset.tacDragRegion = '';"
    "place(header, { position: 'fixed', left: '0', top: '0', right: '0', height: '80px', background: '#234' });"
    "const inside = document.createElement('button'); inside.id = 'inside'; inside.textContent = 'Inside';"
    "place(inside, { position: 'absolute', left: '300px', top: '20px', width: '120px', height: '40px' });"
    "const below = document.createElement('main'); below.id = 'below'; below.textContent = 'Below';"
    "place(below, { position: 'fixed', left: '0', top: '120px', right: '0', bottom: '0' });"
    "header.append(inside); document.body.replaceChildren(header, below); document.body.style.margin = '0';"
    "globalThis.__pressed = [];"
    "addEventListener('mousedown', (event) => __pressed.push(event.target.id || event.target.tagName), true);"
    "return 'ready' })()";
static const char *TACHYON_BAR_INSETS =
    "(() => { const style = getComputedStyle(document.documentElement);"
    "const value = (name) => parseInt(style.getPropertyValue(name), 10);"
    "return JSON.stringify({ height: value('--tac-titlebar-height'), leading: value('--tac-titlebar-leading-inset'),"
    " trailing: value('--tac-titlebar-trailing-inset') }) })()";

static int g_bar_step = 0;
static int g_bar_tick = 0;
static gint64 g_bar_started = 0;
static gboolean g_bar_waiting = FALSE;
static gchar *g_bar_answer = NULL;
static TachyonFakeMotion g_bar_motion = NULL;
static TachyonFakeButton g_bar_button = NULL;
static TachyonFlush g_bar_flush = NULL;
static TachyonTranslate g_bar_translate = NULL;
static int g_bar_origin_x = 0, g_bar_origin_y = 0;
static int g_bar_drag_x = 0, g_bar_drag_y = 0;
static gboolean g_bar_wayland = FALSE;
static GDBusConnection *g_bar_bus = NULL;
static gchar *g_bar_session = NULL;
static int g_bar_pointer_x = 0, g_bar_pointer_y = 0;

static void tachyon_bar_fail(const char *message) {
  fprintf(stderr, "linux title bar fixture failed at step %d tick %d after %.1f s: %s (insets %s, answer %s)\n", g_bar_step, g_bar_tick,
          (double)(g_get_monotonic_time() - g_bar_started) / G_USEC_PER_SEC, message,
          g_title_bar_published != NULL ? g_title_bar_published : "none", g_bar_answer != NULL ? g_bar_answer : "none");
  exit(1);
}

static void tachyon_bar_require(gboolean valid, const char *message) {
  if (!valid) tachyon_bar_fail(message);
}

static void tachyon_bar_load_x11(void) {
  GModule *test = g_module_open("libXtst.so.6", G_MODULE_BIND_LAZY);
  GModule *self = g_module_open(NULL, G_MODULE_BIND_LAZY);
  tachyon_bar_require(test != NULL && self != NULL, "libXtst.so.6 is not installed");
  tachyon_bar_require(g_module_symbol(test, "XTestFakeMotionEvent", (gpointer *)&g_bar_motion) &&
                      g_module_symbol(test, "XTestFakeButtonEvent", (gpointer *)&g_bar_button) &&
                      g_module_symbol(self, "XFlush", (gpointer *)&g_bar_flush) &&
                      g_module_symbol(self, "XTranslateCoordinates", (gpointer *)&g_bar_translate),
                      "the X11 test functions are missing");
}

static GVariant *tachyon_bar_remote(const char *path, const char *interface, const char *method, GVariant *arguments) {
  GError *error = NULL;
  GVariant *answer = g_dbus_connection_call_sync(g_bar_bus, "org.gnome.Mutter.RemoteDesktop", path, interface, method, arguments, NULL,
                                                 G_DBUS_CALL_FLAGS_NONE, -1, NULL, &error);
  if (answer == NULL) tachyon_bar_fail(error != NULL ? error->message : method);
  return answer;
}

static void tachyon_bar_session(const char *method, GVariant *arguments) {
  g_variant_unref(tachyon_bar_remote(g_bar_session, "org.gnome.Mutter.RemoteDesktop.Session", method, arguments));
}

static void tachyon_bar_load_wayland(void) {
  g_bar_bus = g_bus_get_sync(G_BUS_TYPE_SESSION, NULL, NULL);
  tachyon_bar_require(g_bar_bus != NULL, "the fixture needs the session bus of a headless mutter compositor");
  GVariant *created = tachyon_bar_remote("/org/gnome/Mutter/RemoteDesktop", "org.gnome.Mutter.RemoteDesktop", "CreateSession", NULL);
  g_variant_get(created, "(o)", &g_bar_session);
  g_variant_unref(created);
  tachyon_bar_session("Start", NULL);
  tachyon_bar_session("NotifyPointerMotionRelative", g_variant_new("(dd)", -10000.0, -10000.0));
}

static void tachyon_bar_load_input(void) {
  g_bar_wayland = !GDK_IS_X11_DISPLAY(gtk_widget_get_display(GTK_WIDGET(g_native_window)));
  if (g_bar_wayland) tachyon_bar_load_wayland();
  else tachyon_bar_load_x11();
}

static Display *tachyon_bar_display(void) {
  GdkDisplay *display = gtk_widget_get_display(GTK_WIDGET(g_native_window));
  tachyon_bar_require(GDK_IS_X11_DISPLAY(display), "the fixture needs an X11 display (GDK_BACKEND=x11)");
  return gdk_x11_display_get_xdisplay(display);
}

static void tachyon_bar_root(int x, int y, int *root_x, int *root_y) {
  Display *display = tachyon_bar_display();
  Window child = 0;
  g_bar_translate(display, gdk_x11_surface_get_xid(gtk_native_get_surface(GTK_NATIVE(g_native_window))), DefaultRootWindow(display), x, y,
                  root_x, root_y, &child);
}

static void tachyon_bar_page_root(double x, double y, int *root_x, int *root_y) {
  graphene_point_t point;
  tachyon_bar_require(gtk_widget_compute_point(GTK_WIDGET(g_view), GTK_WIDGET(g_native_window), &GRAPHENE_POINT_INIT((float)x, (float)y), &point),
                      "the page is not inside the window");
  double surface_x = 0, surface_y = 0;
  gtk_native_get_surface_transform(GTK_NATIVE(g_native_window), &surface_x, &surface_y);
  if (!g_bar_wayland) {
    tachyon_bar_root((int)(point.x + surface_x), (int)(point.y + surface_y), root_x, root_y);
    return;
  }
  tachyon_bar_require(gtk_window_is_maximized(g_native_window), "a Wayland client knows where its window is only when it fills the monitor");
  *root_x = (int)(point.x + surface_x);
  *root_y = (int)(point.y + surface_y);
}

static void tachyon_bar_pointer(int root_x, int root_y) {
  if (g_bar_wayland) {
    tachyon_bar_session("NotifyPointerMotionRelative", g_variant_new("(dd)", (double)(root_x - g_bar_pointer_x), (double)(root_y - g_bar_pointer_y)));
    g_bar_pointer_x = root_x;
    g_bar_pointer_y = root_y;
    return;
  }
  Display *display = tachyon_bar_display();
  g_bar_motion(display, -1, root_x, root_y, 0);
  g_bar_flush(display);
}

static void tachyon_bar_press(gboolean down) {
  if (g_bar_wayland) {
    tachyon_bar_session("NotifyPointerButton", g_variant_new("(ib)", TACHYON_BAR_LEFT_BUTTON, down));
    return;
  }
  Display *display = tachyon_bar_display();
  g_bar_button(display, 1, down, 0);
  g_bar_flush(display);
}

static void tachyon_bar_replied(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  JSCValue *value = webkit_web_view_evaluate_javascript_finish(WEBKIT_WEB_VIEW(source), result, &error);
  if (value == NULL) tachyon_bar_fail(error != NULL ? error->message : "script failed");
  g_bar_answer = jsc_value_to_string(value);
  g_object_unref(value);
  g_bar_waiting = FALSE;
}

static void tachyon_bar_evaluate(const char *script) {
  g_clear_pointer(&g_bar_answer, g_free);
  g_bar_waiting = TRUE;
  webkit_web_view_evaluate_javascript(g_view, script, -1, NULL, NULL, NULL, tachyon_bar_replied, NULL);
}

static gboolean tachyon_bar_page_loaded(const char *uri) {
  return g_view != NULL && g_strcmp0(webkit_web_view_get_uri(g_view), uri) == 0 && !webkit_web_view_is_loading(g_view) &&
         g_native_window != NULL && gtk_widget_get_mapped(GTK_WIDGET(g_native_window)) && g_title_bar_published != NULL;
}

static int tachyon_bar_inset(const char *name) {
  gchar *key = g_strdup_printf("\"%s\":", name);
  const char *found = g_title_bar_published != NULL ? strstr(g_title_bar_published, key) : NULL;
  g_free(key);
  return found != NULL ? atoi(strchr(found, ':') + 1) : -1;
}

static gboolean tachyon_bar_page_has_published_insets(void) {
  const char *open = strchr(g_title_bar_published, '{');
  const char *close = strrchr(g_title_bar_published, '}');
  return g_bar_answer != NULL && open != NULL && close != NULL && strlen(g_bar_answer) == (size_t)(close - open + 1) &&
         strncmp(g_bar_answer, open, (size_t)(close - open + 1)) == 0;
}

static void tachyon_bar_check_widgets(void) {
  GtkWidget *titlebar = gtk_window_get_titlebar(g_native_window);
  tachyon_bar_require(titlebar != NULL && !gtk_widget_get_visible(titlebar), "the window kept a visible title bar");
  tachyon_bar_require(!gtk_application_window_get_show_menubar(GTK_APPLICATION_WINDOW(g_native_window)), "the menu bar still covers the top edge");
  GtkWidget *menu = gtk_widget_get_first_child(g_title_bar_end);
  tachyon_bar_require(GTK_IS_MENU_BUTTON(menu) && gtk_widget_get_visible(menu) &&
                      gtk_menu_button_get_menu_model(GTK_MENU_BUTTON(menu)) == gtk_application_get_menubar(gtk_window_get_application(g_native_window)),
                      "the main menu button does not hold the application menus");
  tachyon_bar_require(GTK_IS_WINDOW_CONTROLS(gtk_widget_get_next_sibling(menu)) && GTK_IS_WINDOW_CONTROLS(g_title_bar_start),
                      "the window buttons are not GTK window controls");
  graphene_rect_t bounds;
  tachyon_bar_require(gtk_widget_compute_bounds(GTK_WIDGET(g_view), GTK_WIDGET(g_native_window), &bounds) && bounds.origin.y == 0,
                      "the page does not reach the top edge of the window");
  tachyon_bar_require(tachyon_bar_inset("leading") > 12 && tachyon_bar_inset("trailing") > 12 && tachyon_bar_inset("height") > 12,
                      "the insets do not hold the window buttons");
  GtkWidget *content = gtk_widget_get_parent(GTK_WIDGET(g_view));
  graphene_rect_t start, end;
  tachyon_bar_require(gtk_widget_compute_bounds(g_title_bar_start, content, &start) && gtk_widget_compute_bounds(g_title_bar_end, content, &end),
                      "the window buttons are not laid out");
  tachyon_bar_require((int)start.origin.y == TACHYON_BAR_DECLARED_TOP && (int)end.origin.y == TACHYON_BAR_DECLARED_TOP &&
                      (int)start.origin.x == TACHYON_BAR_DECLARED_SIDE &&
                      (int)(gtk_widget_get_width(content) - end.origin.x - end.size.width) == TACHYON_BAR_DECLARED_SIDE,
                      "the window buttons are not at the declared titleBarButtonInset");
}

static gboolean tachyon_bar_clicks(void) {
  static const double points[3][2] = {{360, 40}, {600, 40}, {200, 300}};
  int point = g_bar_tick / 3;
  if (point == 3) { tachyon_bar_evaluate("JSON.stringify(globalThis.__pressed)"); return TRUE; }
  int root_x = 0, root_y = 0;
  tachyon_bar_page_root(points[point][0], points[point][1], &root_x, &root_y);
  if (g_bar_tick % 3 == 0) tachyon_bar_pointer(root_x, root_y);
  else tachyon_bar_press(g_bar_tick % 3 == 1);
  return FALSE;
}

static void tachyon_bar_drag_pointer(void) {
  if (g_bar_tick == 0) {
    tachyon_bar_page_root(600, 40, &g_bar_drag_x, &g_bar_drag_y);
    tachyon_bar_pointer(g_bar_drag_x, g_bar_drag_y);
  } else if (g_bar_tick == 1) tachyon_bar_press(TRUE);
  else if (g_bar_tick <= 13) tachyon_bar_pointer(g_bar_drag_x + (g_bar_tick - 1) * 10, g_bar_drag_y + (g_bar_tick - 1) * 6);
  else if (g_bar_tick == 14) tachyon_bar_press(FALSE);
}

static gboolean tachyon_bar_drag(void) {
  if (g_bar_tick == 0) tachyon_bar_root(0, 0, &g_bar_origin_x, &g_bar_origin_y);
  tachyon_bar_drag_pointer();
  return g_bar_tick >= 24;
}

static gboolean tachyon_bar_moved(void) {
  int x = 0, y = 0;
  tachyon_bar_root(0, 0, &x, &y);
  gboolean moved = abs(x - g_bar_origin_x - 120) <= 30 && abs(y - g_bar_origin_y - 72) <= 30;
  if (!moved && g_bar_tick > 100) {
    printf("window origin moved from %d,%d to %d,%d\n", g_bar_origin_x, g_bar_origin_y, x, y);
    tachyon_bar_fail("dragging the drag region did not move the window");
  }
  return moved;
}

static gboolean tachyon_bar_double_click(gboolean maximized) {
  int press = g_bar_tick - 40;
  if (press == 0) {
    int root_x = 0, root_y = 0;
    tachyon_bar_page_root(600, 40, &root_x, &root_y);
    tachyon_bar_pointer(root_x, root_y);
  } else if (press > 0 && press <= 4) tachyon_bar_press(press % 2 == 1);
  return press > 4 && gtk_window_is_maximized(g_native_window) == maximized;
}

static gboolean tachyon_bar_unmaximized_by_drag(void) {
  if (g_bar_tick <= 14) {
    tachyon_bar_drag_pointer();
    return FALSE;
  }
  if (g_bar_tick > 100) tachyon_bar_fail("dragging the drag region of a maximized window did not start a compositor move");
  return !gtk_window_is_maximized(g_native_window);
}

static gboolean tachyon_bar_wayland_window_step(void) {
  switch (g_bar_step) {
    case 5: return tachyon_bar_double_click(FALSE);
    case 6:
      if (g_bar_tick == 0) gtk_window_maximize(g_native_window);
      return gtk_window_is_maximized(g_native_window);
    case 7: return tachyon_bar_unmaximized_by_drag();
    default: return TRUE;
  }
}

static gboolean tachyon_bar_fullscreen_insets(void) {
  return tachyon_bar_inset("height") == 0 && tachyon_bar_inset("leading") == 0 && tachyon_bar_inset("trailing") == 0 &&
         !gtk_widget_get_visible(g_title_bar_start) && !gtk_widget_get_visible(g_title_bar_end);
}

static void tachyon_bar_check_title(void) {
  size_t length = 0;
  unsigned char *answer = tachyon_capability_call("window.title", "{\"title\":\"\"}", &length);
  tachyon_capability_free(answer, length);
  tachyon_bar_require(g_strcmp0(gtk_window_get_title(g_native_window), TACHYON_APP_NAME) == 0, "an empty page title removed the window name");
}

static gboolean tachyon_bar_step(void) {
  if (g_bar_wayland && g_bar_step >= 5 && g_bar_step <= 8) return tachyon_bar_wayland_window_step();
  switch (g_bar_step) {
    case 0:
      if (!tachyon_bar_page_loaded("tachyon://app/")) return FALSE;
      tachyon_bar_load_input();
      if (g_bar_wayland) gtk_window_maximize(g_native_window);
      g_object_set(gtk_settings_get_default(), "gtk-decoration-layout", "close:minimize,maximize", NULL);
      tachyon_bar_evaluate(TACHYON_BAR_PAGE);
      return TRUE;
    case 1:
      if (g_bar_wayland && !gtk_window_is_maximized(g_native_window)) return FALSE;
      if (g_title_bar_drag == NULL || g_title_bar_drag->len != 1 || g_title_bar_exclude->len != 1 || tachyon_bar_inset("leading") <= 0) return FALSE;
      tachyon_bar_check_widgets();
      tachyon_bar_evaluate(TACHYON_BAR_INSETS);
      return TRUE;
    case 2: tachyon_bar_require(tachyon_bar_page_has_published_insets(), "the page did not get the host insets"); return TRUE;
    case 3: return tachyon_bar_clicks();
    case 4: tachyon_bar_require(g_strcmp0(g_bar_answer, "[\"inside\",\"below\"]") == 0, "a drag region click reached the page, or a control click did not"); return TRUE;
    case 5: return tachyon_bar_drag();
    case 6: return tachyon_bar_moved();
    case 7: return tachyon_bar_double_click(TRUE);
    case 8: return tachyon_bar_double_click(FALSE);
    case 9: gtk_window_fullscreen(g_native_window); return TRUE;
    case 10: if (!tachyon_bar_fullscreen_insets()) return FALSE; tachyon_bar_evaluate(TACHYON_BAR_INSETS); return TRUE;
    case 11: tachyon_bar_require(tachyon_bar_page_has_published_insets(), "the page kept insets in full screen"); gtk_window_unfullscreen(g_native_window); return TRUE;
    case 12: if (tachyon_bar_inset("leading") <= 0) return FALSE; webkit_web_view_load_uri(g_view, "tachyon://app/help/"); return TRUE;
    case 13: if (!tachyon_bar_page_loaded("tachyon://app/help/")) return FALSE; tachyon_bar_evaluate(TACHYON_BAR_INSETS); return TRUE;
    case 14: tachyon_bar_require(tachyon_bar_page_has_published_insets(), "a new page did not start with the insets"); tachyon_bar_check_title(); return TRUE;
    default: return FALSE;
  }
}

static gboolean tachyon_bar_tick(gpointer data) {
  (void)data;
  if (g_bar_waiting) return G_SOURCE_CONTINUE;
  gboolean advanced = tachyon_bar_step();
  g_bar_tick += 1;
  if (!advanced) return G_SOURCE_CONTINUE;
  g_bar_step += 1;
  g_bar_tick = 0;
  printf("step %d done after %.1f s\n", g_bar_step, (double)(g_get_monotonic_time() - g_bar_started) / G_USEC_PER_SEC);
  if (g_bar_step == 15) {
    g_application_quit(g_application_get_default());
    return G_SOURCE_REMOVE;
  }
  return G_SOURCE_CONTINUE;
}

static gboolean tachyon_bar_deadline(gpointer data) {
  (void)data;
  tachyon_bar_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_bar_started = g_get_monotonic_time();
  g_timeout_add(20, tachyon_bar_tick, NULL);
  g_timeout_add_seconds(110, tachyon_bar_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  tachyon_bar_require(status == 0 && g_bar_step == 15, "the host did not finish its run");
  printf("linux title bar passed on %s: the page reached the top edge under GTK window buttons, got the insets, and its drag region moved, maximized and restored the window\n", g_bar_wayland ? "Wayland" : "X11");
  return 0;
}
