#include <execinfo.h>
#include <signal.h>
#include <stdlib.h>
#include <unistd.h>

static gboolean g_crypto_waiting = FALSE;
static gboolean g_crypto_started = FALSE;
static gchar *g_crypto_reply = NULL;

static void tachyon_crypto_fixture_fail(const char *message) {
  fprintf(stderr, "linux crypto fixture failed: %s (reply %s)\n", message, g_crypto_reply != NULL ? g_crypto_reply : "none");
  exit(1);
}

static void tachyon_crypto_fixture_crashed(int number) {
  void *frames[64];
  int count = backtrace(frames, 64);
  fprintf(stderr, "linux crypto fixture crashed with signal %d:\n", number);
  backtrace_symbols_fd(frames, count, STDERR_FILENO);
  _exit(1);
}

static void tachyon_crypto_fixture_replied(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  JSCValue *value = webkit_web_view_call_async_javascript_function_finish(WEBKIT_WEB_VIEW(source), result, &error);
  if (value == NULL) tachyon_crypto_fixture_fail(error != NULL ? error->message : "script failed");
  g_crypto_reply = jsc_value_to_string(value);
  g_object_unref(value);
  g_crypto_waiting = FALSE;
}

static gboolean tachyon_crypto_fixture_tick(gpointer data) {
  (void)data;
  if (g_crypto_waiting) return G_SOURCE_CONTINUE;
  if (!g_crypto_started) {
    const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
    if (g_strcmp0(uri, "tachyon://app/") != 0 || webkit_web_view_is_loading(g_view)) return G_SOURCE_CONTINUE;
    g_crypto_started = TRUE;
    g_crypto_waiting = TRUE;
    webkit_web_view_call_async_javascript_function(g_view, "__CRYPTO_SCRIPT__", -1, NULL, NULL, NULL, NULL,
                                                   tachyon_crypto_fixture_replied, NULL);
    return G_SOURCE_CONTINUE;
  }
  printf("linux crypto: %s\n", g_crypto_reply);
  if (g_strcmp0(g_crypto_reply, "__CRYPTO_EXPECTED__") != 0) tachyon_crypto_fixture_fail("unexpected crypto summary");
  g_application_quit(g_application_get_default());
  return G_SOURCE_REMOVE;
}

static gboolean tachyon_crypto_fixture_deadline(gpointer data) {
  (void)data;
  tachyon_crypto_fixture_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  signal(SIGSEGV, tachyon_crypto_fixture_crashed);
  g_timeout_add(20, tachyon_crypto_fixture_tick, NULL);
  g_timeout_add_seconds(110, tachyon_crypto_fixture_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  if (status != 0 || g_crypto_reply == NULL) tachyon_crypto_fixture_fail("the host did not finish its run");
  printf("linux crypto passed\n");
  return 0;
}
