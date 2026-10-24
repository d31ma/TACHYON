#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdio.h>
__WINDOW_BACKGROUND__
static int failures;
static void expect(int condition, const char *what) {
  if (!condition) { fprintf(stderr, "FAILED: %s\n", what); failures++; }
}
static void pump(void) {
  MSG message;
  while (PeekMessageW(&message, NULL, 0, 0, PM_REMOVE)) { TranslateMessage(&message); DispatchMessageW(&message); }
}
int main(void) {
  WNDCLASSEXW type = {0};
  type.cbSize = sizeof(type);
  type.lpfnWndProc = DefWindowProcW;
  type.hInstance = GetModuleHandleW(NULL);
  type.lpszClassName = L"TachyonWindowBackgroundCheck";
  type.hbrBackground = tachyon_window_background_brush();
  expect(type.hbrBackground != NULL, "background_color gives the window class a brush");
  expect(RegisterClassExW(&type) != 0, "window class registers");
  HWND window = CreateWindowExW(0, L"TachyonWindowBackgroundCheck", L"background", WS_OVERLAPPEDWINDOW, 40, 40, 480, 360, NULL, NULL, type.hInstance, NULL);
  if (window == NULL) { fprintf(stderr, "FAILED: CreateWindowExW error %lu\n", GetLastError()); return 1; }
  LOGBRUSH brush;
  HBRUSH registered = (HBRUSH)GetClassLongPtrW(window, GCLP_HBRBACKGROUND);
  expect(registered != NULL && GetObjectW(registered, sizeof(brush), &brush) == sizeof(brush) && brush.lbStyle == BS_SOLID &&
         brush.lbColor == RGB(TACHYON_EXPECT_RED, TACHYON_EXPECT_GREEN, TACHYON_EXPECT_BLUE),
         "the class brush is the manifest background_color");
  ShowWindow(window, SW_SHOWNORMAL);
  UpdateWindow(window);
  pump();
  RECT client;
  GetClientRect(window, &client);
  HDC context = GetDC(window);
  COLORREF painted = GetPixel(context, client.right / 2, client.bottom / 2);
  ReleaseDC(window, context);
  expect(painted == RGB(TACHYON_EXPECT_RED, TACHYON_EXPECT_GREEN, TACHYON_EXPECT_BLUE), "the empty client area is painted in background_color");
  tachyon_window_background_environment();
  wchar_t value[16] = {0};
  expect(GetEnvironmentVariableW(L"WEBVIEW2_DEFAULT_BACKGROUND_COLOR", value, 16) == 8 && lstrcmpW(value, TACHYON_EXPECT_ENVIRONMENT) == 0,
         "WebView2 starts with background_color as its default background");
  DestroyWindow(window);
  if (failures) return 1;
  printf("win32 window background: class brush, painted client area %06lx and WebView2 start color checked on a real HWND\n", (unsigned long)painted);
  return 0;
}
