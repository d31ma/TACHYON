#include <stdlib.h>

typedef gint64 (*TachyonFixtureCount)(const char *);

static TachyonFixtureCount g_fixture_count = NULL;
static int g_fixture_step = 0;
static gboolean g_fixture_waiting = FALSE;
static gchar *g_fixture_reply = NULL;
static const char *g_fixture_owner = "00000000-0000-4000-8000-000000000021";

static gboolean g_fixture_watching = FALSE;

static void tachyon_fixture_fail(const char *message) {
  const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
  fprintf(stderr, "linux host fixture failed at step %d: %s (view %s, uri %s, loading %d)\n", g_fixture_step, message,
          g_view != NULL ? "present" : "missing", uri != NULL ? uri : "none", g_view != NULL && webkit_web_view_is_loading(g_view));
  exit(1);
}

static void tachyon_fixture_terminated(WebKitWebView *view, WebKitWebProcessTerminationReason reason, gpointer data) {
  (void)view;
  (void)data;
  fprintf(stderr, "web process terminated: %d\n", (int)reason);
}

static gboolean tachyon_fixture_load_failed(WebKitWebView *view, WebKitLoadEvent event, gchar *uri, GError *error, gpointer data) {
  (void)view;
  (void)event;
  (void)data;
  fprintf(stderr, "load failed for %s: %s\n", uri, error != NULL ? error->message : "unknown");
  return FALSE;
}

static void tachyon_fixture_watch(void) {
  if (g_fixture_watching || g_view == NULL) return;
  g_fixture_watching = TRUE;
  g_signal_connect(g_view, "web-process-terminated", G_CALLBACK(tachyon_fixture_terminated), NULL);
  g_signal_connect(g_view, "load-failed", G_CALLBACK(tachyon_fixture_load_failed), NULL);
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

static gchar *tachyon_fixture_request(const char *operation, const char *name) {
  gchar *payload = g_strdup_printf(
      "{\"route\":\"/\",\"service\":\"section\",\"owner\":\"%s\",\"op\":\"%s\",\"name\":\"%s\",\"args\":[]}",
      g_fixture_owner, operation, name);
  gchar *answer = tachyon_companion_invoke(payload);
  g_free(payload);
  return answer;
}

static gboolean tachyon_fixture_answer(const char *operation, const char *name, const char *expected) {
  gchar *answer = tachyon_fixture_request(operation, name);
  gboolean matches = g_strcmp0(answer, expected) == 0;
  if (!matches) fprintf(stderr, "answer for %s %s: %s\n", operation, name, answer);
  g_free(answer);
  return matches;
}

static gboolean tachyon_fixture_enabled(const char *name) {
  return g_action_group_get_action_enabled(G_ACTION_GROUP(g_menu_application), name);
}

static gboolean tachyon_fixture_accelerator(const char *action, const char *expected) {
  gchar **accelerators = gtk_application_get_accels_for_action(g_menu_application, action);
  gboolean matches = accelerators != NULL && g_strcmp0(accelerators[0], expected) == 0;
  g_strfreev(accelerators);
  return matches;
}

static gboolean tachyon_fixture_loaded(const char *expected) {
  const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
  return g_strcmp0(uri, expected) == 0 && !webkit_web_view_is_loading(g_view);
}

static void tachyon_fixture_replied(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  JSCValue *value = webkit_web_view_call_async_javascript_function_finish(WEBKIT_WEB_VIEW(source), result, &error);
  if (value == NULL) tachyon_fixture_fail(error != NULL ? error->message : "script failed");
  g_free(g_fixture_reply);
  g_fixture_reply = jsc_value_to_string(value);
  g_object_unref(value);
  g_fixture_waiting = FALSE;
}

static void tachyon_fixture_script(const char *body) {
  g_fixture_waiting = TRUE;
  webkit_web_view_call_async_javascript_function(g_view, body, -1, NULL, NULL, NULL, NULL, tachyon_fixture_replied, NULL);
}

static void tachyon_fixture_home(void) {
  tachyon_fixture_require(tachyon_fixture_count("created") == 1, "the application service did not start once at launch");
  GMenuModel *bar = gtk_application_get_menubar(g_menu_application);
  tachyon_fixture_require(bar != NULL && g_menu_model_get_n_items(bar) == 3, "menubar does not have File, View and Help");
  tachyon_fixture_require(tachyon_fixture_accelerator("app.menu-1", "<Primary>1"), "Command-1 is not Ctrl+1");
  tachyon_fixture_require(tachyon_fixture_accelerator("app.menu-2", "<Primary><Alt>g"), "Command-Option-G is not Ctrl+Alt+G");
  tachyon_fixture_require(tachyon_fixture_accelerator("app.close-window", "<Primary>w"), "Close Window is not Ctrl+W");
  tachyon_fixture_require(tachyon_fixture_accelerator("app.quit", "<Primary>q"), "Quit is not Ctrl+Q");
  tachyon_fixture_require(!tachyon_fixture_enabled("menu-1"), "page action is enabled without a recipient");
  tachyon_fixture_require(tachyon_fixture_enabled("menu-0"), "application action is disabled");
  gchar *script = g_strdup_printf(
      "globalThis.__menuMarker = 41; return await window.webkit.messageHandlers.tachyon.postMessage({capability: 'companion.invoke', "
      "payload: JSON.stringify({route: '/', service: 'section', owner: '%s', op: 'init', name: '', args: []})})",
      g_fixture_owner);
  tachyon_fixture_script(script);
  g_free(script);
}

static void tachyon_fixture_action(void) {
  tachyon_fixture_require(g_fixture_reply != NULL && strstr(g_fixture_reply, "error") == NULL, "page service init failed through the page bridge");
  tachyon_fixture_require(tachyon_fixture_enabled("menu-1"), "page action stayed disabled after the page bound its listener");
  g_free(tachyon_fixture_request("call", "visit"));
  gchar *uri = g_strdup(webkit_web_view_get_uri(g_view));
  g_action_group_activate_action(G_ACTION_GROUP(g_menu_application), "menu-1", NULL);
  tachyon_fixture_require(tachyon_fixture_count("forwarded") == 1 && tachyon_fixture_count("chosen") == 1, "menu action did not reach the page service");
  tachyon_fixture_require(tachyon_fixture_answer("get", "section", "{\"value\":\"running\"}"), "page service did not receive the section");
  tachyon_fixture_require(tachyon_fixture_answer("get", "visits", "{\"value\":1}"), "menu action reset page state");
  tachyon_fixture_require(g_strcmp0(uri, webkit_web_view_get_uri(g_view)) == 0 && !webkit_web_view_is_loading(g_view), "menu action navigated");
  g_free(uri);
  tachyon_fixture_script("return String(globalThis.__menuMarker);");
}

static void tachyon_fixture_help(void) {
  tachyon_fixture_require(g_strcmp0(g_fixture_reply, "41") == 0, "menu action reloaded the document");
  g_action_group_activate_action(G_ACTION_GROUP(g_menu_application), "menu-2", NULL);
}

static void tachyon_fixture_on_help(void) {
  tachyon_fixture_require(!tachyon_fixture_enabled("menu-1"), "page action stayed enabled without a recipient");
  tachyon_menu_activate(NULL, NULL, (gpointer)&TACHYON_MENU_ITEMS[1]);
  tachyon_fixture_require(tachyon_fixture_count("forwarded") == 1 && tachyon_fixture_count("refused") == 0, "a stale action called the application service");
  g_action_group_activate_action(G_ACTION_GROUP(g_menu_application), "menu-0", NULL);
  tachyon_fixture_require(tachyon_fixture_count("refreshed") == 1, "application action did not run");
  tachyon_fixture_require(tachyon_fixture_count("stopped") == 0, "Help navigation stopped the application service");
  tachyon_menu_open_route("/");
}

static void tachyon_fixture_on_return(void) {
  tachyon_fixture_require(tachyon_fixture_count("created") == 1, "returning Home created another application service");
  tachyon_fixture_require(tachyon_fixture_count("stopped") == 0, "returning Home stopped the application service");
  g_action_group_activate_action(G_ACTION_GROUP(g_menu_application), "close-window", NULL);
}

static gboolean tachyon_fixture_tick(gpointer data) {
  (void)data;
  tachyon_fixture_watch();
  if (g_fixture_waiting) return G_SOURCE_CONTINUE;
  switch (g_fixture_step) {
    case 0: if (!tachyon_fixture_loaded("tachyon://app/")) return G_SOURCE_CONTINUE; tachyon_fixture_home(); break;
    case 1: tachyon_fixture_action(); break;
    case 2: tachyon_fixture_help(); break;
    case 3: if (!tachyon_fixture_loaded("tachyon://app/help/")) return G_SOURCE_CONTINUE; tachyon_fixture_on_help(); break;
    case 4: if (!tachyon_fixture_loaded("tachyon://app/")) return G_SOURCE_CONTINUE; tachyon_fixture_on_return(); break;
    default: return G_SOURCE_REMOVE;
  }
  g_fixture_step += 1;
  return G_SOURCE_CONTINUE;
}

static gboolean tachyon_fixture_deadline(gpointer data) {
  (void)data;
  tachyon_fixture_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_timeout_add(20, tachyon_fixture_tick, NULL);
  g_timeout_add_seconds(30, tachyon_fixture_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  tachyon_fixture_require(status == 0 && g_fixture_step == 5, "the host did not finish its run");
  tachyon_fixture_require(tachyon_fixture_count("stopped") == 1, "Close Window did not stop the application service once");
  tachyon_application_stop();
  tachyon_fixture_require(tachyon_fixture_count("stopped") == 1, "a second stop ran the cleanup again");
  printf("linux host: launch, menu action, disabled recipient, Help and Home, Close Window and one stop passed\n");
  return 0;
}
