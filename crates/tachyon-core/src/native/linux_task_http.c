#define TACHYON_HTTP_CHUNK 16384

typedef void (*TachyonTaskDispatch)(void);
typedef void (*TachyonCompanionSetDispatch)(TachyonTaskDispatch);
typedef void (*TachyonHttpSend)(guint64, const char *, const char *, const char *, const guint8 *, gsize, guint64, gsize);
typedef void (*TachyonCompanionSetHttpSender)(TachyonHttpSend);
typedef void (*TachyonHttpAnswer)(guint64, guint32, const guint8 *, gsize, const char *);

typedef struct {
  guint64 ticket;
  gchar *method;
  gchar *url;
  gchar *headers;
  GBytes *body;
  guint64 timeout;
  gsize limit;
  guint timer;
  gboolean expired;
  GCancellable *cancel;
  SoupMessage *message;
  GInputStream *stream;
  GByteArray *received;
} TachyonHttpCall;

static gint g_resume_posted = 0;
static TachyonHttpAnswer g_http_answer = NULL;
static SoupSession *g_http_session = NULL;

static gboolean tachyon_task_resume(gpointer unused) {
  (void)unused;
  g_atomic_int_set(&g_resume_posted, 0);
  if (g_companion_invoke == NULL) return G_SOURCE_REMOVE;
  const gchar *answer = g_companion_invoke("{\"op\":\"host-resume\"}");
  if (answer != NULL && g_companion_free != NULL) g_companion_free(answer);
  return G_SOURCE_REMOVE;
}

static void tachyon_task_dispatch(void) {
  if (g_atomic_int_compare_and_exchange(&g_resume_posted, 0, 1)) g_idle_add(tachyon_task_resume, NULL);
}

static void tachyon_http_finish(TachyonHttpCall *call, guint32 status, const char *error) {
  if (call->timer != 0) g_source_remove(call->timer);
  const guint8 *body = call->received != NULL ? call->received->data : NULL;
  gsize length = call->received != NULL ? call->received->len : 0;
  if (g_http_answer != NULL) g_http_answer(call->ticket, status, body, length, error);
  g_clear_object(&call->stream);
  g_clear_object(&call->message);
  g_clear_object(&call->cancel);
  if (call->received != NULL) g_byte_array_unref(call->received);
  if (call->body != NULL) g_bytes_unref(call->body);
  g_free(call->method);
  g_free(call->url);
  g_free(call->headers);
  g_free(call);
}

static const char *tachyon_http_failure(TachyonHttpCall *call, GError *error) {
  g_clear_error(&error);
  return call->expired ? "DEADLINE" : "HTTP_NETWORK";
}

static gboolean tachyon_http_expire(gpointer data) {
  TachyonHttpCall *call = data;
  call->timer = 0;
  call->expired = TRUE;
  g_cancellable_cancel(call->cancel);
  return G_SOURCE_REMOVE;
}

static void tachyon_http_read(GObject *source, GAsyncResult *result, gpointer data) {
  TachyonHttpCall *call = data;
  GError *error = NULL;
  GBytes *chunk = g_input_stream_read_bytes_finish(G_INPUT_STREAM(source), result, &error);
  if (chunk == NULL) { tachyon_http_finish(call, 0, tachyon_http_failure(call, error)); return; }
  gsize size = 0;
  const guint8 *bytes = g_bytes_get_data(chunk, &size);
  if (size == 0) { g_bytes_unref(chunk); tachyon_http_finish(call, soup_message_get_status(call->message), NULL); return; }
  if (call->received->len + size > call->limit) { g_bytes_unref(chunk); tachyon_http_finish(call, 0, "HTTP_LIMIT"); return; }
  g_byte_array_append(call->received, bytes, size);
  g_bytes_unref(chunk);
  g_input_stream_read_bytes_async(call->stream, TACHYON_HTTP_CHUNK, G_PRIORITY_DEFAULT, call->cancel, tachyon_http_read, call);
}

static void tachyon_http_sent(GObject *source, GAsyncResult *result, gpointer data) {
  TachyonHttpCall *call = data;
  GError *error = NULL;
  call->stream = soup_session_send_finish(SOUP_SESSION(source), result, &error);
  if (call->stream == NULL) { tachyon_http_finish(call, 0, tachyon_http_failure(call, error)); return; }
  guint status = soup_message_get_status(call->message);
  if (status < 200 || status >= 300) { tachyon_http_finish(call, status, NULL); return; }
  g_input_stream_read_bytes_async(call->stream, TACHYON_HTTP_CHUNK, G_PRIORITY_DEFAULT, call->cancel, tachyon_http_read, call);
}

static void tachyon_http_headers(SoupMessage *message, const gchar *lines) {
  SoupMessageHeaders *headers = soup_message_get_request_headers(message);
  gchar **parts = g_strsplit(lines, "\n", -1);
  for (guint index = 0; parts[index] != NULL && parts[index + 1] != NULL; index += 2) {
    soup_message_headers_append(headers, parts[index], parts[index + 1]);
  }
  g_strfreev(parts);
}

static gboolean tachyon_http_begin(gpointer data) {
  TachyonHttpCall *call = data;
  if (g_http_session == NULL) g_http_session = soup_session_new();
  call->message = soup_message_new(call->method, call->url);
  if (call->message == NULL) { tachyon_http_finish(call, 0, "HTTP_REQUEST"); return G_SOURCE_REMOVE; }
  soup_message_add_flags(call->message, SOUP_MESSAGE_NO_REDIRECT);
  tachyon_http_headers(call->message, call->headers);
  if (call->body != NULL) soup_message_set_request_body_from_bytes(call->message, NULL, call->body);
  call->cancel = g_cancellable_new();
  call->received = g_byte_array_new();
  call->timer = g_timeout_add((guint)CLAMP(call->timeout, 1, G_MAXUINT), tachyon_http_expire, call);
  soup_session_send_async(g_http_session, call->message, G_PRIORITY_DEFAULT, call->cancel, tachyon_http_sent, call);
  return G_SOURCE_REMOVE;
}

static void tachyon_http_send(guint64 ticket, const char *method, const char *url, const char *headers,
                              const guint8 *body, gsize body_length, guint64 timeout, gsize limit) {
  TachyonHttpCall *call = g_new0(TachyonHttpCall, 1);
  call->ticket = ticket;
  call->method = g_strdup(method);
  call->url = g_strdup(url);
  call->headers = g_strdup(headers != NULL ? headers : "");
  call->body = body != NULL ? g_bytes_new(body, body_length) : NULL;
  call->timeout = timeout;
  call->limit = limit;
  g_idle_add(tachyon_http_begin, call);
}

static void tachyon_tasks_bind(GModule *module) {
  TachyonCompanionSetDispatch set_dispatch = NULL;
  g_module_symbol(module, "tac_native_set_dispatch", (gpointer *)&set_dispatch);
  if (set_dispatch != NULL) set_dispatch(tachyon_task_dispatch);
  TachyonCompanionSetHttpSender set_sender = NULL;
  g_module_symbol(module, "tac_native_set_http_sender", (gpointer *)&set_sender);
  g_module_symbol(module, "tac_native_http_answer", (gpointer *)&g_http_answer);
  if (set_sender != NULL && g_http_answer != NULL) set_sender(tachyon_http_send);
}
