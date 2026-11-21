#include <stdlib.h>

#define TACHYON_RECORDER_FOLDER "__RECORDER_FOLDER__"
#define TACHYON_RECORDER_ESCAPE 0xff1b
#define TACHYON_RECORDER_RETURN 0xff0d
#define TACHYON_RECORDER_SELECT_ALL 0x0061
#define TACHYON_RECORDER_PASTE 0x0076
#define TACHYON_RECORDER_SETTLE_TICKS 25
#define TACHYON_RECORDER_SECONDS 3
#define TACHYON_RECORDER_AUDIO_SINK "fakeaudiosink:MAX"

static const char *TACHYON_RECORDER_SETUP = "__RECORDER_SETUP__";
static const char *TACHYON_RECORDER_UNKNOWN_WRITE =
    "return await globalThis.__tachyonNativeHostCall('browserApi.recorder.write', '{\"recording\":\"nobody\",\"position\":0,\"data\":\"AA==\"}')";
static const char *TACHYON_RECORDER_BAD_OPEN =
    "return await globalThis.__tachyonNativeHostCall('browserApi.recorder.open', '{\"recording\":\"bad name\",\"fileName\":\"a\",\"format\":\"mp4\"}')";
static const char *TACHYON_RECORDER_CLOSED = "window.record('closed'); return 'started'";
static const char *TACHYON_RECORDER_MEETING = "window.record('meeting'); return 'started'";
static const char *TACHYON_RECORDER_STOP = "window.stopRecording('meeting'); return 'stopping'";
static const char *TACHYON_RECORDER_FAILED_EVENT = "return window.takeEvent('RecordingFailed')";
static const char *TACHYON_RECORDER_STARTED_EVENT = "return window.takeEvent('RecordingStarted')";
static const char *TACHYON_RECORDER_STOPPED_EVENT = "return window.takeEvent('RecordingStopped')";

static int g_recorder_step = 0;
static gint64 g_recorder_started = 0;
static gboolean g_recorder_waiting = FALSE;
static gchar *g_recorder_reply = NULL;
static gchar *g_recorder_format = NULL;
static gchar *g_recorder_file = NULL;
static const char *g_recorder_choice = NULL;
static gboolean g_recorder_open = FALSE;
static gboolean g_recorder_finished = FALSE;
static gboolean g_recorder_in_process = FALSE;
static gboolean g_recorder_refused_second = FALSE;
static int g_recorder_dialogs = 0;
static int g_recorder_action = 0;
static int g_recorder_settle = 0;
static gint64 g_recorder_recording_since = 0;

static void tachyon_recorder_fixture_fail(const char *message) {
  fprintf(stderr, "linux recorder fixture failed at step %d after %.1f s: %s (reply %s, dialogs %d, format %s)\n", g_recorder_step,
          (double)(g_get_monotonic_time() - g_recorder_started) / G_USEC_PER_SEC, message,
          g_recorder_reply != NULL ? g_recorder_reply : "none", g_recorder_dialogs, g_recorder_format != NULL ? g_recorder_format : "none");
  exit(1);
}

static void tachyon_recorder_require(gboolean valid, const char *message) {
  if (!valid) tachyon_recorder_fixture_fail(message);
}

static void tachyon_fixture_keys_require(gboolean valid, const char *message) {
  tachyon_recorder_require(valid, message);
}

static void tachyon_recorder_replied(GObject *source, GAsyncResult *result, gpointer data) {
  (void)data;
  GError *error = NULL;
  JSCValue *value = webkit_web_view_call_async_javascript_function_finish(WEBKIT_WEB_VIEW(source), result, &error);
  if (value == NULL) tachyon_recorder_fixture_fail(error != NULL ? error->message : "script failed");
  g_free(g_recorder_reply);
  g_recorder_reply = jsc_value_to_string(value);
  g_object_unref(value);
  g_recorder_waiting = FALSE;
}

static void tachyon_recorder_run(const char *script) {
  g_recorder_waiting = TRUE;
  webkit_web_view_call_async_javascript_function(g_view, script, -1, NULL, NULL, NULL, NULL, tachyon_recorder_replied, NULL);
}

static gboolean tachyon_recorder_reply_has(const char *text) {
  return g_recorder_reply != NULL && strstr(g_recorder_reply, text) != NULL;
}

static GtkWindow *tachyon_recorder_shown_dialog(void) {
  GListModel *windows = gtk_window_get_toplevels();
  GtkWindow *found = NULL;
  for (guint index = 0; found == NULL && index < g_list_model_get_n_items(windows); index += 1) {
    GtkWindow *window = g_list_model_get_item(windows, index);
    if (window != g_native_window && gtk_widget_get_visible(GTK_WIDGET(window))) found = window;
    g_object_unref(window);
  }
  return found;
}

static gboolean tachyon_recorder_dialog_has_focus(void) {
  GtkWindow *dialog = tachyon_recorder_shown_dialog();
  if (dialog == NULL) return g_native_window != NULL && !gtk_window_is_active(g_native_window);
  g_recorder_in_process = TRUE;
  if (!gtk_window_is_active(dialog)) gtk_window_present(dialog);
  return gtk_window_is_active(dialog);
}

static void tachyon_recorder_check_one_dialog(void) {
  JSCContext *context = jsc_context_new();
  JSCValue *request = jsc_value_new_from_json(context, "{\"recording\":\"second\",\"fileName\":\"b\",\"format\":\"webm\"}");
  const gchar *answer = tachyon_recorder_refusal("browserApi.recorder.open", request);
  g_object_unref(request);
  g_object_unref(context);
  tachyon_recorder_require(answer == TACHYON_RECORDER_BUSY, "a second save dialog must be refused while one is open");
  g_recorder_refused_second = TRUE;
}

static void tachyon_recorder_dialog_opened(void) {
  tachyon_recorder_require(g_recorder_dialogs < 2, "the host opened more save dialogs than the fixture asked for");
  g_recorder_open = TRUE;
  g_recorder_dialogs += 1;
  g_recorder_finished = FALSE;
  g_recorder_action = 0;
  g_recorder_settle = TACHYON_RECORDER_SETTLE_TICKS;
  if (g_recorder_choice != NULL) gdk_clipboard_set_text(gdk_display_get_clipboard(gdk_display_get_default()), g_recorder_choice);
  tachyon_recorder_check_one_dialog();
}

static void tachyon_recorder_choose(void) {
  if (g_recorder_choice == NULL) {
    tachyon_fixture_press(TACHYON_RECORDER_ESCAPE, FALSE);
    g_recorder_finished = TRUE;
    return;
  }
  switch (g_recorder_action) {
    case 0: tachyon_fixture_press(TACHYON_RECORDER_SELECT_ALL, TRUE); break;
    case 1: tachyon_fixture_press(TACHYON_RECORDER_PASTE, TRUE); break;
    default:
      tachyon_fixture_press(TACHYON_RECORDER_RETURN, FALSE);
      g_recorder_finished = TRUE;
  }
  g_recorder_action += 1;
  g_recorder_settle = TACHYON_RECORDER_SETTLE_TICKS;
}

static void tachyon_recorder_drive(void) {
  if (g_recorder_dialog == NULL) {
    g_recorder_open = FALSE;
    return;
  }
  if (!g_recorder_open) {
    tachyon_recorder_dialog_opened();
    return;
  }
  if (g_recorder_finished || !tachyon_recorder_dialog_has_focus()) return;
  if (g_recorder_settle > 0) {
    g_recorder_settle -= 1;
    return;
  }
  tachyon_recorder_choose();
}

static void tachyon_recorder_setup_done(void) {
  gchar **parts = g_strsplit(g_recorder_reply, "|", 3);
  tachyon_recorder_require(g_strv_length(parts) == 3 && g_strcmp0(parts[0], "function") == 0, "the page did not load the recorder");
  tachyon_recorder_require(g_str_has_suffix(parts[1], ",recorder"), "the page bridge list does not end with recorder");
  tachyon_recorder_require(g_strcmp0(parts[2], "mp4") == 0 || g_strcmp0(parts[2], "webm") == 0, "WebKitGTK records neither MP4 nor WebM");
  g_recorder_format = g_strdup(parts[2]);
  g_recorder_file = g_strdup_printf("Fixture.%s", g_recorder_format);
  g_strfreev(parts);
}

static const char *tachyon_recorder_event_script(void) {
  if (g_recorder_step == 4) return TACHYON_RECORDER_FAILED_EVENT;
  if (g_recorder_step == 5) return TACHYON_RECORDER_STARTED_EVENT;
  return TACHYON_RECORDER_STOPPED_EVENT;
}

static gboolean tachyon_recorder_event_arrived(void) {
  if (g_str_has_prefix(g_recorder_reply, "Recording")) return TRUE;
  tachyon_recorder_run(tachyon_recorder_event_script());
  return FALSE;
}

static guchar *tachyon_recorder_saved_bytes(gsize *length) {
  gchar *path = g_build_filename(TACHYON_RECORDER_FOLDER, g_recorder_file, NULL);
  gchar *bytes = NULL;
  gboolean read = g_file_get_contents(path, &bytes, length, NULL);
  g_free(path);
  tachyon_recorder_require(read, "the chosen file was not written");
  return (guchar *)bytes;
}

static void tachyon_recorder_check_file(void) {
  gsize length = 0;
  guchar *bytes = tachyon_recorder_saved_bytes(&length);
  gchar *size = g_strdup_printf("\"bytes\":%" G_GSIZE_FORMAT ",", length);
  tachyon_recorder_require(tachyon_recorder_reply_has(size), "RecordingStopped does not count the bytes of the file");
  tachyon_recorder_require(length > 1000, "the recording file is too small");
  gboolean mp4 = length > 8 && memcmp(bytes + 4, "ftyp", 4) == 0;
  gboolean webm = length > 4 && bytes[0] == 0x1a && bytes[1] == 0x45 && bytes[2] == 0xdf && bytes[3] == 0xa3;
  tachyon_recorder_require(g_strcmp0(g_recorder_format, "mp4") == 0 ? mp4 : webm, "the file does not start like its format");
  tachyon_recorder_require(g_hash_table_size(tachyon_recorder_files()) == 0, "a stopped recording left its file open");
  printf("saved %" G_GSIZE_FORMAT " bytes of %s through the %s save dialog\n", length, g_recorder_format,
         g_recorder_in_process ? "in-process GTK" : "desktop portal");
  g_free(size);
  g_free(bytes);
}

static gboolean tachyon_recorder_step_ready(void) {
  switch (g_recorder_step) {
    case 0: {
      const char *uri = g_view != NULL ? webkit_web_view_get_uri(g_view) : NULL;
      return g_strcmp0(uri, "tachyon://app/") == 0 && !webkit_web_view_is_loading(g_view) && tachyon_fixture_wayland_focused();
    }
    case 4: case 5: case 7: return tachyon_recorder_event_arrived();
    case 6: return g_get_monotonic_time() - g_recorder_recording_since >= TACHYON_RECORDER_SECONDS * G_USEC_PER_SEC;
    default: return TRUE;
  }
}

static void tachyon_recorder_started_check(void) {
  gchar *named = g_strdup_printf("\"fileName\":\"%s\"", g_recorder_file);
  tachyon_recorder_require(tachyon_recorder_reply_has("RecordingStarted") && tachyon_recorder_reply_has(named), "the recording did not start in the chosen file");
  g_free(named);
  g_recorder_recording_since = g_get_monotonic_time();
}

static void tachyon_recorder_stopped_check(void) {
  tachyon_recorder_require(tachyon_recorder_reply_has("RecordingStopped"), "the recording did not stop");
  tachyon_recorder_require(g_recorder_dialogs == 2 && g_recorder_refused_second, "the fixture did not drive two save dialogs");
  tachyon_recorder_check_file();
  g_application_quit(g_application_get_default());
}

static void tachyon_recorder_step(void) {
  switch (g_recorder_step) {
    case 0:
      webkit_settings_set_enable_mock_capture_devices(webkit_web_view_get_settings(g_view), TRUE);
      tachyon_recorder_run(TACHYON_RECORDER_SETUP);
      break;
    case 1: tachyon_recorder_setup_done(); tachyon_recorder_run(TACHYON_RECORDER_UNKNOWN_WRITE); break;
    case 2:
      tachyon_recorder_require(tachyon_recorder_reply_has("(unknown)"), "a write to no recording must be unknown");
      tachyon_recorder_run(TACHYON_RECORDER_BAD_OPEN);
      break;
    case 3:
      tachyon_recorder_require(tachyon_recorder_reply_has("(invalid)"), "a bad recording name must be invalid");
      g_recorder_choice = NULL;
      tachyon_recorder_run(TACHYON_RECORDER_CLOSED);
      break;
    case 4:
      tachyon_recorder_require(tachyon_recorder_reply_has("RecordingFailed") && tachyon_recorder_reply_has("\"reason\":\"cancelled\""), "a closed save dialog must fail as cancelled");
      printf("closed dialog: cancelled\n");
      g_recorder_choice = g_build_filename(TACHYON_RECORDER_FOLDER, g_recorder_file, NULL);
      tachyon_recorder_run(TACHYON_RECORDER_MEETING);
      break;
    case 5: tachyon_recorder_started_check(); break;
    case 6: tachyon_recorder_run(TACHYON_RECORDER_STOP); break;
    case 7: tachyon_recorder_stopped_check(); break;
    default: break;
  }
}

static gboolean tachyon_recorder_tick(gpointer data) {
  (void)data;
  tachyon_recorder_drive();
  if (g_recorder_waiting || g_recorder_step > 7 || !tachyon_recorder_step_ready()) return G_SOURCE_CONTINUE;
  tachyon_recorder_step();
  g_recorder_step += 1;
  printf("step %d done after %.1f s\n", g_recorder_step, (double)(g_get_monotonic_time() - g_recorder_started) / G_USEC_PER_SEC);
  return G_SOURCE_CONTINUE;
}

static gboolean tachyon_recorder_deadline(gpointer data) {
  (void)data;
  tachyon_recorder_fixture_fail("deadline");
  return G_SOURCE_REMOVE;
}

int main(int argc, char **argv) {
  g_setenv("GST_PLUGIN_FEATURE_RANK", TACHYON_RECORDER_AUDIO_SINK, FALSE);
  g_recorder_started = g_get_monotonic_time();
  g_timeout_add(20, tachyon_recorder_tick, NULL);
  g_timeout_add_seconds(110, tachyon_recorder_deadline, NULL);
  int status = tachyon_host_main(argc, argv);
  tachyon_recorder_require(status == 0 && g_recorder_step == 8, "the host did not finish its run");
  printf("linux recorder passed\n");
  return 0;
}
