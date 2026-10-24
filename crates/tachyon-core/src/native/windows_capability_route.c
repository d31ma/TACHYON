#define TACHYON_WM_CAPABILITY (WM_APP + 4)
#define TACHYON_CAPABILITY_ROUTE "https://tachyon.local/__tachyon/capability/"
#define TACHYON_CAPABILITY_BODY_LIMIT 65536

typedef struct {
  ICoreWebView2Deferral *deferral;
  ICoreWebView2WebResourceRequestedEventArgs *arguments;
  char *capability;
  char *payload;
} TachyonCapabilityRequest;

static TachyonCapabilityRequest g_capability_call = {NULL, NULL, NULL, NULL};

static void tachyon_capability_route_answer(ICoreWebView2WebResourceRequestedEventArgs *arguments, const char *answer) {
  IStream *stream = answer == NULL ? NULL : SHCreateMemStream((const BYTE *)answer, (UINT)strlen(answer));
  ICoreWebView2WebResourceResponse *response = NULL;
  if (SUCCEEDED(ICoreWebView2Environment_CreateWebResourceResponse(g_environment, stream, stream == NULL ? 403 : 200,
        stream == NULL ? L"Forbidden" : L"OK",
        L"Content-Type: application/json\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n", &response)) &&
      response != NULL) {
    ICoreWebView2WebResourceRequestedEventArgs_put_Response(arguments, response);
    ICoreWebView2WebResourceResponse_Release(response);
  }
  if (stream != NULL) IStream_Release(stream);
}

static void tachyon_capability_route_finish(const char *answer) {
  if (g_capability_call.deferral == NULL) return;
  tachyon_capability_route_answer(g_capability_call.arguments, answer);
  ICoreWebView2Deferral_Complete(g_capability_call.deferral);
  ICoreWebView2Deferral_Release(g_capability_call.deferral);
  ICoreWebView2WebResourceRequestedEventArgs_Release(g_capability_call.arguments);
  free(g_capability_call.capability);
  free(g_capability_call.payload);
  memset(&g_capability_call, 0, sizeof(g_capability_call));
}

static void tachyon_capability_route_run(void) {
  if (g_capability_call.deferral == NULL) return;
  if (tachyon_page_busy()) { tachyon_capability_route_finish("{\"status\":\"unavailable\"}"); return; }
  size_t length = 0;
  unsigned char *answer = tachyon_native_capability(g_capability_call.capability, g_capability_call.payload, &length);
  tachyon_capability_route_finish(answer == NULL ? "{\"status\":\"unavailable\"}" : (const char *)answer);
  tachyon_native_capability_free(answer, length);
  tachyon_page_messages_resume();
}

static BOOL tachyon_capability_route_header(ICoreWebView2WebResourceRequest *request, const char *capability) {
  ICoreWebView2HttpRequestHeaders *headers = NULL;
  LPWSTR named = NULL;
  BOOL matches = FALSE;
  if (SUCCEEDED(ICoreWebView2WebResourceRequest_get_Headers(request, &headers)) && headers != NULL) {
    if (SUCCEEDED(ICoreWebView2HttpRequestHeaders_GetHeader(headers, L"X-Tachyon-Capability", &named)) && named != NULL) {
      char *text = tachyon_narrow(named);
      matches = text != NULL && strcmp(text, capability) == 0;
      free(text);
      CoTaskMemFree(named);
    }
    ICoreWebView2HttpRequestHeaders_Release(headers);
  }
  return matches;
}

static BOOL tachyon_capability_route_allowed(ICoreWebView2WebResourceRequest *request, const char *capability) {
  LPWSTR method = NULL;
  BOOL posted = SUCCEEDED(ICoreWebView2WebResourceRequest_get_Method(request, &method)) && method != NULL && wcscmp(method, L"POST") == 0;
  CoTaskMemFree(method);
  return posted && strncmp(capability, "filePicker.", 11) == 0 && strnlen(capability, 65) <= 64 &&
         tachyon_current_route() != NULL && tachyon_capability_route_header(request, capability);
}

static char *tachyon_capability_route_body(ICoreWebView2WebResourceRequest *request) {
  IStream *content = NULL;
  if (FAILED(ICoreWebView2WebResourceRequest_get_Content(request, &content)) || content == NULL) return NULL;
  char *buffer = calloc(TACHYON_CAPABILITY_BODY_LIMIT + 2, 1);
  ULONG total = 0, count = 0;
  while (buffer != NULL && total <= TACHYON_CAPABILITY_BODY_LIMIT &&
         SUCCEEDED(content->lpVtbl->Read(content, buffer + total, TACHYON_CAPABILITY_BODY_LIMIT + 1 - total, &count)) && count > 0) {
    total += count;
  }
  IStream_Release(content);
  if (buffer != NULL && total <= TACHYON_CAPABILITY_BODY_LIMIT) return buffer;
  free(buffer);
  return NULL;
}

static BOOL tachyon_capability_route(ICoreWebView2WebResourceRequest *request, ICoreWebView2WebResourceRequestedEventArgs *arguments, const char *uri) {
  size_t prefix = strlen(TACHYON_CAPABILITY_ROUTE);
  if (uri == NULL || strncmp(uri, TACHYON_CAPABILITY_ROUTE, prefix) != 0) return FALSE;
  const char *capability = uri + prefix;
  char *payload = g_capability_call.deferral == NULL && g_window != NULL && tachyon_capability_route_allowed(request, capability)
      ? tachyon_capability_route_body(request) : NULL;
  ICoreWebView2Deferral *deferral = NULL;
  char *name = payload == NULL ? NULL : _strdup(capability);
  if (name == NULL || FAILED(ICoreWebView2WebResourceRequestedEventArgs_GetDeferral(arguments, &deferral)) || deferral == NULL) {
    free(payload); free(name);
    tachyon_capability_route_answer(arguments, NULL);
    return TRUE;
  }
  ICoreWebView2WebResourceRequestedEventArgs_AddRef(arguments);
  g_capability_call.deferral = deferral;
  g_capability_call.arguments = arguments;
  g_capability_call.capability = name;
  g_capability_call.payload = payload;
  PostMessageW(g_window, TACHYON_WM_CAPABILITY, 0, 0);
  return TRUE;
}
