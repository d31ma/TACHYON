#define TACHYON_LINK_COPY 0x544C4E4BUL
#define TACHYON_LINK_RECEIVER L"TachyonLinkReceiver"
static wchar_t g_launch_link[TACHYON_DEEP_LINK_LIMIT + 1];
static wchar_t g_launch_target[TACHYON_PATH_LIMIT];
static HANDLE g_link_instance = NULL;

static BOOL tachyon_link_target(const wchar_t *link, wchar_t *target) {
  char *narrow = link != NULL ? tachyon_narrow(link) : NULL;
  char local[TACHYON_PATH_LIMIT];
  int opens = narrow != NULL && tachyon_deep_link_target(narrow, local, sizeof(local));
  free(narrow);
  wchar_t *wide = opens ? tachyon_widen(local) : NULL;
  BOOL copied = wide != NULL && wcscpy_s(target, TACHYON_PATH_LIMIT, wide) == 0;
  free(wide);
  return copied;
}

static void tachyon_read_launch_link(void) {
  int count = 0;
  LPWSTR *arguments = CommandLineToArgvW(GetCommandLineW(), &count);
  if (arguments == NULL) return;
  if (count >= 2 && wcslen(arguments[1]) <= TACHYON_DEEP_LINK_LIMIT) {
    wcscpy_s(g_launch_link, TACHYON_DEEP_LINK_LIMIT + 1, arguments[1]);
  }
  LocalFree(arguments);
  if (g_launch_link[0] != L'\0' && tachyon_link_target(g_launch_link, g_launch_target)) {
    tachyon_record("link.opened");
  }
}

static void tachyon_register_link_scheme(const char *name, const wchar_t *command, const wchar_t *label) {
  wchar_t *scheme = tachyon_widen(name);
  if (scheme == NULL) return;
  wchar_t key[128], open[160];
  _snwprintf_s(key, 128, _TRUNCATE, L"Software\\Classes\\%s", scheme);
  _snwprintf_s(open, 160, _TRUNCATE, L"Software\\Classes\\%s\\shell\\open\\command", scheme);
  free(scheme);
  LSTATUS named = RegSetKeyValueW(HKEY_CURRENT_USER, key, NULL, REG_SZ, label,
                                  (DWORD)((wcslen(label) + 1) * sizeof(wchar_t)));
  LSTATUS marked = RegSetKeyValueW(HKEY_CURRENT_USER, key, L"URL Protocol", REG_SZ, L"", sizeof(wchar_t));
  LSTATUS opened = RegSetKeyValueW(HKEY_CURRENT_USER, open, NULL, REG_SZ, command,
                                   (DWORD)((wcslen(command) + 1) * sizeof(wchar_t)));
  tachyon_record(named == ERROR_SUCCESS && marked == ERROR_SUCCESS && opened == ERROR_SUCCESS
                     ? "link.registered" : "link.register.failed");
}

static void tachyon_register_link_schemes(void) {
  wchar_t *package_data = tachyon_package_data_folder();
  if (package_data != NULL) {
    free(package_data);
    return;
  }
  wchar_t executable[MAX_PATH], command[MAX_PATH + 16], label[160];
  DWORD length = GetModuleFileNameW(NULL, executable, MAX_PATH);
  wchar_t *name = tachyon_widen(TACHYON_APP_NAME);
  if (length == 0 || length >= MAX_PATH || name == NULL) { free(name); return; }
  _snwprintf_s(command, MAX_PATH + 16, _TRUNCATE, L"\"%s\" \"%%1\"", executable);
  _snwprintf_s(label, 160, _TRUNCATE, L"URL:%s", name);
  free(name);
  for (size_t index = 0; TACHYON_LINK_SCHEMES[index] != NULL; index++) {
    tachyon_register_link_scheme(TACHYON_LINK_SCHEMES[index], command, label);
  }
}

static BOOL tachyon_forward_link(void) {
  wchar_t *bundle = TACHYON_LINK_SCHEMES[0] != NULL ? tachyon_widen(TACHYON_BUNDLE_ID) : NULL;
  if (bundle == NULL) return FALSE;
  wchar_t name[320];
  _snwprintf_s(name, 320, _TRUNCATE, L"Local\\Tachyon.%s", bundle);
  g_link_instance = CreateMutexW(NULL, FALSE, name);
  BOOL running = g_link_instance != NULL && GetLastError() == ERROR_ALREADY_EXISTS;
  HWND receiver = NULL;
  for (int attempt = 0; running && receiver == NULL && attempt < 50; attempt++) {
    receiver = FindWindowExW(HWND_MESSAGE, NULL, TACHYON_LINK_RECEIVER, bundle);
    if (receiver == NULL) Sleep(100);
  }
  free(bundle);
  if (!running) return FALSE;
  if (receiver != NULL) {
    COPYDATASTRUCT copy = {TACHYON_LINK_COPY, (DWORD)((wcslen(g_launch_link) + 1) * sizeof(wchar_t)), g_launch_link};
    DWORD process = 0;
    GetWindowThreadProcessId(receiver, &process);
    AllowSetForegroundWindow(process);
    SendMessageTimeoutW(receiver, WM_COPYDATA, 0, (LPARAM)&copy, SMTO_ABORTIFHUNG, 5000, NULL);
  }
  CloseHandle(g_link_instance);
  g_link_instance = NULL;
  return TRUE;
}

static void tachyon_open_link(const wchar_t *link) {
  wchar_t target[TACHYON_PATH_LIMIT];
  if (link[0] != L'\0' && tachyon_link_target(link, target)) {
    tachyon_record("link.opened");
    if (g_webview != NULL) ICoreWebView2_Navigate(g_webview, target);
    else wcscpy_s(g_launch_target, TACHYON_PATH_LIMIT, target);
  }
  if (g_window == NULL) return;
  if (IsIconic(g_window)) ShowWindow(g_window, SW_RESTORE);
  SetForegroundWindow(g_window);
}

static LRESULT CALLBACK tachyon_link_receiver_proc(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
  if (message != WM_COPYDATA) return DefWindowProcW(window, message, wparam, lparam);
  const COPYDATASTRUCT *copy = (const COPYDATASTRUCT *)lparam;
  size_t characters = copy != NULL ? copy->cbData / sizeof(wchar_t) : 0;
  if (copy == NULL || copy->dwData != TACHYON_LINK_COPY || copy->lpData == NULL || characters == 0 ||
      characters > TACHYON_DEEP_LINK_LIMIT + 1 || copy->cbData % sizeof(wchar_t) != 0) return FALSE;
  const wchar_t *link = (const wchar_t *)copy->lpData;
  if (link[characters - 1] != L'\0') return FALSE;
  tachyon_open_link(link);
  return TRUE;
}

static void tachyon_links_listen(HINSTANCE instance) {
  wchar_t *bundle = g_link_instance != NULL ? tachyon_widen(TACHYON_BUNDLE_ID) : NULL;
  if (bundle == NULL) return;
  WNDCLASSEXW receiver_class = {0};
  receiver_class.cbSize = sizeof(receiver_class);
  receiver_class.lpfnWndProc = tachyon_link_receiver_proc;
  receiver_class.hInstance = instance;
  receiver_class.lpszClassName = TACHYON_LINK_RECEIVER;
  RegisterClassExW(&receiver_class);
  CreateWindowExW(0, TACHYON_LINK_RECEIVER, bundle, 0, 0, 0, 0, 0, HWND_MESSAGE, NULL, instance, NULL);
  free(bundle);
}
