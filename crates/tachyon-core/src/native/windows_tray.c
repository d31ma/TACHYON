__NATIVE_TRAY_SETTINGS__

#if TACHYON_TRAY
#define TACHYON_WM_TRAY (WM_APP + 7)
#define TACHYON_TRAY_OPEN_COMMAND 0x7004
#define TACHYON_TRAY_QUIT_COMMAND 0x7005
#define TACHYON_TRAY_ID 1

static NOTIFYICONDATAW g_tray = {0};
static BOOL g_tray_shown = FALSE;
static BOOL g_tray_quitting = FALSE;
static UINT g_tray_restarted = 0;
static BOOL g_tray_icon_loaded = FALSE;

static const char *tachyon_tray_icon_event(void) {
  return g_tray_icon_loaded ? "tray.icon.loaded" : "tray.icon.missing";
}

static HICON tachyon_tray_icon(void) {
  wchar_t path[MAX_PATH];
  DWORD length = GetModuleFileNameW(NULL, path, MAX_PATH);
  if (length == 0 || length >= MAX_PATH) return NULL;
  for (int level = 0; level < 2; level += 1) {
    wchar_t *separator = wcsrchr(path, L'\\');
    if (separator == NULL) return NULL;
    *separator = L'\0';
  }
  if (wcscat_s(path, MAX_PATH, L"\\resources\\" TACHYON_TRAY_ICON_FILE) != 0) return NULL;
  return (HICON)LoadImageW(NULL, path, IMAGE_ICON, GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON), LR_LOADFROMFILE);
}

static void tachyon_tray_add(void) {
  g_tray_shown = Shell_NotifyIconW(NIM_ADD, &g_tray);
  if (!g_tray_shown) return;
  g_tray.uVersion = NOTIFYICON_VERSION_4;
  Shell_NotifyIconW(NIM_SETVERSION, &g_tray);
}

static void tachyon_tray_install(HWND window) {
  g_tray_restarted = RegisterWindowMessageW(L"TaskbarCreated");
  g_tray.cbSize = sizeof(g_tray);
  g_tray.hWnd = window;
  g_tray.uID = TACHYON_TRAY_ID;
  g_tray.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE | NIF_SHOWTIP;
  g_tray.uCallbackMessage = TACHYON_WM_TRAY;
  g_tray.hIcon = tachyon_tray_icon();
  g_tray_icon_loaded = g_tray.hIcon != NULL;
  if (g_tray.hIcon == NULL) g_tray.hIcon = LoadIconW(NULL, (LPCWSTR)IDI_APPLICATION);
  if (MultiByteToWideChar(CP_UTF8, 0, TACHYON_TRAY_TOOLTIP, -1, g_tray.szTip, ARRAYSIZE(g_tray.szTip)) == 0) g_tray.szTip[0] = L'\0';
  tachyon_tray_add();
}

static void tachyon_tray_remove(void) {
  if (g_tray_shown) Shell_NotifyIconW(NIM_DELETE, &g_tray);
  g_tray_shown = FALSE;
}

static void tachyon_tray_show_window(HWND window) {
  ShowWindow(window, IsIconic(window) ? SW_RESTORE : SW_SHOW);
  SetForegroundWindow(window);
}

static void tachyon_tray_append(HMENU popup, UINT command, const char *label, BOOL enabled) {
  wchar_t text[TACHYON_MENU_TEXT_LIMIT];
  if (MultiByteToWideChar(CP_UTF8, 0, label, -1, text, TACHYON_MENU_TEXT_LIMIT) == 0) text[0] = L'\0';
  AppendMenuW(popup, MF_STRING | (enabled ? MF_ENABLED : MF_GRAYED), command, text);
}

static void tachyon_tray_menu(HWND window) {
  HMENU popup = CreatePopupMenu();
  tachyon_tray_append(popup, TACHYON_TRAY_OPEN_COMMAND, TACHYON_TRAY_OPEN_LABEL, TRUE);
  BOOL declared = FALSE;
  for (int index = 0; index < TACHYON_MENU_ITEM_COUNT && index < TACHYON_MENU_LIMIT; index += 1) {
    const TachyonMenuItem *item = &TACHYON_MENU_ITEMS[index];
    if (item->group != TACHYON_MENU_TRAY) continue;
    if (!declared) AppendMenuW(popup, MF_SEPARATOR, 0, NULL);
    declared = TRUE;
    tachyon_tray_append(popup, (UINT)(TACHYON_MENU_FIRST_COMMAND + index), item->label, tachyon_menu_enabled(item));
  }
  AppendMenuW(popup, MF_SEPARATOR, 0, NULL);
  tachyon_tray_append(popup, TACHYON_TRAY_QUIT_COMMAND, TACHYON_TRAY_QUIT_LABEL, TRUE);
  POINT cursor;
  GetCursorPos(&cursor);
  SetForegroundWindow(window);
  TrackPopupMenuEx(popup, TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, cursor.x, cursor.y, window, NULL);
  PostMessageW(window, WM_NULL, 0, 0);
  DestroyMenu(popup);
}

static BOOL tachyon_tray_message(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
  (void)wparam;
  if (g_tray_restarted != 0 && message == g_tray_restarted) {
    Shell_NotifyIconW(NIM_DELETE, &g_tray);
    tachyon_tray_add();
    return TRUE;
  }
  if (message != TACHYON_WM_TRAY) return FALSE;
  UINT event = LOWORD(lparam);
  if (event == NIN_SELECT || event == NIN_KEYSELECT) tachyon_tray_show_window(window);
  else if (event == WM_CONTEXTMENU) tachyon_tray_menu(window);
  return TRUE;
}

static BOOL tachyon_tray_command(HWND window, UINT command) {
  if (command == TACHYON_TRAY_OPEN_COMMAND) {
    tachyon_tray_show_window(window);
    return TRUE;
  }
  if (command == TACHYON_TRAY_QUIT_COMMAND || command == TACHYON_MENU_EXIT_COMMAND) {
    g_tray_quitting = TRUE;
    PostMessageW(window, WM_CLOSE, 0, 0);
    return TRUE;
  }
  int index = (int)command - TACHYON_MENU_FIRST_COMMAND;
  if (index >= 0 && index < TACHYON_MENU_ITEM_COUNT && TACHYON_MENU_ITEMS[index].group == TACHYON_MENU_TRAY &&
      TACHYON_MENU_ITEMS[index].route != NULL) tachyon_tray_show_window(window);
  return FALSE;
}

static BOOL tachyon_tray_hides_window(HWND window) {
#if TACHYON_TRAY_CLOSE_HIDES
  if (!g_tray_shown || g_tray_quitting) return FALSE;
  ShowWindow(window, SW_HIDE);
  return TRUE;
#else
  (void)window;
  return FALSE;
#endif
}
#else
static void tachyon_tray_install(HWND window) {
  (void)window;
}

static const char *tachyon_tray_icon_event(void) {
  return NULL;
}

static void tachyon_tray_remove(void) {}

static BOOL tachyon_tray_message(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
  (void)window;
  (void)message;
  (void)wparam;
  (void)lparam;
  return FALSE;
}

static BOOL tachyon_tray_command(HWND window, UINT command) {
  (void)window;
  (void)command;
  return FALSE;
}

static BOOL tachyon_tray_hides_window(HWND window) {
  (void)window;
  return FALSE;
}
#endif
