#define TACHYON_CAPABILITY_ROUTE "/__tachyon/capability/"

typedef struct {
  WebKitURISchemeRequest *request;
  gchar *capability;
  gchar *payload;
} TachyonCapabilityRequest;

static void tachyon_capability_route_answer(WebKitURISchemeRequest *request, gchar *answer) {
  gsize length = strlen(answer);
  GInputStream *stream = g_memory_input_stream_new_from_data(answer, (gssize)length, g_free);
  WebKitURISchemeResponse *response = webkit_uri_scheme_response_new(stream, (gint64)length);
  SoupMessageHeaders *headers = soup_message_headers_new(SOUP_MESSAGE_HEADERS_RESPONSE);
  soup_message_headers_append(headers, "Content-Type", "application/json");
  soup_message_headers_append(headers, "Cache-Control", "no-store");
  soup_message_headers_append(headers, "X-Content-Type-Options", "nosniff");
  webkit_uri_scheme_response_set_content_type(response, "application/json");
  webkit_uri_scheme_response_set_http_headers(response, headers);
  webkit_uri_scheme_request_finish_with_response(request, response);
  g_object_unref(response);
  g_object_unref(stream);
}

static gboolean tachyon_capability_route_run(gpointer data) {
  TachyonCapabilityRequest *call = data;
  tachyon_capability_route_answer(call->request, tachyon_capability_page_call(call->capability, call->payload));
  g_object_unref(call->request);
  g_free(call->capability);
  g_free(call->payload);
  g_free(call);
  return G_SOURCE_REMOVE;
}

static gchar *tachyon_capability_route_body(WebKitURISchemeRequest *request) {
  GInputStream *body = webkit_uri_scheme_request_get_http_body(request);
  if (body == NULL) return NULL;
  gchar *buffer = g_malloc0(TACHYON_MESSAGE_LIMIT + 2);
  gsize count = 0;
  gboolean read = g_input_stream_read_all(body, buffer, TACHYON_MESSAGE_LIMIT + 1, &count, NULL, NULL);
  g_object_unref(body);
  if (read && count <= TACHYON_MESSAGE_LIMIT && g_utf8_validate_len(buffer, count, NULL)) return buffer;
  g_free(buffer);
  return NULL;
}

static gboolean tachyon_capability_route_allowed(WebKitURISchemeRequest *request, const gchar *capability) {
  SoupMessageHeaders *headers = webkit_uri_scheme_request_get_http_headers(request);
  const char *named = headers != NULL ? soup_message_headers_get_one(headers, "X-Tachyon-Capability") : NULL;
  return g_strcmp0(webkit_uri_scheme_request_get_http_method(request), "POST") == 0 &&
         g_str_has_prefix(capability, "filePicker.") && strlen(capability) <= 64 && g_strcmp0(named, capability) == 0 &&
         webkit_uri_scheme_request_get_web_view(request) == g_view && tachyon_trusted_view(g_view);
}

static gboolean tachyon_capability_route(WebKitURISchemeRequest *request) {
  const gchar *path = webkit_uri_scheme_request_get_path(request);
  if (path == NULL || !g_str_has_prefix(path, TACHYON_CAPABILITY_ROUTE)) return FALSE;
  const gchar *capability = path + strlen(TACHYON_CAPABILITY_ROUTE);
  gchar *payload = tachyon_capability_route_allowed(request, capability) ? tachyon_capability_route_body(request) : NULL;
  if (payload == NULL) {
    GError *error = g_error_new_literal(G_IO_ERROR, G_IO_ERROR_PERMISSION_DENIED, "Native capability calls come from the application page.");
    webkit_uri_scheme_request_finish_error(request, error);
    g_error_free(error);
    return TRUE;
  }
  TachyonCapabilityRequest *call = g_new0(TachyonCapabilityRequest, 1);
  call->request = g_object_ref(request);
  call->capability = g_strdup(capability);
  call->payload = payload;
  g_idle_add(tachyon_capability_route_run, call);
  return TRUE;
}
