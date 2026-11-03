#include <winhttp.h>
#pragma comment(lib, "winhttp.lib")
#define TACHYON_DEVELOPMENT_ORIGIN "__DEVELOPMENT_ORIGIN__"
#define TACHYON_DEVELOPMENT_LIMIT (16 * 1024 * 1024)

static HINTERNET g_development_session = NULL;

static BOOL tachyon_development_answer(ICoreWebView2WebResourceRequestedEventArgs *arguments, IStream *stream, DWORD status, const wchar_t *type) {
  wchar_t headers[__RESPONSE_HEADER_CAPACITY__ + 512];
  _snwprintf_s(headers, sizeof(headers) / sizeof(headers[0]), _TRUNCATE,
    L"Content-Type: %s\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\nContent-Security-Policy: __CONTENT_SECURITY_POLICY__\r\n", type);
  ICoreWebView2WebResourceResponse *response = NULL;
  if (SUCCEEDED(ICoreWebView2Environment_CreateWebResourceResponse(g_environment, stream, (int)status, status < 400 ? L"OK" : L"Error", headers, &response)) && response != NULL) {
    ICoreWebView2WebResourceRequestedEventArgs_put_Response(arguments, response); ICoreWebView2WebResourceResponse_Release(response);
  }
  if (stream != NULL) IStream_Release(stream);
  return TRUE;
}

static BOOL tachyon_development_failed(ICoreWebView2WebResourceRequestedEventArgs *arguments, const char *reason) {
  return tachyon_development_answer(arguments, SHCreateMemStream((const BYTE *)reason, (UINT)strlen(reason)), 502, L"text/plain; charset=utf-8");
}

static BOOL tachyon_development_response(ICoreWebView2WebResourceRequestedEventArgs *arguments, const char *uri, const char *relative) {
  const char *query = strchr(uri, '?');
  size_t length = strlen(TACHYON_DEVELOPMENT_ORIGIN) + strlen(relative) + (query != NULL ? strlen(query) : 0) + 2;
  char *target = malloc(length);
  if (target == NULL) return FALSE;
  snprintf(target, length, "%s/%s%s", TACHYON_DEVELOPMENT_ORIGIN, relative, query != NULL ? query : "");
  wchar_t *address = tachyon_widen(target);
  free(target);
  if (address == NULL) return FALSE;
  URL_COMPONENTS parts; ZeroMemory(&parts, sizeof(parts)); parts.dwStructSize = sizeof(parts);
  wchar_t host[256], path[TACHYON_PATH_LIMIT * 2];
  parts.lpszHostName = host; parts.dwHostNameLength = sizeof(host) / sizeof(host[0]);
  parts.lpszUrlPath = path; parts.dwUrlPathLength = sizeof(path) / sizeof(path[0]);
  BOOL cracked = WinHttpCrackUrl(address, 0, ICU_ESCAPE, &parts);
  free(address);
  if (!cracked) return tachyon_development_failed(arguments, "The development address is invalid.");
  if (g_development_session == NULL) {
    g_development_session = WinHttpOpen(L"Tachyon", WINHTTP_ACCESS_TYPE_NO_PROXY, WINHTTP_NO_PROXY_NAME, WINHTTP_NO_PROXY_BYPASS, 0);
    if (g_development_session != NULL) WinHttpSetTimeouts(g_development_session, 30000, 30000, 30000, 30000);
  }
  HINTERNET connection = g_development_session == NULL ? NULL : WinHttpConnect(g_development_session, host, parts.nPort, 0);
  HINTERNET request = connection == NULL ? NULL : WinHttpOpenRequest(connection, L"GET", path, NULL, WINHTTP_NO_REFERER, WINHTTP_DEFAULT_ACCEPT_TYPES, 0);
  DWORD disabled = WINHTTP_DISABLE_REDIRECTS;
  if (request != NULL) WinHttpSetOption(request, WINHTTP_OPTION_DISABLE_FEATURE, &disabled, sizeof(disabled));
  BOOL sent = request != NULL && WinHttpSendRequest(request, WINHTTP_NO_ADDITIONAL_HEADERS, 0, WINHTTP_NO_REQUEST_DATA, 0, 0, 0) && WinHttpReceiveResponse(request, NULL);
  DWORD status = 0, size = sizeof(status);
  if (sent) sent = WinHttpQueryHeaders(request, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_HEADER_NAME_BY_INDEX, &status, &size, WINHTTP_NO_HEADER_INDEX);
  wchar_t type[256] = L"application/octet-stream";
  DWORD type_size = sizeof(type);
  if (sent && !WinHttpQueryHeaders(request, WINHTTP_QUERY_CONTENT_TYPE, WINHTTP_HEADER_NAME_BY_INDEX, type, &type_size, WINHTTP_NO_HEADER_INDEX)) wcscpy_s(type, sizeof(type) / sizeof(type[0]), L"application/octet-stream");
  BYTE *body = NULL; DWORD total = 0, available = 0;
  BOOL complete = sent && (status < 300 || status >= 400);
  while (sent && complete && WinHttpQueryDataAvailable(request, &available) && available > 0) {
    if (total + available > TACHYON_DEVELOPMENT_LIMIT) { complete = FALSE; break; }
    BYTE *grown = realloc(body, total + available);
    if (grown == NULL) { complete = FALSE; break; }
    body = grown;
    DWORD read = 0;
    if (!WinHttpReadData(request, body + total, available, &read)) { complete = FALSE; break; }
    total += read;
  }
  if (request != NULL) WinHttpCloseHandle(request);
  if (connection != NULL) WinHttpCloseHandle(connection);
  if (!sent || !complete) {
    free(body);
    return tachyon_development_failed(arguments, sent ? "The development server redirected or sent more than 16 MiB." : "The development server did not answer.");
  }
  IStream *stream = SHCreateMemStream(body, total);
  free(body);
  return tachyon_development_answer(arguments, stream, status, type);
}
