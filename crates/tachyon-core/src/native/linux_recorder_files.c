#include <errno.h>

#define TACHYON_RECORDER_CHUNK_LIMIT 49152
#define TACHYON_RECORDER_FILE_NAME_LIMIT 128
#define TACHYON_RECORDER_NAME_LIMIT 64
#define TACHYON_RECORDER_POSITION_LIMIT 9007199254740991.0

typedef struct {
  WebKitScriptMessageReply *reply;
  JSCContext *context;
  gchar *recording;
} TachyonRecorderOpening;

typedef void (*TachyonRecorderAsk)(GtkFileDialog *dialog, GCancellable *cancellable, TachyonRecorderOpening *opening);

static const gchar *TACHYON_RECORDER_NOT_GRANTED =
    "{\"error\":\"The recorder needs \\\"files\\\" and \\\"microphone\\\", \\\"camera\\\" or \\\"screen\\\" in tac.browserApis in client/manifest.json (denied).\"}";
static const gchar *TACHYON_RECORDER_INVALID = "{\"error\":\"A recording call has a bad argument (invalid).\"}";
static const gchar *TACHYON_RECORDER_UNKNOWN = "{\"error\":\"There is no such recording (unknown).\"}";
static const gchar *TACHYON_RECORDER_BUSY = "{\"error\":\"A save dialog is already open (invalid).\"}";
static const gchar *TACHYON_RECORDER_CANCELLED = "{\"error\":\"The save dialog was closed (cancelled).\"}";
static const gchar *TACHYON_RECORDER_DENIED = "{\"error\":\"The recording file could not be made there (denied).\"}";
static const gchar *TACHYON_RECORDER_FAILED = "{\"error\":\"The recording could not be written (failed).\"}";
static const gchar *TACHYON_RECORDER_FULL = "{\"error\":\"There is no disk space left for the recording (full).\"}";
static const gchar *TACHYON_RECORDER_DONE = "{\"value\":null}";
static const gchar *TACHYON_RECORDER_NAME_LETTERS =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_.-";
static const gchar *TACHYON_RECORDER_UNSAFE_LETTERS = "/\\:*?\"<>|";
static const gchar *TACHYON_RECORDER_BASE64_LETTERS =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
static const gchar *const TACHYON_RECORDER_FORMATS[] = {"mp4", "webm"};

static GHashTable *g_recorder_files = NULL;
static GCancellable *g_recorder_dialog = NULL;

static void tachyon_recorder_close_file(gpointer data) {
  close(GPOINTER_TO_INT(data));
}

static GHashTable *tachyon_recorder_files(void) {
  if (g_recorder_files == NULL) {
    g_recorder_files = g_hash_table_new_full(g_str_hash, g_str_equal, g_free, tachyon_recorder_close_file);
  }
  return g_recorder_files;
}

static void tachyon_recorder_dispose_all(void) {
  if (g_recorder_dialog != NULL) g_cancellable_cancel(g_recorder_dialog);
  g_hash_table_remove_all(tachyon_recorder_files());
}

static gboolean tachyon_recorder_granted(void) {
  return tachyon_browser_api_granted("files") && tachyon_media_granted();
}

static gchar *tachyon_recorder_text(JSCValue *request, const char *name) {
  JSCValue *value = jsc_value_object_get_property(request, name);
  gchar *text = jsc_value_is_string(value) ? jsc_value_to_string(value) : NULL;
  g_object_unref(value);
  return text;
}

static gboolean tachyon_recorder_letters_only(const gchar *text, const gchar *letters) {
  return strspn(text, letters) == strlen(text);
}

static gboolean tachyon_recorder_safe_letter(gunichar letter) {
  if (letter >= 128) return TRUE;
  return !g_ascii_iscntrl((gchar)letter) && strchr(TACHYON_RECORDER_UNSAFE_LETTERS, (int)letter) == NULL;
}

static gboolean tachyon_recorder_safe_letters(const gchar *text) {
  for (const gchar *place = text; *place != '\0'; place = g_utf8_next_char(place)) {
    if (!tachyon_recorder_safe_letter(g_utf8_get_char(place))) return FALSE;
  }
  return TRUE;
}

static gboolean tachyon_recorder_known_format(const gchar *format) {
  for (gsize index = 0; index < G_N_ELEMENTS(TACHYON_RECORDER_FORMATS); index++) {
    if (g_strcmp0(format, TACHYON_RECORDER_FORMATS[index]) == 0) return TRUE;
  }
  return FALSE;
}

static gchar *tachyon_recorder_name(JSCValue *request) {
  gchar *name = tachyon_recorder_text(request, "recording");
  gboolean valid = name != NULL && name[0] != '\0' && strlen(name) <= TACHYON_RECORDER_NAME_LIMIT &&
                   tachyon_recorder_letters_only(name, TACHYON_RECORDER_NAME_LETTERS);
  if (!valid) g_clear_pointer(&name, g_free);
  return name;
}

static gchar *tachyon_recorder_file_name(JSCValue *request) {
  gchar *name = tachyon_recorder_text(request, "fileName");
  gboolean valid = name != NULL && name[0] != '\0' && g_utf8_validate(name, -1, NULL) &&
                   g_utf8_strlen(name, -1) <= TACHYON_RECORDER_FILE_NAME_LIMIT && tachyon_recorder_safe_letters(name);
  if (!valid) g_clear_pointer(&name, g_free);
  return name;
}

static gchar *tachyon_recorder_format(JSCValue *request) {
  gchar *format = tachyon_recorder_text(request, "format");
  if (!tachyon_recorder_known_format(format)) g_clear_pointer(&format, g_free);
  return format;
}

static gboolean tachyon_recorder_position(JSCValue *request, off_t *position) {
  JSCValue *value = jsc_value_object_get_property(request, "position");
  gboolean number = jsc_value_is_number(value);
  gdouble place = number ? jsc_value_to_double(value) : -1.0;
  g_object_unref(value);
  if (!(place >= 0.0 && place <= TACHYON_RECORDER_POSITION_LIMIT)) return FALSE;
  *position = (off_t)place;
  return place == (gdouble)*position;
}

static gboolean tachyon_recorder_base64_text(const gchar *text) {
  gsize length = strlen(text);
  gsize letters = strspn(text, TACHYON_RECORDER_BASE64_LETTERS);
  gsize padding = length - letters;
  return length % 4 == 0 && padding <= 2 && strspn(text + letters, "=") == padding;
}

static guchar *tachyon_recorder_chunk(JSCValue *request, gsize *length) {
  gchar *text = tachyon_recorder_text(request, "data");
  guchar *data = text != NULL && tachyon_recorder_base64_text(text) ? g_base64_decode(text, length) : NULL;
  g_free(text);
  if (data != NULL && (*length == 0 || *length > TACHYON_RECORDER_CHUNK_LIMIT)) g_clear_pointer(&data, g_free);
  return data;
}

static const gchar *tachyon_recorder_write_failure(int number) {
  return number == ENOSPC || number == EDQUOT ? TACHYON_RECORDER_FULL : TACHYON_RECORDER_FAILED;
}

static gboolean tachyon_recorder_write_all(int file, const guchar *data, gsize length, off_t position) {
  gsize written = 0;
  while (written < length) {
    ssize_t count = pwrite(file, data + written, length - written, position + (off_t)written);
    if (count < 0 && errno == EINTR) continue;
    if (count <= 0) return FALSE;
    written += (gsize)count;
  }
  return TRUE;
}

static const gchar *tachyon_recorder_write(JSCValue *request) {
  gchar *name = tachyon_recorder_name(request);
  gpointer file = name != NULL ? g_hash_table_lookup(tachyon_recorder_files(), name) : NULL;
  g_free(name);
  if (file == NULL) return TACHYON_RECORDER_UNKNOWN;
  off_t position = 0;
  gsize length = 0;
  guchar *data = tachyon_recorder_position(request, &position) ? tachyon_recorder_chunk(request, &length) : NULL;
  if (data == NULL) return TACHYON_RECORDER_INVALID;
  gboolean written = tachyon_recorder_write_all(GPOINTER_TO_INT(file), data, length, position);
  int number = errno;
  g_free(data);
  return written ? TACHYON_RECORDER_DONE : tachyon_recorder_write_failure(number);
}

static const gchar *tachyon_recorder_close(JSCValue *request) {
  gchar *name = tachyon_recorder_name(request);
  gpointer key = NULL;
  gpointer file = NULL;
  gboolean found = name != NULL && g_hash_table_steal_extended(tachyon_recorder_files(), name, &key, &file);
  g_free(name);
  g_free(key);
  if (!found) return TACHYON_RECORDER_UNKNOWN;
  gboolean saved = fsync(GPOINTER_TO_INT(file)) == 0;
  int number = errno;
  saved = close(GPOINTER_TO_INT(file)) == 0 && saved;
  return saved ? TACHYON_RECORDER_DONE : tachyon_recorder_write_failure(number);
}

static gchar *tachyon_recorder_opened(JSCContext *context, const gchar *path) {
  gchar *name = g_path_get_basename(path);
  JSCValue *answer = jsc_value_new_object(context, NULL, NULL);
  JSCValue *value = jsc_value_new_object(context, NULL, NULL);
  JSCValue *text = jsc_value_new_string(context, name);
  jsc_value_object_set_property(value, "fileName", text);
  jsc_value_object_set_property(answer, "value", value);
  gchar *json = jsc_value_to_json(answer, 0);
  g_object_unref(text);
  g_object_unref(value);
  g_object_unref(answer);
  g_free(name);
  return json;
}

static gchar *tachyon_recorder_create(TachyonRecorderOpening *opening, const gchar *path) {
  if (path == NULL) return g_strdup(TACHYON_RECORDER_DENIED);
  int file = open(path, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
  if (file < 0) return g_strdup(TACHYON_RECORDER_DENIED);
  g_hash_table_replace(tachyon_recorder_files(), g_strdup(opening->recording), GINT_TO_POINTER(file));
  gchar *answer = tachyon_recorder_opened(opening->context, path);
  return answer != NULL ? answer : g_strdup(TACHYON_RECORDER_FAILED);
}

static gchar *tachyon_recorder_chosen_answer(TachyonRecorderOpening *opening, GFile *file) {
  if (file == NULL) return g_strdup(TACHYON_RECORDER_CANCELLED);
  gchar *path = g_file_get_path(file);
  gchar *answer = tachyon_recorder_create(opening, path);
  g_free(path);
  return answer;
}

static void tachyon_recorder_reply(WebKitScriptMessageReply *reply, JSCContext *context, const gchar *answer) {
  JSCValue *result = jsc_value_new_string(context, answer);
  webkit_script_message_reply_return_value(reply, result);
  g_object_unref(result);
}

static void tachyon_recorder_chosen(TachyonRecorderOpening *opening, GFile *file) {
  gchar *answer = tachyon_recorder_chosen_answer(opening, file);
  tachyon_recorder_reply(opening->reply, opening->context, answer);
  g_clear_object(&g_recorder_dialog);
  webkit_script_message_reply_unref(opening->reply);
  g_object_unref(opening->context);
  g_free(opening->recording);
  g_free(opening);
  g_free(answer);
}

static void tachyon_recorder_saved(GObject *source, GAsyncResult *result, gpointer data) {
  GFile *file = gtk_file_dialog_save_finish(GTK_FILE_DIALOG(source), result, NULL);
  tachyon_recorder_chosen(data, file);
  g_clear_object(&file);
}

static void tachyon_recorder_ask_person(GtkFileDialog *dialog, GCancellable *cancellable, TachyonRecorderOpening *opening) {
  gtk_file_dialog_save(dialog, tachyon_file_picker_parent(), cancellable, tachyon_recorder_saved, opening);
}

static TachyonRecorderAsk g_recorder_ask = tachyon_recorder_ask_person;

static void tachyon_recorder_start_in_videos(GtkFileDialog *dialog) {
  const gchar *videos = g_get_user_special_dir(G_USER_DIRECTORY_VIDEOS);
  if (videos == NULL || !g_file_test(videos, G_FILE_TEST_IS_DIR)) return;
  GFile *folder = g_file_new_for_path(videos);
  gtk_file_dialog_set_initial_folder(dialog, folder);
  g_object_unref(folder);
}

static GtkFileDialog *tachyon_recorder_dialog(const gchar *file_name, const gchar *format) {
  GtkFileDialog *dialog = gtk_file_dialog_new();
  GtkFileFilter *filter = gtk_file_filter_new();
  GListStore *filters = g_list_store_new(GTK_TYPE_FILE_FILTER);
  gtk_file_filter_set_name(filter, format);
  gtk_file_filter_add_suffix(filter, format);
  g_list_store_append(filters, filter);
  gtk_file_dialog_set_modal(dialog, TRUE);
  tachyon_recorder_start_in_videos(dialog);
  gtk_file_dialog_set_initial_name(dialog, file_name);
  gtk_file_dialog_set_filters(dialog, G_LIST_MODEL(filters));
  gtk_file_dialog_set_default_filter(dialog, filter);
  g_object_unref(filters);
  g_object_unref(filter);
  return dialog;
}

static gboolean tachyon_recorder_open(JSCValue *request, WebKitScriptMessageReply *reply) {
  gchar *recording = tachyon_recorder_name(request);
  gchar *file_name = tachyon_recorder_file_name(request);
  gchar *format = tachyon_recorder_format(request);
  gboolean valid = recording != NULL && file_name != NULL && format != NULL && g_recorder_dialog == NULL;
  if (valid) {
    TachyonRecorderOpening *opening = g_new0(TachyonRecorderOpening, 1);
    opening->reply = webkit_script_message_reply_ref(reply);
    opening->context = g_object_ref(jsc_value_get_context(request));
    opening->recording = g_steal_pointer(&recording);
    g_recorder_dialog = g_cancellable_new();
    GtkFileDialog *dialog = tachyon_recorder_dialog(file_name, format);
    g_recorder_ask(dialog, g_recorder_dialog, opening);
    g_object_unref(dialog);
  }
  g_free(recording);
  g_free(file_name);
  g_free(format);
  return valid;
}

static const gchar *tachyon_recorder_refusal(const gchar *capability, JSCValue *request) {
  if (!tachyon_recorder_granted()) return TACHYON_RECORDER_NOT_GRANTED;
  if (request == NULL || !jsc_value_is_object(request)) return TACHYON_RECORDER_INVALID;
  if (g_strcmp0(capability, "browserApi.recorder.open") == 0 && g_recorder_dialog != NULL) return TACHYON_RECORDER_BUSY;
  return NULL;
}

static const gchar *tachyon_recorder_answer(const gchar *capability, JSCValue *request, WebKitScriptMessageReply *reply) {
  if (g_strcmp0(capability, "browserApi.recorder.write") == 0) return tachyon_recorder_write(request);
  if (g_strcmp0(capability, "browserApi.recorder.close") == 0) return tachyon_recorder_close(request);
  if (g_strcmp0(capability, "browserApi.recorder.open") == 0) return tachyon_recorder_open(request, reply) ? NULL : TACHYON_RECORDER_INVALID;
  return TACHYON_RECORDER_INVALID;
}

static void tachyon_recorder_start(const gchar *capability, const gchar *payload, JSCValue *value, WebKitScriptMessageReply *reply) {
  JSCContext *context = jsc_value_get_context(value);
  JSCValue *request = jsc_value_new_from_json(context, payload);
  const gchar *answer = tachyon_recorder_refusal(capability, request);
  if (answer == NULL) answer = tachyon_recorder_answer(capability, request, reply);
  if (answer != NULL) tachyon_recorder_reply(reply, context, answer);
  g_clear_object(&request);
}
