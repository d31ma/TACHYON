#include <stdlib.h>
#include <string.h>

static LRESULT CALLBACK tachyon_fixture_proc(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
  if (tachyon_tray_message(window, message, wparam, lparam)) return 0;
  if (message == WM_CLOSE && tachyon_tray_hides_window(window)) return 0;
  if (message == WM_COMMAND && tachyon_menus_command(window, LOWORD(wparam))) return 0;
  if (message == WM_DESTROY) { tachyon_tray_remove(); PostQuitMessage(0); return 0; }
  return DefWindowProcW(window, message, wparam, lparam);
}

static void tachyon_fixture_require(BOOL valid, const char *message) {
  if (valid) return;
  printf("windows tray fixture failed: %s\n", message);
  exit(1);
}

static void tachyon_fixture_pump(void) {
  MSG message;
  while (PeekMessageW(&message, NULL, 0, 0, PM_REMOVE)) {
    if (message.message == WM_QUIT) return;
    TranslateMessage(&message);
    DispatchMessageW(&message);
  }
}

int main(void) {
  WNDCLASSEXW window_class = {0};
  window_class.cbSize = sizeof(window_class);
  window_class.lpfnWndProc = tachyon_fixture_proc;
  window_class.hInstance = GetModuleHandleW(NULL);
  window_class.lpszClassName = L"TachyonTrayFixture";
  RegisterClassExW(&window_class);
  g_window = CreateWindowExW(0, L"TachyonTrayFixture", L"Tray", WS_OVERLAPPEDWINDOW, 0, 0, 320, 200, NULL, NULL, window_class.hInstance, NULL);
  tachyon_fixture_require(g_window != NULL, "no window");
  tachyon_menus_install(g_window);
  tachyon_tray_install(g_window);
  tachyon_fixture_require(g_tray_shown, "Shell_NotifyIconW refused the icon");
  tachyon_fixture_require(g_tray.hIcon != NULL, "no icon");
  tachyon_fixture_require(g_tray_icon_loaded, "the icon fell back to IDI_APPLICATION instead of loading the manifest's tray.ico");
  tachyon_fixture_require(strcmp(tachyon_tray_icon_event(), "tray.icon.loaded") == 0, "the host would record the wrong tray icon event");
  tachyon_fixture_require(wcscmp(g_tray.szTip, L"Tray tooltip é") == 0, "the tooltip is not the declared one");
  ShowWindow(g_window, SW_SHOW);
  SendMessageW(g_window, WM_CLOSE, 0, 0);
  tachyon_fixture_require(IsWindow(g_window) && !IsWindowVisible(g_window), "closing did not hide the window");
  SendMessageW(g_window, g_tray.uCallbackMessage, 0, MAKELPARAM(NIN_SELECT, TACHYON_TRAY_ID));
  tachyon_fixture_require(IsWindowVisible(g_window), "a click on the icon did not show the window");
  ShowWindow(g_window, SW_HIDE);
  SendMessageW(g_window, WM_COMMAND, MAKEWPARAM(TACHYON_TRAY_OPEN_COMMAND, 0), 0);
  tachyon_fixture_require(IsWindowVisible(g_window), "Open did not show the window");
  SendMessageW(g_window, g_tray_restarted, 0, 0);
  tachyon_fixture_require(g_tray_shown, "the icon did not come back after Explorer restarted");
  SendMessageW(g_window, WM_COMMAND, MAKEWPARAM(TACHYON_TRAY_QUIT_COMMAND, 0), 0);
  tachyon_fixture_pump();
  tachyon_fixture_require(!IsWindow(g_window), "Exit did not close the window");
  tachyon_fixture_require(!g_tray_shown, "the icon stayed after Exit");
  (void)&tachyon_menus_bind;
  (void)&tachyon_menus_refresh;
  (void)&tachyon_menus_translate;
  (void)&tachyon_menus_key;
  (void)&tachyon_menus_destroy;
  printf("windows tray passed\n");
  return 0;
}
