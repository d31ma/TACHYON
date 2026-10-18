#define TACHYON_CAPTION_COLOR_ATTRIBUTE 35
#define TACHYON_CAPTION_TEXT_ATTRIBUTE 36

static SIZE tachyon_window_outer(HWND window, int width, int height) {
  UINT dpi = GetDpiForWindow(window);
  if (dpi == 0) dpi = USER_DEFAULT_SCREEN_DPI;
  RECT bounds = {0, 0, 0, 0};
  bounds.right = MulDiv(width, (int)dpi, USER_DEFAULT_SCREEN_DPI);
  bounds.bottom = MulDiv(height, (int)dpi, USER_DEFAULT_SCREEN_DPI);
  AdjustWindowRectExForDpi(&bounds, (DWORD)GetWindowLongPtrW(window, GWL_STYLE), GetMenu(window) != NULL, (DWORD)GetWindowLongPtrW(window, GWL_EXSTYLE), dpi);
  SIZE outer;
  outer.cx = bounds.right - bounds.left;
  outer.cy = bounds.bottom - bounds.top;
  return outer;
}
__WINDOW_LIMITS__
static int tachyon_window_open(HWND window, int show) {
  SIZE outer = tachyon_window_outer(window, __WINDOW_WIDTH__, __WINDOW_HEIGHT__);
  SetWindowPos(window, NULL, 0, 0, outer.cx, outer.cy, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
__WINDOW_OPENING__  return show;
}
