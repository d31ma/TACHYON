#define TACHYON_FILE_PICKER_BYTE_LIMIT 65536
#define TACHYON_FILE_PICKER_HANDLE_LIMIT 128
#define TACHYON_FILE_PICKER_OWNER_LIMIT 8
#define TACHYON_FILE_PICKER_LIFETIME (300 * G_USEC_PER_SEC)
#define TACHYON_FILE_PICKER_DIALOG_SECONDS 30

typedef struct {
  gchar *owner;
  GFile *file;
  gint64 expiry;
} TachyonPickedFile;

typedef struct {
  gboolean waiting;
  GFile *file;
} TachyonFilePickerChoice;

static const gchar *TACHYON_FILE_PICKER_PAGE =
    "globalThis.__tachyonNativeFilePicker = true; globalThis.__tachyonNativeFilePickerRoute = true; "
    "globalThis.__tachyonNativeGrants = __NATIVE_GRANTS__;";
static gboolean g_file_picker_granted = __FILE_PICKER_GRANTED__;
static GHashTable *g_picked_files = NULL;
static GCancellable *g_file_picker_dialog = NULL;
static gchar *g_file_picker_dialog_owner = NULL;
static gint64 (*g_file_picker_clock)(void) = g_get_monotonic_time;

static void tachyon_picked_file_free(gpointer data) {
  TachyonPickedFile *picked = data;
  g_free(picked->owner);
  g_object_unref(picked->file);
  g_free(picked);
}

static GHashTable *tachyon_picked_files(void) {
  if (g_picked_files == NULL) {
    g_picked_files = g_hash_table_new_full(g_str_hash, g_str_equal, g_free, tachyon_picked_file_free);
  }
  return g_picked_files;
}

static gchar *tachyon_file_picker_status(const char *status) {
  return g_strdup_printf("{\"status\":\"%s\"}", status);
}

static void tachyon_file_picker_set(JSCValue *result, const char *name, const char *text) {
  JSCValue *value = jsc_value_new_string(jsc_value_get_context(result), text);
  jsc_value_object_set_property(result, name, value);
  g_object_unref(value);
}

static gchar *tachyon_file_picker_result(JSCContext *context, const char *status, const char *name,
                                         const char *text) {
  JSCValue *result = jsc_value_new_object(context, NULL, NULL);
  tachyon_file_picker_set(result, "status", status);
  tachyon_file_picker_set(result, name, text);
  gchar *json = jsc_value_to_json(result, 0);
  g_object_unref(result);
  return json != NULL ? json : tachyon_file_picker_status("unavailable");
}

static gchar *tachyon_file_picker_text(JSCValue *request, const char *name) {
  JSCValue *value = jsc_value_object_get_property(request, name);
  gchar *text = jsc_value_is_string(value) ? jsc_value_to_string(value) : NULL;
  g_object_unref(value);
  return text;
}

static gboolean tachyon_picked_file_expired(gpointer handle, gpointer data, gpointer now) {
  (void)handle;
  TachyonPickedFile *picked = data;
  return picked->expiry <= *(gint64 *)now;
}

static gboolean tachyon_picked_file_owned(gpointer handle, gpointer data, gpointer owner) {
  (void)handle;
  TachyonPickedFile *picked = data;
  return g_strcmp0(picked->owner, owner) == 0;
}

static void tachyon_file_picker_prune(void) {
  gint64 now = g_file_picker_clock();
  g_hash_table_foreach_remove(tachyon_picked_files(), tachyon_picked_file_expired, &now);
}

static guint tachyon_file_picker_owner_count(const gchar *owner) {
  guint count = 0;
  GHashTableIter iterator;
  gpointer data = NULL;
  g_hash_table_iter_init(&iterator, tachyon_picked_files());
  while (g_hash_table_iter_next(&iterator, NULL, &data)) {
    TachyonPickedFile *picked = data;
    if (g_strcmp0(picked->owner, owner) == 0) count += 1;
  }
  return count;
}

static void tachyon_file_picker_dispose(const gchar *owner) {
  if (g_file_picker_dialog != NULL && g_strcmp0(g_file_picker_dialog_owner, owner) == 0) {
    g_cancellable_cancel(g_file_picker_dialog);
  }
  g_hash_table_foreach_remove(tachyon_picked_files(), tachyon_picked_file_owned, (gpointer)owner);
}

static void tachyon_file_picker_dispose_all(void) {
  if (g_file_picker_dialog != NULL) g_cancellable_cancel(g_file_picker_dialog);
  g_hash_table_remove_all(tachyon_picked_files());
}

static gchar *tachyon_file_picker_keep(JSCContext *context, const gchar *owner, GFile *file) {
  gchar *handle = g_uuid_string_random();
  TachyonPickedFile *picked = g_new0(TachyonPickedFile, 1);
  picked->owner = g_strdup(owner);
  picked->file = g_object_ref(file);
  picked->expiry = g_file_picker_clock() + TACHYON_FILE_PICKER_LIFETIME;
  g_hash_table_insert(tachyon_picked_files(), g_strdup(handle), picked);
  JSCValue *result = jsc_value_new_object(context, NULL, NULL);
  gchar *name = g_file_get_basename(file);
  tachyon_file_picker_set(result, "status", "selected");
  tachyon_file_picker_set(result, "handle", handle);
  tachyon_file_picker_set(result, "name", name != NULL ? name : "");
  gchar *json = jsc_value_to_json(result, 0);
  g_object_unref(result);
  g_free(name);
  g_free(handle);
  return json != NULL ? json : tachyon_file_picker_status("unavailable");
}

static gboolean tachyon_file_picker_allowed_type(const gchar *type) {
  return g_strcmp0(type, "text/plain") == 0 || g_strcmp0(type, "application/json") == 0;
}

static gboolean tachyon_file_picker_types(JSCValue *request, GtkFileFilter **filter) {
  *filter = NULL;
  JSCValue *types = jsc_value_object_get_property(request, "types");
  gboolean valid = jsc_value_is_array(types);
  gint32 count = 0;
  if (valid) {
    JSCValue *length = jsc_value_object_get_property(types, "length");
    count = jsc_value_to_int32(length);
    g_object_unref(length);
    valid = count >= 0 && count <= 2;
  }
  for (gint32 index = 0; valid && index < count; index += 1) {
    JSCValue *type = jsc_value_object_get_property_at_index(types, (guint)index);
    gchar *text = jsc_value_is_string(type) ? jsc_value_to_string(type) : NULL;
    valid = tachyon_file_picker_allowed_type(text);
    if (valid && *filter == NULL) *filter = gtk_file_filter_new();
    if (valid) gtk_file_filter_add_mime_type(*filter, text);
    g_free(text);
    g_object_unref(type);
  }
  g_object_unref(types);
  if (!valid) g_clear_object(filter);
  return valid;
}

static void tachyon_file_picker_chosen(GObject *source, GAsyncResult *result, gpointer data) {
  TachyonFilePickerChoice *choice = data;
  choice->file = gtk_file_dialog_open_finish(GTK_FILE_DIALOG(source), result, NULL);
  choice->waiting = FALSE;
}

static gboolean tachyon_file_picker_timed_out(gpointer cancellable) {
  g_cancellable_cancel(G_CANCELLABLE(cancellable));
  return G_SOURCE_REMOVE;
}

static GtkWindow *tachyon_file_picker_parent(void) {
  GApplication *application = g_application_get_default();
  if (application == NULL || !GTK_IS_APPLICATION(application)) return NULL;
  return gtk_application_get_active_window(GTK_APPLICATION(application));
}

static GFile *tachyon_file_picker_run(GtkFileDialog *dialog, const gchar *owner) {
  GtkWindow *parent = tachyon_file_picker_parent();
  gboolean parented = parent != NULL;
  if (parented) g_object_add_weak_pointer(G_OBJECT(parent), (gpointer *)&parent);
  GCancellable *cancellable = g_cancellable_new();
  g_file_picker_dialog = cancellable;
  g_file_picker_dialog_owner = g_strdup(owner);
  GSource *timer = g_timeout_source_new_seconds(TACHYON_FILE_PICKER_DIALOG_SECONDS);
  g_source_set_callback(timer, tachyon_file_picker_timed_out, cancellable, NULL);
  g_source_attach(timer, NULL);
  TachyonFilePickerChoice choice = {TRUE, NULL};
  gtk_file_dialog_open(dialog, parent, cancellable, tachyon_file_picker_chosen, &choice);
  while (choice.waiting) {
    if (parented && parent == NULL) g_cancellable_cancel(cancellable);
    g_main_context_iteration(NULL, TRUE);
  }
  if (parent != NULL) g_object_remove_weak_pointer(G_OBJECT(parent), (gpointer *)&parent);
  g_source_destroy(timer);
  g_source_unref(timer);
  g_file_picker_dialog = NULL;
  g_clear_pointer(&g_file_picker_dialog_owner, g_free);
  gboolean cancelled = g_cancellable_is_cancelled(cancellable);
  g_object_unref(cancellable);
  if (cancelled) g_clear_object(&choice.file);
  return choice.file;
}

static gchar *tachyon_file_picker_open(JSCValue *request, const gchar *owner) {
  if (g_file_picker_dialog != NULL ||
      g_hash_table_size(tachyon_picked_files()) >= TACHYON_FILE_PICKER_HANDLE_LIMIT ||
      tachyon_file_picker_owner_count(owner) >= TACHYON_FILE_PICKER_OWNER_LIMIT) {
    return tachyon_file_picker_status("unavailable");
  }
  GtkFileFilter *filter = NULL;
  if (!tachyon_file_picker_types(request, &filter)) return tachyon_file_picker_status("invalid");
  GtkFileDialog *dialog = gtk_file_dialog_new();
  gtk_file_dialog_set_modal(dialog, TRUE);
  if (filter != NULL) {
    gtk_file_dialog_set_default_filter(dialog, filter);
    g_object_unref(filter);
  }
  GFile *file = tachyon_file_picker_run(dialog, owner);
  g_object_unref(dialog);
  if (file == NULL) return tachyon_file_picker_status("cancelled");
  gchar *answer = tachyon_file_picker_keep(jsc_value_get_context(request), owner, file);
  g_object_unref(file);
  return answer;
}

static TachyonPickedFile *tachyon_file_picker_owned(JSCValue *request, const gchar *owner) {
  gchar *handle = tachyon_file_picker_text(request, "handle");
  TachyonPickedFile *picked = handle != NULL ? g_hash_table_lookup(tachyon_picked_files(), handle) : NULL;
  g_free(handle);
  return picked != NULL && g_strcmp0(picked->owner, owner) == 0 ? picked : NULL;
}

static gchar *tachyon_file_picker_read(JSCValue *request, const gchar *owner) {
  TachyonPickedFile *picked = tachyon_file_picker_owned(request, owner);
  if (picked == NULL) return tachyon_file_picker_status("invalid");
  GFileInputStream *stream = g_file_read(picked->file, NULL, NULL);
  if (stream == NULL) return tachyon_file_picker_status("unavailable");
  gchar *buffer = g_malloc(TACHYON_FILE_PICKER_BYTE_LIMIT + 1);
  gsize count = 0;
  gboolean read = g_input_stream_read_all(G_INPUT_STREAM(stream), buffer, TACHYON_FILE_PICKER_BYTE_LIMIT + 1,
                                          &count, NULL, NULL);
  g_object_unref(stream);
  gchar *answer = NULL;
  if (!read) answer = tachyon_file_picker_status("unavailable");
  else if (count > TACHYON_FILE_PICKER_BYTE_LIMIT) answer = tachyon_file_picker_status("tooLarge");
  else if (!g_utf8_validate_len(buffer, count, NULL)) answer = tachyon_file_picker_status("invalid");
  else {
    buffer[count] = '\0';
    answer = tachyon_file_picker_result(jsc_value_get_context(request), "read", "text", buffer);
  }
  g_free(buffer);
  return answer;
}

static gchar *tachyon_file_picker_release(JSCValue *request, const gchar *owner) {
  if (tachyon_file_picker_owned(request, owner) == NULL) return tachyon_file_picker_status("invalid");
  gchar *handle = tachyon_file_picker_text(request, "handle");
  g_hash_table_remove(tachyon_picked_files(), handle);
  g_free(handle);
  return tachyon_file_picker_status("released");
}

static gchar *tachyon_file_picker_answer(const char *operation, JSCValue *request, const gchar *owner) {
  tachyon_file_picker_prune();
  if (strcmp(operation, "dispose") == 0) {
    tachyon_file_picker_dispose(owner);
    return tachyon_file_picker_status("disposed");
  }
  if (!g_file_picker_granted) {
    tachyon_file_picker_dispose(owner);
    return tachyon_file_picker_status("denied");
  }
  if (strcmp(operation, "available") == 0) return tachyon_file_picker_status("available");
  if (strcmp(operation, "open") == 0) return tachyon_file_picker_open(request, owner);
  if (strcmp(operation, "readText") == 0) return tachyon_file_picker_read(request, owner);
  if (strcmp(operation, "release") == 0) return tachyon_file_picker_release(request, owner);
  return tachyon_file_picker_status("invalid");
}

static gchar *tachyon_file_picker_call(const char *operation, JSCValue *request) {
  gchar *owner = tachyon_file_picker_text(request, "owner");
  gchar *answer = owner != NULL && g_uuid_string_is_valid(owner)
      ? tachyon_file_picker_answer(operation, request, owner)
      : tachyon_file_picker_status("invalid");
  g_free(owner);
  return answer;
}
