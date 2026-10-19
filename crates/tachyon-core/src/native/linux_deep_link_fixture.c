#include <stdlib.h>

static int g_link_stage = 0;
static const char *const TACHYON_LINK_STEPS[][2] = {
  {NULL, "tachyon://app/help/"},
  {"myapp://help?tab=2", "tachyon://app/help/?tab=2"},
  {"other:missing/page", "tachyon://app/"},
};

static void tachyon_link_fail(const char *message) {
  const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
  fprintf(stderr, "linux deep link fixture failed: %s (stage %d, page %s)\n", message, g_link_stage,
          uri != NULL ? uri : "none");
  exit(1);
}

static void tachyon_link_forwarded(GPid child, gint status, gpointer data) {
  (void)data;
  g_spawn_close_pid(child);
  if (!g_spawn_check_wait_status(status, NULL)) tachyon_link_fail("second instance failed");
}

static void tachyon_link_forward(const char *link) {
  gchar *self = g_file_read_link("/proc/self/exe", NULL);
  gchar *arguments[] = {self, (gchar *)link, NULL};
  gchar **environment = g_environ_setenv(g_get_environ(), "TACHYON_LINK_SECONDARY", "1", TRUE);
  GError *error = NULL;
  GPid child = 0;
  gboolean ran = g_spawn_async(NULL, arguments, environment, G_SPAWN_DO_NOT_REAP_CHILD, NULL, NULL, &child, &error);
  g_strfreev(environment);
  g_free(self);
  if (!ran) tachyon_link_fail(error != NULL ? error->message : "second instance did not start");
  g_child_watch_add(child, tachyon_link_forwarded, NULL);
}

static gboolean tachyon_link_tick(gpointer data) {
  (void)data;
  const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
  if (uri == NULL || webkit_web_view_is_loading(g_view)) return G_SOURCE_CONTINUE;
  if (g_strcmp0(uri, TACHYON_LINK_STEPS[g_link_stage][1]) != 0) return G_SOURCE_CONTINUE;
  printf("linux deep links: stage %d opened %s\n", g_link_stage, uri);
  g_link_stage++;
  if (g_link_stage < (int)(sizeof(TACHYON_LINK_STEPS) / sizeof(TACHYON_LINK_STEPS[0]))) {
    tachyon_link_forward(TACHYON_LINK_STEPS[g_link_stage][0]);
    return G_SOURCE_CONTINUE;
  }
  g_application_quit(g_application_get_default());
  return G_SOURCE_REMOVE;
}

static gboolean tachyon_link_deadline(gpointer data) {
  (void)data;
  tachyon_link_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  if (getenv("TACHYON_LINK_SECONDARY") != NULL) return tachyon_host_main(argc, argv);
  g_timeout_add(20, tachyon_link_tick, NULL);
  g_timeout_add_seconds(110, tachyon_link_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  if (status != 0 || g_link_stage != 3) tachyon_link_fail("the host did not finish its run");
  printf("linux deep links passed\n");
  return 0;
}
