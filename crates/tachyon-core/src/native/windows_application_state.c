static BOOL g_application_background = FALSE;

static void tachyon_application_state_post(void) {
  if (g_webview == NULL || tachyon_current_route() == NULL) return;
  ICoreWebView2_PostWebMessageAsJson(g_webview, g_application_background
      ? L"{\"applicationState\":\"background\"}"
      : L"{\"applicationState\":\"foreground\"}");
}

static void tachyon_application_state_send(BOOL background) {
  if (background == g_application_background) return;
  g_application_background = background;
  tachyon_record(background ? "application.background" : "application.foreground");
  tachyon_application_state_post();
}

static void tachyon_application_state_resized(HWND window, WPARAM kind) {
  if (kind == SIZE_MINIMIZED) tachyon_application_state_send(TRUE);
  else if (GetForegroundWindow() == window) tachyon_application_state_send(FALSE);
}
