#include <stdlib.h>
#include <string.h>

typedef char *(*TachyonFixtureInvoke)(const char *);
typedef void (*TachyonFixtureFree)(char *);
typedef void (*TachyonFixtureEmit)(const char *);
typedef void (*TachyonFixtureSetEmit)(TachyonFixtureEmit);
typedef bool (*TachyonFixtureStart)(void);
typedef long long (*TachyonFixtureCount)(const char *);

static TachyonFixtureInvoke g_fixture_invoke = NULL;
static TachyonFixtureFree g_fixture_free = NULL;
static TachyonFixtureCount g_fixture_count = NULL;
static char g_fixture_signals[4096] = {0};

static LRESULT CALLBACK tachyon_fixture_proc(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
  if (message == WM_COMMAND && tachyon_menus_command(window, LOWORD(wparam))) return 0;
  if (message == WM_INITMENUPOPUP) { tachyon_menus_refresh((HMENU)wparam); return 0; }
  return DefWindowProcW(window, message, wparam, lparam);
}

static void tachyon_fixture_require(BOOL valid, const char *message) {
  if (valid) return;
  printf("windows menu action fixture failed: %s\n", message);
  exit(1);
}

static void tachyon_fixture_emit(const char *payload) {
  strncat(g_fixture_signals, payload, sizeof(g_fixture_signals) - strlen(g_fixture_signals) - 2);
  strncat(g_fixture_signals, "\n", sizeof(g_fixture_signals) - strlen(g_fixture_signals) - 1);
}

static BOOL tachyon_fixture_answer(const char *operation, const char *name, const char *expected) {
  char request[512];
  snprintf(request, sizeof(request),
           "{\"route\":\"/\",\"service\":\"section\",\"owner\":\"00000000-0000-4000-8000-000000000001\",\"op\":\"%s\",\"name\":\"%s\",\"args\":[]}",
           operation, name);
  char *answer = g_fixture_invoke(request);
  BOOL matches = answer != NULL && (expected == NULL ? strstr(answer, "error") == NULL : strcmp(answer, expected) == 0);
  if (answer != NULL) g_fixture_free(answer);
  return matches;
}

static UINT tachyon_fixture_command(const char *id) {
  for (int index = 0; index < TACHYON_MENU_ITEM_COUNT; index += 1) {
    if (strcmp(TACHYON_MENU_ITEMS[index].id, id) == 0) return (UINT)(TACHYON_MENU_FIRST_COMMAND + index);
  }
  return 0;
}

static BOOL tachyon_fixture_grayed(HMENU popup, UINT command) {
  SendMessageW(g_window, WM_INITMENUPOPUP, (WPARAM)popup, 0);
  return (GetMenuState(popup, command, MF_BYCOMMAND) & MF_GRAYED) != 0;
}

static void tachyon_fixture_control(BOOL down) {
  BYTE keys[256];
  GetKeyboardState(keys);
  keys[VK_CONTROL] = down ? 0x80 : 0;
  SetKeyboardState(keys);
}

static void tachyon_fixture_pump(void) {
  MSG message;
  while (PeekMessageW(&message, NULL, 0, 0, PM_REMOVE)) DispatchMessageW(&message);
}

int main(void) {
  HMODULE library = LoadLibraryW(L"TachyonRustCompanion.dll");
  tachyon_fixture_require(library != NULL, "the Rust companion DLL does not load");
  tachyon_menus_bind(library);
  g_fixture_invoke = (TachyonFixtureInvoke)(void (*)(void))GetProcAddress(library, "tac_native_invoke");
  g_fixture_free = (TachyonFixtureFree)(void (*)(void))GetProcAddress(library, "tac_native_free");
  g_fixture_count = (TachyonFixtureCount)(void (*)(void))GetProcAddress(library, "tac_fixture_menu_count");
  TachyonFixtureSetEmit set_emit = (TachyonFixtureSetEmit)(void (*)(void))GetProcAddress(library, "tac_native_set_emit");
  TachyonFixtureStart start = (TachyonFixtureStart)(void (*)(void))GetProcAddress(library, "tac_native_application_start");
  tachyon_fixture_require(g_fixture_invoke != NULL && g_fixture_free != NULL && g_fixture_count != NULL && set_emit != NULL && start != NULL,
                          "the Rust companion DLL lacks its exports");
  set_emit(tachyon_fixture_emit);
  tachyon_fixture_require(start(), "the application services did not start");
  WNDCLASSEXW window_class = {0};
  window_class.cbSize = sizeof(window_class);
  window_class.lpfnWndProc = tachyon_fixture_proc;
  window_class.hInstance = GetModuleHandleW(NULL);
  window_class.lpszClassName = L"TachyonMenuActionFixture";
  RegisterClassExW(&window_class);
  g_window = CreateWindowExW(0, L"TachyonMenuActionFixture", L"Menus", WS_OVERLAPPEDWINDOW, 0, 0, 320, 200, NULL, NULL, window_class.hInstance, NULL);
  tachyon_fixture_require(g_window != NULL, "no window");
  tachyon_menus_install(g_window);
  HMENU view = GetSubMenu(GetMenu(g_window), 2);
  UINT running = tachyon_fixture_command("show-running");
  tachyon_fixture_require(view != NULL && running != 0, "the View menu has no Show Running item");
  tachyon_fixture_require(tachyon_fixture_grayed(view, running), "the page action is enabled without a recipient");
  tachyon_fixture_require(tachyon_fixture_answer("init", "", NULL), "the page service did not start");
  tachyon_fixture_require(!tachyon_fixture_grayed(view, running), "the page action stayed MF_GRAYED after the page bound its listener");
  SendMessageW(g_window, WM_COMMAND, MAKEWPARAM(running, 0), 0);
  tachyon_fixture_require(g_fixture_count("forwarded") == 1 && g_fixture_count("chosen") == 1, "WM_COMMAND did not reach the page service");
  tachyon_fixture_require(tachyon_fixture_answer("get", "section", "{\"value\":\"running\"}"), "the page service did not receive the section");
  tachyon_fixture_require(strstr(g_fixture_signals,
                                 "{\"route\":\"/\",\"service\":\"section\",\"owner\":\"00000000-0000-4000-8000-000000000001\",\"changed\":true}\n") != NULL,
                          "the page was not told to read its changed service");
  tachyon_fixture_control(TRUE);
  MSG key = {g_window, WM_KEYDOWN, '1', 0, 0, {0, 0}};
  tachyon_fixture_require(tachyon_menus_translate(g_window, &key), "Ctrl+1 is not in the accelerator table");
  tachyon_fixture_require(g_fixture_count("forwarded") == 2, "Ctrl+1 did not run the menu action");
  tachyon_fixture_require(tachyon_menus_key(g_window, '1'), "the WebView2 key path does not know Ctrl+1");
  tachyon_fixture_control(FALSE);
  tachyon_fixture_pump();
  tachyon_fixture_require(g_fixture_count("forwarded") == 3, "Ctrl+1 from the web view did not run the menu action");
  tachyon_fixture_require(tachyon_fixture_answer("dispose", "", "{\"value\":null}"), "the page service did not stop");
  tachyon_fixture_require(tachyon_fixture_grayed(view, running), "the page action stayed enabled after its listener left");
  tachyon_menus_destroy();
  DestroyWindow(g_window);
  (void)&tachyon_tray_install;
  (void)&tachyon_tray_remove;
  (void)&tachyon_tray_message;
  (void)&tachyon_tray_hides_window;
  (void)&tachyon_tray_icon_event;
  printf("windows menu actions passed\n");
  return 0;
}
