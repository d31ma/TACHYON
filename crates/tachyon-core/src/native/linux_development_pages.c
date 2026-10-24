#define TACHYON_DEVELOPMENT_ORIGIN "__DEVELOPMENT_ORIGIN__"
#define TACHYON_DEVELOPMENT_LIMIT (16 * 1024 * 1024)

static SoupSession *g_development_session = NULL;

static void tachyon_development_failed(WebKitURISchemeRequest *request, const char *reason) {
  GError *error = g_error_new(G_IO_ERROR, G_IO_ERROR_FAILED, "The development server did not answer: %s", reason);
  webkit_uri_scheme_request_finish_error(request, error);
  g_error_free(error);
}

static void tachyon_development_answered(GObject *source, GAsyncResult *result, gpointer data) {
  WebKitURISchemeRequest *request = WEBKIT_URI_SCHEME_REQUEST(data);
  SoupMessage *message = soup_session_get_async_result_message(SOUP_SESSION(source), result);
  GError *error = NULL;
  GBytes *body = soup_session_send_and_read_finish(SOUP_SESSION(source), result, &error);
  if (body == NULL || message == NULL || g_bytes_get_size(body) > TACHYON_DEVELOPMENT_LIMIT) {
    tachyon_development_failed(request, error != NULL ? error->message : "the answer is larger than 16 MiB");
    g_clear_error(&error);
    if (body != NULL) g_bytes_unref(body);
    g_object_unref(request);
    return;
  }
  const char *type = soup_message_headers_get_one(soup_message_get_response_headers(message), "Content-Type");
  if (type == NULL) type = "application/octet-stream";
  GInputStream *stream = g_memory_input_stream_new_from_bytes(body);
  WebKitURISchemeResponse *response = webkit_uri_scheme_response_new(stream, (gint64)g_bytes_get_size(body));
  webkit_uri_scheme_response_set_status(response, soup_message_get_status(message), soup_message_get_reason_phrase(message));
  webkit_uri_scheme_response_set_content_type(response, type);
  SoupMessageHeaders *headers = soup_message_headers_new(SOUP_MESSAGE_HEADERS_RESPONSE);
  soup_message_headers_append(headers, "Content-Type", type);
  soup_message_headers_append(headers, "Content-Security-Policy", "__CONTENT_SECURITY_POLICY__");
  soup_message_headers_append(headers, "X-Content-Type-Options", "nosniff");
  soup_message_headers_append(headers, "Cache-Control", "no-store");
  webkit_uri_scheme_response_set_http_headers(response, headers);
  webkit_uri_scheme_request_finish_with_response(request, response);
  g_object_unref(response);
  g_object_unref(stream);
  g_bytes_unref(body);
  g_object_unref(request);
}

static gboolean tachyon_development_request(WebKitURISchemeRequest *request, const char *relative) {
  const char *query = strchr(webkit_uri_scheme_request_get_uri(request), '?');
  gchar *escaped = g_uri_escape_string(relative, "/", FALSE);
  gchar *target = g_strconcat(TACHYON_DEVELOPMENT_ORIGIN, "/", escaped, query != NULL ? query : "", NULL);
  g_free(escaped);
  SoupMessage *message = soup_message_new(SOUP_METHOD_GET, target);
  g_free(target);
  if (message == NULL) return FALSE;
  if (g_development_session == NULL) g_development_session = soup_session_new_with_options("timeout", 30, NULL);
  soup_session_send_and_read_async(g_development_session, message, G_PRIORITY_DEFAULT, NULL,
                                   tachyon_development_answered, g_object_ref(request));
  g_object_unref(message);
  return TRUE;
}
