#include <stdlib.h>

static int g_fixture_step = 0;
static gint64 g_fixture_started = 0;
static gint64 g_fixture_settled = 0;

static void tachyon_fixture_fail(const char *message) {
  const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
  fprintf(stderr, "linux system back failed at step %d: %s (uri %s, route %s)\n", g_fixture_step, message,
          uri != NULL ? uri : "none", g_route != NULL ? g_route : "none");
  exit(1);
}

static gboolean tachyon_fixture_at(const char *uri, const char *route) {
  return g_view != NULL && g_strcmp0(webkit_web_view_get_uri(g_view), uri) == 0 &&
         !webkit_web_view_is_loading(g_view) && g_strcmp0(g_route, route) == 0;
}

static void tachyon_fixture_input(const char *input) {
  gchar *command = g_strdup_printf(
      "xdotool search --sync --name '%s' windowactivate --sync mousemove --window %%1 200 200 && xdotool %s",
      TACHYON_APP_NAME, input);
  if (system(command) != 0) tachyon_fixture_fail("xdotool could not reach the window");
  g_free(command);
}

static gboolean tachyon_fixture_tick(gpointer data) {
  (void)data;
  static const char *const HOME = "tachyon://app/";
  static const char *const HELP = "tachyon://app/help/";
  switch (g_fixture_step) {
    case 0: if (!tachyon_fixture_at(HOME, "/")) return G_SOURCE_CONTINUE;
      if (webkit_web_view_can_go_back(g_view)) tachyon_fixture_fail("the first route has a page to go back to");
      tachyon_menu_open_route("/help"); break;
    case 1: if (!tachyon_fixture_at(HELP, "/help")) return G_SOURCE_CONTINUE; tachyon_fixture_input("key alt+Left"); break;
    case 2: if (!tachyon_fixture_at(HOME, "/")) return G_SOURCE_CONTINUE; tachyon_fixture_input("key alt+Right"); break;
    case 3: if (!tachyon_fixture_at(HELP, "/help")) return G_SOURCE_CONTINUE; tachyon_fixture_input("click 8"); break;
    case 4: if (!tachyon_fixture_at(HOME, "/")) return G_SOURCE_CONTINUE; tachyon_fixture_input("click 9"); break;
    case 5: if (!tachyon_fixture_at(HELP, "/help")) return G_SOURCE_CONTINUE; tachyon_fixture_input("key XF86Back"); break;
    case 6: if (!tachyon_fixture_at(HOME, "/")) return G_SOURCE_CONTINUE;
      tachyon_fixture_input("key alt+Left");
      g_fixture_settled = g_get_monotonic_time() + G_USEC_PER_SEC; break;
    case 7: if (g_get_monotonic_time() < g_fixture_settled) return G_SOURCE_CONTINUE;
      if (!tachyon_fixture_at(HOME, "/")) tachyon_fixture_fail("Back at the first route left it");
      g_application_quit(G_APPLICATION(g_menu_application)); break;
    default: return G_SOURCE_REMOVE;
  }
  g_fixture_step += 1;
  printf("step %d done after %.1f s\n", g_fixture_step, (double)(g_get_monotonic_time() - g_fixture_started) / G_USEC_PER_SEC);
  return G_SOURCE_CONTINUE;
}

static gboolean tachyon_fixture_deadline(gpointer data) {
  (void)data;
  tachyon_fixture_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_timeout_add(50, tachyon_fixture_tick, NULL);
  g_fixture_started = g_get_monotonic_time();
  g_timeout_add_seconds(100, tachyon_fixture_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  if (status != 0 || g_fixture_step != 8) tachyon_fixture_fail("the host did not finish its run");
  printf("linux system back: Alt+Left, Alt+Right, mouse back and forward, the Back key and the first route passed\n");
  return 0;
}
