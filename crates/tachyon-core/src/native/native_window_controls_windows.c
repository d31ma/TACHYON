__WINDOW_PAYLOAD__

__WINDOW_FULLSCREEN__
static char *tachyon_window_control(const char *capability, const char *payload) {
  (void)payload;
  if (strcmp(capability, "window.granted") == 0) {
    int granted = __WINDOW_GRANTS__;
    return _strdup(granted ? "{\"granted\":true}" : "{\"granted\":false}");
  }
  if (g_window == NULL || GetWindowThreadProcessId(g_window, NULL) != GetCurrentThreadId()) return _strdup("{\"ok\":false,\"error\":\"Native window is unavailable.\"}");
__WINDOW_ACTIONS__
  return _strdup("{\"ok\":false,\"error\":\"Window control is not granted.\"}");
}
