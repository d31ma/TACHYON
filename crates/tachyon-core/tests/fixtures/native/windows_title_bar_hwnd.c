#define UNICODE
#define WIN32_LEAN_AND_MEAN
#define COBJMACROS
#include <windows.h>
#include <dwmapi.h>
#include <objbase.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#include <WebView2.h>
#define TACHYON_APP_NAME "Title Bar Check"
static ICoreWebView2 *g_webview = NULL;
static int g_records;
static const char *tachyon_current_route(void) { return NULL; }
static void tachyon_record(const char *event) { (void)event; g_records++; }
__WINDOW_FRAME__
__WINDOW_TITLE_BAR__
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
    case WM_NCCALCSIZE:
      if (tachyon_title_bar_calculate(window, wparam, lparam)) return 0;
      break;
    case WM_NCHITTEST: {
      BOOL handled = FALSE;
      LRESULT hit = tachyon_title_bar_hit(window, wparam, lparam, &handled);
      if (handled) return hit;
      break;
    }
    case WM_ACTIVATE:
    case WM_SIZE:
      tachyon_title_bar_layout(window);
      break;
    case WM_PAINT:
      if (tachyon_title_bar_paint(window)) return 0;
      break;
    default:
      break;
  }
  return DefWindowProcW(window, message, wparam, lparam);
}
static int scaled(HWND window, int edge) { return MulDiv(edge, (int)GetDpiForWindow(window), USER_DEFAULT_SCREEN_DPI); }
static LRESULT hit(HWND window, int x, int y) { return SendMessageW(window, WM_NCHITTEST, 0, MAKELPARAM(x, y)); }
static int insets(int *height, int *leading, int *trailing) {
  char *answer = tachyon_title_bar_insets();
  int read = answer != NULL ? sscanf(answer, "{\"height\":%d,\"leading\":%d,\"trailing\":%d}", height, leading, trailing) : 0;
  free(answer);
  return read == 3;
}
static BOOL in_system_menu(HWND window, const wchar_t *title) {
  HMENU system = GetSystemMenu(window, FALSE);
  wchar_t text[64];
  for (int index = 0; system != NULL && index < GetMenuItemCount(system); index += 1) {
    if (GetSubMenu(system, index) != NULL && GetMenuStringW(system, (UINT)index, text, 64, MF_BYPOSITION) > 0 && wcscmp(text, title) == 0) return TRUE;
  }
  return FALSE;
}
int main(void) {
  (void)TACHYON_TITLE_BAR_PAGE_SCRIPT;
  (void)tachyon_title_bar_settings;
  SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
  WNDCLASSW type = {0};
  type.lpfnWndProc = procedure;
  type.hInstance = GetModuleHandleW(NULL);
  type.lpszClassName = L"TachyonTitleBarCheck";
  type.hCursor = LoadCursorW(NULL, (LPCWSTR)IDC_ARROW);
  expect(RegisterClassW(&type) != 0, "window class registers");
  HWND window = CreateWindowExW(0, L"TachyonTitleBarCheck", L"Title Bar Check", WS_OVERLAPPEDWINDOW, 40, 40, 400, 300, NULL, NULL, type.hInstance, NULL);
  if (window == NULL) { fprintf(stderr, "FAILED: CreateWindowExW error %lu\n", GetLastError()); return 1; }
  HMENU bar = CreateMenu(), file = CreatePopupMenu();
  AppendMenuW(file, MF_STRING, 0x7000, L"&Close Window\tCtrl+W");
  AppendMenuW(bar, MF_POPUP, (UINT_PTR)file, L"&File");
  SetMenu(window, bar);
  expect(tachyon_title_bar_install(window, type.hInstance), "the title bar installs");
  expect(g_title_bar_host != NULL && GetParent(g_title_bar_host) == window, "the web view gets its own host window");
  expect(tachyon_title_bar_web_parent(window) == g_title_bar_host, "the web view is created in the host window");
  expect(GetMenu(window) == NULL, "the menu bar leaves the top edge");
  expect(in_system_menu(window, L"&File"), "the menus move into the window menu");
  ShowWindow(window, tachyon_window_open(window, SW_SHOWNORMAL));
  pump();
  RECT frame, client, host;
  POINT origin = {0, 0};
  GetWindowRect(window, &frame);
  GetClientRect(window, &client);
  ClientToScreen(window, &origin);
  expect(origin.y == frame.top, "the client area starts at the top edge of the window");
  expect(origin.x > frame.left && frame.bottom > origin.y + client.bottom, "the sides and the bottom keep their resize frame");
  expect(client.right == scaled(window, 800) && client.bottom == scaled(window, 600), "the client area is the manifest size without a caption");
  GetWindowRect(g_title_bar_host, &host);
  expect(host.right - host.left == client.right && host.bottom - host.top == client.bottom, "the web view host fills the client area");
  int border = tachyon_title_bar_border(window);
  expect(hit(window, origin.x + client.right / 2, frame.top + border / 2) == HTTOP, "the top edge resizes the window");
  expect(hit(window, origin.x + 1, frame.top + border / 2) == HTTOPLEFT, "the top leading corner resizes the window");
  expect(hit(window, origin.x + client.right / 2, origin.y + client.bottom / 2) == HTCLIENT, "the page area stays client");
  int height = 0, leading = -1, trailing = 0;
  expect(insets(&height, &leading, &trailing), "the insets answer is JSON");
  expect(height > 0 && leading == 0 && trailing > 0 && trailing < 400, "the insets hold the caption height and the caption button width");
  RECT buttons;
  BOOL found = tachyon_title_bar_buttons(window, &buttons);
  printf("caption buttons: %ld,%ld,%ld,%ld; client right %ld; border %d\n", buttons.left, buttons.top, buttons.right, buttons.bottom, client.right, border);
  expect(found && labs(buttons.right - client.right) <= border && buttons.top <= 0, "the caption buttons sit at the trailing top corner");
  HRGN region = CreateRectRgn(0, 0, 0, 0);
  expect(GetWindowRgn(g_title_bar_host, region) != ERROR, "the web view host has a region");
  expect(!PtInRegion(region, buttons.left + 1, buttons.bottom - 1) && PtInRegion(region, buttons.left - 1, buttons.bottom - 1) && PtInRegion(region, 1, client.bottom - 1),
         "the region leaves out only the caption buttons");
  DeleteObject(region);
  ShowWindow(window, SW_MAXIMIZE);
  pump();
  origin.x = 0;
  origin.y = 0;
  GetWindowRect(window, &frame);
  ClientToScreen(window, &origin);
  if (IsZoomed(window)) {
    expect(origin.y == frame.top + border, "a maximized window keeps its page on the screen");
    expect(hit(window, origin.x + 10, origin.y + 1) != HTTOP, "a maximized window does not resize from the top edge");
  }
  ShowWindow(window, SW_RESTORE);
  pump();
  LONG_PTR style = GetWindowLongPtrW(window, GWL_STYLE);
  SetWindowLongPtrW(window, GWL_STYLE, style & ~(LONG_PTR)WS_OVERLAPPEDWINDOW);
  SetWindowPos(window, HWND_TOP, 0, 0, 640, 480, SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
  pump();
  expect(insets(&height, &leading, &trailing) && height == 0 && leading == 0 && trailing == 0, "full screen has no title bar insets");
  BOOL handled = TRUE;
  tachyon_title_bar_hit(window, 0, MAKELPARAM(10, 10), &handled);
  expect(!handled, "full screen leaves hit testing to Windows");
  SetWindowLongPtrW(window, GWL_STYLE, style);
  SetWindowPos(window, NULL, 40, 40, 400, 300, SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
  pump();
  expect(insets(&height, &leading, &trailing) && height > 0 && trailing > 0, "leaving full screen brings the insets back");
  DestroyWindow(window);
  if (failures) return 1;
  printf("win32 title bar: client area at the top edge, resize edges, menus in the window menu, insets, caption button hole and full screen checked on a real HWND\n");
  return 0;
}
