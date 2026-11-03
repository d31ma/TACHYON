#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#define TACHYON_MESSAGE_LIMIT 65536
static HWND g_window = NULL;
__LOCAL_JSON__
__WINDOW_HOST__
static int failures, closes;
static void expect(int condition, const char *what) {
  if (!condition) { fprintf(stderr, "FAILED: %s\n", what); failures++; }
}
static void check(const char *operation, const char *request, const char *expected) {
  char *answer = tachyon_window_control(operation, request);
  if (strcmp(answer, expected) != 0) { fprintf(stderr, "FAILED: %s %s answered %s\n", operation, request, answer); failures++; }
  free(answer);
}
static void pump(void) {
  MSG message;
  while (PeekMessageW(&message, NULL, 0, 0, PM_REMOVE)) { TranslateMessage(&message); DispatchMessageW(&message); }
}
static int same(RECT left, RECT right) {
  return left.left == right.left && left.top == right.top && left.right == right.right && left.bottom == right.bottom;
}
static LRESULT CALLBACK procedure(HWND window, UINT message, WPARAM first, LPARAM second) {
  if (message == WM_CLOSE) { closes++; return 0; }
  return DefWindowProcW(window, message, first, second);
}
static DWORD WINAPI foreign(LPVOID unused) {
  (void)unused;
  check("window.title", "{\"title\":\"foreign\"}", "{\"ok\":false,\"error\":\"Native window is unavailable.\"}");
  return 0;
}
int main(void) {
  SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
  WNDCLASSW type = {0};
  type.lpfnWndProc = procedure;
  type.hInstance = GetModuleHandleW(NULL);
  type.lpszClassName = L"TachyonWindowControlsCheck";
  expect(RegisterClassW(&type) != 0, "window class registers");
  g_window = CreateWindowExW(0, type.lpszClassName, L"start", WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT, 400, 300, NULL, NULL, type.hInstance, NULL);
  if (g_window == NULL) { fprintf(stderr, "FAILED: CreateWindowExW error %lu\n", GetLastError()); return 1; }
  ShowWindow(g_window, SW_SHOWNORMAL);
  pump();

  check("window.granted", "{\"control\":\"title\"}", "{\"granted\":true}");
  check("window.granted", "{\"control\":\"close\"}", "{\"granted\":false}");
  check("window.close", "{}", "{\"ok\":false,\"error\":\"Window control is not granted.\"}");
  pump();
  expect(closes == 0 && IsWindow(g_window), "denied close leaves the window open without WM_CLOSE");

  check("window.title", "{\"title\":\"A\\\"\\uD83D\\uDE00\"}", "{\"ok\":true}");
  wchar_t title[16] = {0};
  GetWindowTextW(g_window, title, 16);
  expect(wcscmp(title, L"A\"\xD83D\xDE00") == 0, "title is the decoded UTF-16 text");
  check("window.title", "{\"title\":\"\\u0000\"}", "{\"ok\":false,\"error\":\"Invalid window title.\"}");
  HANDLE thread = CreateThread(NULL, 0, foreign, NULL, 0, NULL);
  expect(thread != NULL && WaitForSingleObject(thread, 10000) == WAIT_OBJECT_0, "foreign thread finishes");
  if (thread != NULL) CloseHandle(thread);
  GetWindowTextW(g_window, title, 16);
  expect(wcscmp(title, L"A\"\xD83D\xDE00") == 0, "a foreign thread cannot retitle the window");

  check("window.resize", "{\"width\":640,\"height\":480}", "{\"ok\":true}");
  pump();
  RECT client;
  UINT dpi = GetDpiForWindow(g_window);
  int width = MulDiv(640, (int)dpi, USER_DEFAULT_SCREEN_DPI), height = MulDiv(480, (int)dpi, USER_DEFAULT_SCREEN_DPI);
  GetClientRect(g_window, &client);
  expect(client.right - client.left == width && client.bottom - client.top == height, "resize sets the client area in device-independent pixels");
  check("window.resize", "{\"width\":64.5,\"height\":100}", "{\"ok\":false,\"error\":\"Invalid window size.\"}");
  GetClientRect(g_window, &client);
  expect(client.right - client.left == width && client.bottom - client.top == height, "invalid resize keeps the client area");

  RECT before, covered;
  GetWindowRect(g_window, &before);
  LONG_PTR style = GetWindowLongPtrW(g_window, GWL_STYLE);
  check("window.fullscreen", "{}", "{\"ok\":true}");
  pump();
  MONITORINFO monitor = {sizeof(MONITORINFO)};
  GetMonitorInfoW(MonitorFromWindow(g_window, MONITOR_DEFAULTTONEAREST), &monitor);
  GetWindowRect(g_window, &covered);
  expect(same(covered, monitor.rcMonitor), "fullscreen covers the monitor");
  expect((GetWindowLongPtrW(g_window, GWL_STYLE) & WS_CAPTION) == 0, "fullscreen removes the frame");
  check("window.fullscreen", "{}", "{\"ok\":true}");
  pump();
  GetWindowRect(g_window, &covered);
  expect(same(covered, before), "leaving fullscreen restores the bounds");
  expect(GetWindowLongPtrW(g_window, GWL_STYLE) == style, "leaving fullscreen restores the frame");

  check("window.minimize", "{}", "{\"ok\":true}");
  pump();
  expect(IsIconic(g_window), "minimize iconifies the window");

  DestroyWindow(g_window);
  if (failures) return 1;
  printf("win32 window controls: granted title/resize (at %u DPI)/fullscreen/minimize, denied close and foreign thread on a real HWND\n", dpi);
  return 0;
}
