#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <dwmapi.h>
#include <stdio.h>
__WINDOW_FULLSCREEN__
__WINDOW_FRAME__
static int failures;
static void expect(int condition, const char *what) {
  if (!condition) { fprintf(stderr, "FAILED: %s\n", what); failures++; }
}
static void pump(void) {
  MSG message;
  while (PeekMessageW(&message, NULL, 0, 0, PM_REMOVE)) { TranslateMessage(&message); DispatchMessageW(&message); }
}
static LRESULT CALLBACK procedure(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
  switch (message) {
__WINDOW_FRAME_MESSAGES__    default:
      break;
  }
  return DefWindowProcW(window, message, wparam, lparam);
}
#ifndef TACHYON_EXPECT_FULLSCREEN
static int scaled(HWND window, int edge) { return MulDiv(edge, (int)GetDpiForWindow(window), USER_DEFAULT_SCREEN_DPI); }
#endif
static HWND opened(const wchar_t *name, int requested, int *show) {
  HWND window = CreateWindowExW(0, L"TachyonWindowFrameCheck", name, WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, NULL, NULL, GetModuleHandleW(NULL), NULL);
  if (window == NULL) { fprintf(stderr, "FAILED: CreateWindowExW error %lu\n", GetLastError()); return NULL; }
  *show = tachyon_window_open(window, requested);
  ShowWindow(window, *show);
  pump();
  return window;
}
int main(void) {
  SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
  WNDCLASSW type = {0};
  type.lpfnWndProc = procedure;
  type.hInstance = GetModuleHandleW(NULL);
  type.lpszClassName = L"TachyonWindowFrameCheck";
  expect(RegisterClassW(&type) != 0, "window class registers");
  int show = 0;
#ifdef TACHYON_EXPECT_FULLSCREEN
  HWND window = opened(L"fullscreen", SW_SHOWMAXIMIZED, &show);
  if (window == NULL) return 1;
  expect(show == SW_SHOWNORMAL, "a fullscreen start shows the window normally");
  MONITORINFO monitor = {sizeof(MONITORINFO)};
  GetMonitorInfoW(MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST), &monitor);
  RECT bounds;
  GetWindowRect(window, &bounds);
  expect(EqualRect(&bounds, &monitor.rcMonitor), "a fullscreen start covers the monitor");
  expect((GetWindowLongPtrW(window, GWL_STYLE) & WS_CAPTION) == 0, "a fullscreen start has no frame");
  expect(tachyon_window_toggle_fullscreen(window) && (GetWindowLongPtrW(window, GWL_STYLE) & WS_CAPTION) != 0, "leaving a fullscreen start brings the frame back");
#else
  HWND window = opened(L"sized", SW_SHOWNORMAL, &show);
  if (window == NULL) return 1;
  expect(show == SW_SHOWNORMAL, "a windowed start keeps the requested show state");
  RECT client;
  GetClientRect(window, &client);
  expect(client.right == scaled(window, 800) && client.bottom == scaled(window, 600), "the client area is the manifest size in DIPs");
  SetWindowPos(window, NULL, 0, 0, 10, 10, SWP_NOMOVE | SWP_NOZORDER);
  pump();
  GetClientRect(window, &client);
  expect(client.right >= scaled(window, 480) && client.bottom >= scaled(window, 360), "the window cannot shrink below the manifest minimum");
  expect(client.right <= scaled(window, 481) && client.bottom <= scaled(window, 361), "the minimum is the manifest minimum, not more");
#endif
  DestroyWindow(window);
  if (failures) return 1;
  printf("win32 window frame: manifest size, minimum and fullscreen start checked on a real HWND\n");
  return 0;
}
