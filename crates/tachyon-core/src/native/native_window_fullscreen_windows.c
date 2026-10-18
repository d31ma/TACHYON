static LONG_PTR tachyon_fullscreen_style;
static RECT tachyon_fullscreen_bounds;
static int tachyon_fullscreen_active;

static BOOL tachyon_window_toggle_fullscreen(HWND window) {
  if (!tachyon_fullscreen_active) {
    MONITORINFO info = {sizeof(MONITORINFO)};
    if (!GetWindowRect(window, &tachyon_fullscreen_bounds) || !GetMonitorInfoW(MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST), &info)) return FALSE;
    tachyon_fullscreen_style = GetWindowLongPtrW(window, GWL_STYLE);
    SetWindowLongPtrW(window, GWL_STYLE, tachyon_fullscreen_style & ~(LONG_PTR)WS_OVERLAPPEDWINDOW);
    SetWindowPos(window, HWND_TOP, info.rcMonitor.left, info.rcMonitor.top, info.rcMonitor.right - info.rcMonitor.left, info.rcMonitor.bottom - info.rcMonitor.top, SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
  } else {
    SetWindowLongPtrW(window, GWL_STYLE, tachyon_fullscreen_style);
    SetWindowPos(window, NULL, tachyon_fullscreen_bounds.left, tachyon_fullscreen_bounds.top, tachyon_fullscreen_bounds.right - tachyon_fullscreen_bounds.left, tachyon_fullscreen_bounds.bottom - tachyon_fullscreen_bounds.top, SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
  }
  tachyon_fullscreen_active = !tachyon_fullscreen_active;
  return TRUE;
}
