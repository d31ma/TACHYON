#include <stdlib.h>
#include <gdk/x11/gdkx.h>

typedef int (*TachyonFakeKey)(Display *, unsigned int, int, unsigned long);
typedef unsigned char (*TachyonKeysymToKeycode)(Display *, unsigned long);
typedef int (*TachyonFlush)(Display *);

#define TACHYON_PICKER_FOLDER "__PICKER_FOLDER__"
#define TACHYON_PICKER_ESCAPE 0xff1b
#define TACHYON_PICKER_RETURN 0xff0d
#define TACHYON_PICKER_DELETE 0xffff
#define TACHYON_PICKER_CONTROL 0xffe3
#define TACHYON_PICKER_LOCATION 0x006c
#define TACHYON_PICKER_SELECT_ALL 0x0061
#define TACHYON_PICKER_PASTE 0x0076
#define TACHYON_PICKER_SETTLE_TICKS 25

static const char *TACHYON_PICKER_CHOICES[] = {TACHYON_PICKER_FOLDER "/notes.txt", NULL,
                                               TACHYON_PICKER_FOLDER "/data.json"};
static int g_picker_step = 0;
static gint64 g_picker_started = 0;
static gboolean g_picker_waiting = FALSE;
static gchar *g_picker_reply = NULL;
static gint64 g_picker_now = 0;
static gboolean g_picker_open = FALSE;
static gboolean g_picker_in_process = FALSE;
static int g_picker_dialogs = 0;
static const char *g_picker_choice = NULL;
static gboolean g_picker_finished = FALSE;
static int g_picker_action = 0;
static int g_picker_settle = 0;
static gboolean g_picker_refused_second = FALSE;
static TachyonFakeKey g_picker_key = NULL;
static TachyonKeysymToKeycode g_picker_keycode = NULL;
static TachyonFlush g_picker_flush = NULL;

static void tachyon_picker_fail(const char *message) {
  fprintf(stderr, "linux file picker fixture failed at step %d after %.1f s: %s (reply %s, dialogs %d)\n", g_picker_step,
          (double)(g_get_monotonic_time() - g_picker_started) / G_USEC_PER_SEC, message,
          g_picker_reply != NULL ? g_picker_reply : "none", g_picker_dialogs);
  exit(1);
}

static void tachyon_picker_require(gboolean valid, const char *message) {
  if (!valid) tachyon_picker_fail(message);
}

static gint64 tachyon_picker_clock(void) {
  return g_picker_now;
}

static gchar *tachyon_picker_call(const char *operation, const char *json) {
  JSCContext *context = jsc_context_new();
  JSCValue *request = jsc_value_new_from_json(context, json);
  gchar *answer = tachyon_file_picker_call(operation, request);
  g_object_unref(request);
  g_object_unref(context);
  return answer;
}

static gboolean tachyon_picker_answers(const char *operation, const char *json, const char *status) {
  gchar *answer = tachyon_picker_call(operation, json);
  gchar *expected = g_strdup_printf("{\"status\":\"%s\"", status);
  gboolean matches = g_str_has_prefix(answer, expected);
  if (!matches) fprintf(stderr, "%s %s answered %s, not %s\n", operation, json, answer, status);
  g_free(expected);
  g_free(answer);
  return matches;
}

static gchar *tachyon_picker_owner(int index) {
  return g_strdup_printf("00000000-0000-4000-8000-%012d", index);
}

static gchar *tachyon_picker_keep(int index, const char *name) {
  JSCContext *context = jsc_context_new();
  gchar *owner = tachyon_picker_owner(index);
  gchar *path = g_build_filename(TACHYON_PICKER_FOLDER, name, NULL);
  GFile *file = g_file_new_for_path(path);
  gchar *answer = tachyon_file_picker_keep(context, owner, file);
  JSCValue *result = jsc_value_new_from_json(context, answer);
  JSCValue *handle = jsc_value_object_get_property(result, "handle");
  gchar *text = jsc_value_to_string(handle);
  g_object_unref(handle);
  g_object_unref(result);
  g_free(answer);
  g_object_unref(file);
  g_free(path);
  g_free(owner);
  g_object_unref(context);
  return text;
}

static gchar *tachyon_picker_request(int index, const char *handle) {
  gchar *owner = tachyon_picker_owner(index);
  gchar *json = g_strdup_printf("{\"owner\":\"%s\",\"handle\":\"%s\",\"types\":[\"text/plain\"]}", owner, handle);
  g_free(owner);
  return json;
}

static gboolean tachyon_picker_handle_answers(const char *operation, int index, const char *handle, const char *status) {
  gchar *json = tachyon_picker_request(index, handle);
  gboolean matches = tachyon_picker_answers(operation, json, status);
  g_free(json);
  return matches;
}

static void tachyon_picker_check_handle_limits(void) {
  for (int owner = 0; owner < TACHYON_FILE_PICKER_HANDLE_LIMIT / TACHYON_FILE_PICKER_OWNER_LIMIT; owner += 1) {
    for (int index = 0; index < TACHYON_FILE_PICKER_OWNER_LIMIT; index += 1) g_free(tachyon_picker_keep(owner, "notes.txt"));
  }
  tachyon_picker_require(g_hash_table_size(g_picked_files) == TACHYON_FILE_PICKER_HANDLE_LIMIT, "the fixture must fill every handle");
  tachyon_picker_require(tachyon_picker_handle_answers("open", 16, "", "unavailable"), "the handle after the limit must be refused");
  tachyon_picker_require(g_hash_table_size(g_picked_files) == TACHYON_FILE_PICKER_HANDLE_LIMIT, "a refused open must not add a handle");
  tachyon_file_picker_dispose_all();
  for (int index = 0; index < TACHYON_FILE_PICKER_OWNER_LIMIT; index += 1) g_free(tachyon_picker_keep(0, "notes.txt"));
  tachyon_picker_require(tachyon_picker_handle_answers("open", 0, "", "unavailable"), "one owner must not pass its own limit");
  tachyon_file_picker_dispose_all();
}

static void tachyon_picker_check_lifetime(void) {
  g_picker_now = (gint64)1000000 * G_USEC_PER_SEC;
  g_file_picker_clock = tachyon_picker_clock;
  gchar *handle = tachyon_picker_keep(0, "notes.txt");
  g_picker_now += TACHYON_FILE_PICKER_LIFETIME - 1;
  tachyon_picker_require(tachyon_picker_handle_answers("readText", 0, handle, "read"), "a handle must work until its lifetime ends");
  g_picker_now += 1;
  tachyon_picker_require(tachyon_picker_handle_answers("readText", 0, handle, "invalid"), "an expired handle must answer invalid");
  tachyon_picker_require(g_hash_table_size(g_picked_files) == 0, "an expired handle must be removed");
  tachyon_picker_require(tachyon_picker_handle_answers("release", 0, handle, "invalid"), "an expired handle cannot be released");
  g_file_picker_clock = g_get_monotonic_time;
  g_free(handle);
}

static void tachyon_picker_check_owners_and_reads(void) {
  gchar *handle = tachyon_picker_keep(0, "notes.txt");
  tachyon_picker_require(tachyon_picker_handle_answers("readText", 1, handle, "invalid"), "another owner read the handle");
  tachyon_picker_require(tachyon_picker_handle_answers("release", 1, handle, "invalid"), "another owner released the handle");
  tachyon_picker_require(tachyon_picker_handle_answers("available", 0, handle, "available"), "a granted picker must be available");
  tachyon_picker_require(tachyon_picker_answers("readText", "{\"owner\":\"not-a-uuid\"}", "invalid"), "an owner must be a UUID");
  tachyon_picker_require(tachyon_picker_answers("open", "{\"owner\":\"00000000-0000-4000-8000-000000000000\"}", "invalid"), "an open needs a type list");
  tachyon_picker_require(tachyon_picker_answers("open", "{\"owner\":\"00000000-0000-4000-8000-000000000000\",\"types\":[\"image/png\"]}", "invalid"), "an image type must be refused");
  tachyon_picker_require(tachyon_picker_answers("open", "{\"owner\":\"00000000-0000-4000-8000-000000000000\",\"types\":[\"text/plain\",\"application/json\",\"text/plain\"]}", "invalid"), "three types must be refused");
  tachyon_picker_require(tachyon_picker_handle_answers("dispose", 0, handle, "disposed"), "dispose must answer disposed");
  tachyon_picker_require(g_hash_table_size(g_picked_files) == 0, "dispose must remove the owner's handles");
  g_free(handle);
  gchar *large = tachyon_picker_keep(0, "large.txt");
  tachyon_picker_require(tachyon_picker_handle_answers("readText", 0, large, "tooLarge"), "a file over 65536 bytes must be too large");
  gchar *binary = tachyon_picker_keep(0, "binary.txt");
  tachyon_picker_require(tachyon_picker_handle_answers("readText", 0, binary, "invalid"), "a file that is not UTF-8 must be invalid");
  g_file_picker_granted = FALSE;
  tachyon_picker_require(tachyon_picker_handle_answers("open", 0, "", "denied"), "a picker without its grant must answer denied");
  tachyon_picker_require(g_hash_table_size(g_picked_files) == 0, "a denied call must dispose the owner's handles");
  g_file_picker_granted = TRUE;
  g_free(large);
  g_free(binary);
}

static void tachyon_picker_replied(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  JSCValue *value = webkit_web_view_call_async_javascript_function_finish(WEBKIT_WEB_VIEW(source), result, &error);
  if (value == NULL) tachyon_picker_fail(error != NULL ? error->message : "script failed");
  g_free(g_picker_reply);
  g_picker_reply = jsc_value_to_string(value);
  g_object_unref(value);
  g_picker_waiting = FALSE;
}

static void tachyon_picker_run(const char *script) {
  g_picker_waiting = TRUE;
  webkit_web_view_call_async_javascript_function(g_view, script, -1, NULL, NULL, NULL, NULL, tachyon_picker_replied, NULL);
}

static void tachyon_picker_load_x11(void) {
  GModule *test = g_module_open("libXtst.so.6", G_MODULE_BIND_LAZY);
  GModule *self = g_module_open(NULL, G_MODULE_BIND_LAZY);
  tachyon_picker_require(test != NULL && self != NULL, "libXtst.so.6 is not installed");
  tachyon_picker_require(g_module_symbol(test, "XTestFakeKeyEvent", (gpointer *)&g_picker_key) &&
                         g_module_symbol(self, "XKeysymToKeycode", (gpointer *)&g_picker_keycode) &&
                         g_module_symbol(self, "XFlush", (gpointer *)&g_picker_flush),
                         "the X11 test functions are missing");
}

static unsigned int tachyon_picker_keycode(Display *display, unsigned long keysym) {
  unsigned int keycode = g_picker_keycode(display, keysym);
  tachyon_picker_require(keycode != 0, "the X11 keymap has no key the fixture needs");
  return keycode;
}

static void tachyon_picker_wayland_press(unsigned long keysym, gboolean control) {
  if (control) tachyon_wayland_key(TACHYON_PICKER_CONTROL, TRUE);
  tachyon_wayland_key(keysym, TRUE);
  tachyon_wayland_key(keysym, FALSE);
  if (control) tachyon_wayland_key(TACHYON_PICKER_CONTROL, FALSE);
}

static void tachyon_picker_press(unsigned long keysym, gboolean control) {
  if (tachyon_wayland_input_wanted()) {
    tachyon_picker_wayland_press(keysym, control);
    return;
  }
  if (g_picker_key == NULL) tachyon_picker_load_x11();
  GdkDisplay *display = gdk_display_get_default();
  tachyon_picker_require(GDK_IS_X11_DISPLAY(display), "the fixture needs an X11 display (GDK_BACKEND=x11)");
  G_GNUC_BEGIN_IGNORE_DEPRECATIONS
  Display *xdisplay = gdk_x11_display_get_xdisplay(display);
  G_GNUC_END_IGNORE_DEPRECATIONS
  unsigned int keycode = tachyon_picker_keycode(xdisplay, keysym);
  unsigned int modifier = tachyon_picker_keycode(xdisplay, TACHYON_PICKER_CONTROL);
  if (control) g_picker_key(xdisplay, modifier, True, 0);
  g_picker_key(xdisplay, keycode, True, 0);
  g_picker_key(xdisplay, keycode, False, 0);
  if (control) g_picker_key(xdisplay, modifier, False, 0);
  g_picker_flush(xdisplay);
}

static GtkWindow *tachyon_picker_shown_dialog(void) {
  GListModel *windows = gtk_window_get_toplevels();
  GtkWindow *found = NULL;
  for (guint index = 0; found == NULL && index < g_list_model_get_n_items(windows); index += 1) {
    GtkWindow *window = g_list_model_get_item(windows, index);
    if (window != g_native_window && gtk_widget_get_visible(GTK_WIDGET(window))) found = window;
    g_object_unref(window);
  }
  return found;
}

static void tachyon_picker_check_one_dialog(void) {
  if (g_picker_refused_second) return;
  tachyon_picker_require(tachyon_picker_handle_answers("open", 9, "", "unavailable"), "a second dialog must be refused while one is open");
  g_picker_refused_second = TRUE;
}

static void tachyon_picker_choose(void) {
  switch (g_picker_action) {
    case 0: tachyon_picker_press(TACHYON_PICKER_LOCATION, TRUE); break;
    case 1: tachyon_picker_press(TACHYON_PICKER_SELECT_ALL, TRUE); break;
    case 2: tachyon_picker_press(TACHYON_PICKER_PASTE, TRUE); break;
    case 3: tachyon_picker_press(TACHYON_PICKER_DELETE, FALSE); break;
    default:
      tachyon_picker_press(TACHYON_PICKER_RETURN, FALSE);
      g_picker_finished = TRUE;
  }
  g_picker_action += 1;
  g_picker_settle = TACHYON_PICKER_SETTLE_TICKS;
}

static gboolean tachyon_picker_dialog_has_focus(void) {
  GtkWindow *dialog = tachyon_picker_shown_dialog();
  if (dialog != NULL) {
    g_picker_in_process = TRUE;
    if (!gtk_window_is_active(dialog)) gtk_window_present(dialog);
    return gtk_window_is_active(dialog);
  }
  return g_native_window != NULL && !gtk_window_is_active(g_native_window);
}

static void tachyon_picker_drive(void) {
  if (g_file_picker_dialog == NULL) {
    g_picker_open = FALSE;
    return;
  }
  if (!g_picker_open) {
    tachyon_picker_require(g_picker_dialogs < 3, "the host opened more dialogs than the fixture asked for");
    g_picker_open = TRUE;
    g_picker_choice = TACHYON_PICKER_CHOICES[g_picker_dialogs];
    g_picker_action = 0;
    if (g_picker_choice != NULL) gdk_clipboard_set_text(gdk_display_get_clipboard(gdk_display_get_default()), g_picker_choice);
    g_picker_dialogs += 1;
    g_picker_finished = FALSE;
    g_picker_settle = TACHYON_PICKER_SETTLE_TICKS;
    tachyon_picker_check_one_dialog();
    return;
  }
  if (g_picker_finished || !tachyon_picker_dialog_has_focus()) return;
  if (g_picker_settle > 0) {
    g_picker_settle -= 1;
    return;
  }
  if (g_picker_choice == NULL) {
    tachyon_picker_press(TACHYON_PICKER_ESCAPE, FALSE);
    g_picker_finished = TRUE;
    return;
  }
  tachyon_picker_choose();
}

static gboolean tachyon_picker_wayland_focused(void) {
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

static gboolean tachyon_picker_tick(gpointer data) {
  (void)data;
  tachyon_picker_drive();
  if (g_picker_waiting) return G_SOURCE_CONTINUE;
  switch (g_picker_step) {
    case 0: {
      const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
      if (g_strcmp0(uri, "tachyon://app/") != 0 || webkit_web_view_is_loading(g_view)) return G_SOURCE_CONTINUE;
      if (!tachyon_picker_wayland_focused()) return G_SOURCE_CONTINUE;
      tachyon_picker_check_handle_limits();
      tachyon_picker_check_lifetime();
      tachyon_picker_check_owners_and_reads();
      tachyon_picker_run("__PAGE_SCRIPT__");
      break;
    }
    case 1:
      tachyon_picker_require(g_strcmp0(g_picker_reply, "__PAGE_EXPECTED__") == 0, "unexpected page picker summary");
      tachyon_picker_run("__SERVICE_SCRIPT__");
      break;
    case 2:
      tachyon_picker_require(g_strcmp0(g_picker_reply, "__SERVICE_EXPECTED__") == 0, "unexpected Rust picker summary");
      tachyon_picker_require(g_picker_dialogs == 3 && g_picker_refused_second, "the fixture did not drive three dialogs");
      tachyon_picker_require(g_hash_table_size(g_picked_files) == 0, "released handles must be gone");
      g_free(tachyon_picker_keep(0, "notes.txt"));
      webkit_web_view_reload(g_view);
      break;
    case 3:
      if (webkit_web_view_is_loading(g_view)) return G_SOURCE_CONTINUE;
      tachyon_picker_require(g_hash_table_size(g_picked_files) == 0, "a new page must dispose every handle");
      g_application_quit(g_application_get_default());
      break;
    default: return G_SOURCE_REMOVE;
  }
  g_picker_step += 1;
  printf("step %d done after %.1f s\n", g_picker_step, (double)(g_get_monotonic_time() - g_picker_started) / G_USEC_PER_SEC);
  return G_SOURCE_CONTINUE;
}

static gboolean tachyon_picker_deadline(gpointer data) {
  (void)data;
  tachyon_picker_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_picker_started = g_get_monotonic_time();
  g_timeout_add(20, tachyon_picker_tick, NULL);
  g_timeout_add_seconds(110, tachyon_picker_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  tachyon_picker_require(status == 0 && g_picker_step == 4, "the host did not finish its run");
  printf("linux file picker passed with the %s file dialog: a JavaScript page and a Rust service chose files in it with %s key presses, read them and released them; cancel, the limits, the lifetime, other owners, the grant and a new page were checked\n",
         g_picker_in_process ? "in-process GTK" : "desktop portal", tachyon_wayland_input_wanted() ? "Wayland (Mutter RemoteDesktop)" : "X11");
  return 0;
}
