#define TACHYON_WEBVIEW2_FLOOR L"__WEBVIEW2_FLOOR__"

static bool tachyon_web_engine_ready(void) {
  LPWSTR installed = NULL;
  int order = -1;
  bool ready = SUCCEEDED(GetAvailableCoreWebView2BrowserVersionString(NULL, &installed)) &&
               installed != NULL &&
               SUCCEEDED(CompareBrowserVersions(installed, TACHYON_WEBVIEW2_FLOOR, &order)) &&
               order >= 0;
  if (installed != NULL) CoTaskMemFree(installed);
  return ready;
}

static void tachyon_show_web_engine_message(void) {
  wchar_t *name = tachyon_widen(TACHYON_APP_NAME);
  const wchar_t *shown = name != NULL ? name : L"This app";
  wchar_t message[768];
  _snwprintf_s(message, 768, _TRUNCATE,
               L"%s needs the Microsoft Edge WebView2 Runtime %s or newer.\n\n"
               L"Install or update it from https://developer.microsoft.com/microsoft-edge/webview2/, "
               L"then open %s again.",
               shown, TACHYON_WEBVIEW2_FLOOR, shown);
  MessageBoxW(NULL, message, shown, MB_OK | MB_ICONERROR);
  free(name);
}
