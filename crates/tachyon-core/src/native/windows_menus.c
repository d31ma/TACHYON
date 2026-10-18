typedef bool (*TachyonMenuCall)(const char *);

#define TACHYON_MENU_CLOSE_COMMAND 0x7000
#define TACHYON_MENU_EXIT_COMMAND 0x7001
#define TACHYON_MENU_MINIMIZE_COMMAND 0x7002
#define TACHYON_MENU_MAXIMIZE_COMMAND 0x7003
#define TACHYON_MENU_EDIT_COMMAND 0x7010
#define TACHYON_MENU_EDIT_COUNT 6
#define TACHYON_MENU_FIRST_COMMAND 0x7100
#define TACHYON_MENU_TEXT_LIMIT 320

static TachyonMenuCall g_menu_perform = NULL;
static TachyonMenuCall g_menu_ready = NULL;
static HACCEL g_menu_accelerators = NULL;

__NATIVE_MENU_ITEMS__

static void tachyon_menus_bind(HMODULE library) {
  g_menu_perform = (TachyonMenuCall)(void (*)(void))GetProcAddress(library, "tac_native_menu_action");
  g_menu_ready = (TachyonMenuCall)(void (*)(void))GetProcAddress(library, "tac_native_menu_ready");
}

static BOOL tachyon_menu_enabled(const TachyonMenuItem *item) {
  if (item->route != NULL) return tachyon_menu_window_open();
  if (g_menu_perform == NULL) return FALSE;
  if (item->event[0] == '\0') return TRUE;
  return g_menu_ready != NULL && tachyon_menu_page_shown() && g_menu_ready(item->event);
}

static WORD tachyon_menu_key(const TachyonMenuItem *item) {
  return (WORD)toupper((unsigned char)item->key);
}

static void tachyon_menu_text(const TachyonMenuItem *item, wchar_t *text) {
  char narrow[TACHYON_MENU_TEXT_LIMIT];
  if (item->key == 0) snprintf(narrow, sizeof(narrow), "%s", item->label);
  else snprintf(narrow, sizeof(narrow), "%s\t%s%s%s%c", item->label,
                (item->modifiers & TACHYON_MENU_PRIMARY) ? "Ctrl+" : "",
                (item->modifiers & TACHYON_MENU_ALT) ? "Alt+" : "",
                (item->modifiers & TACHYON_MENU_SHIFT) ? "Shift+" : "", (char)tachyon_menu_key(item));
  if (MultiByteToWideChar(CP_UTF8, 0, narrow, -1, text, TACHYON_MENU_TEXT_LIMIT) == 0) text[0] = L'\0';
}

static HMENU tachyon_menu_popup(int group) {
  HMENU popup = CreatePopupMenu();
  wchar_t text[TACHYON_MENU_TEXT_LIMIT];
  for (int index = 0; index < TACHYON_MENU_ITEM_COUNT && index < TACHYON_MENU_LIMIT; index += 1) {
    if (TACHYON_MENU_ITEMS[index].group != group) continue;
    tachyon_menu_text(&TACHYON_MENU_ITEMS[index], text);
    AppendMenuW(popup, MF_STRING, (UINT_PTR)(TACHYON_MENU_FIRST_COMMAND + index), text);
  }
  return popup;
}

static void tachyon_menu_group(HMENU bar, int group, const wchar_t *title) {
  HMENU popup = tachyon_menu_popup(group);
  if (GetMenuItemCount(popup) > 0) AppendMenuW(bar, MF_POPUP, (UINT_PTR)popup, title);
  else DestroyMenu(popup);
}

static const char *tachyon_menu_edit_command(UINT command) {
  static const char *const commands[TACHYON_MENU_EDIT_COUNT] = {"Undo", "Redo", "Cut", "Copy", "Paste", "SelectAll"};
  if (command < TACHYON_MENU_EDIT_COMMAND || command >= TACHYON_MENU_EDIT_COMMAND + TACHYON_MENU_EDIT_COUNT) return NULL;
  return commands[command - TACHYON_MENU_EDIT_COMMAND];
}

static HMENU tachyon_menu_edit_popup(void) {
  static const wchar_t *const labels[TACHYON_MENU_EDIT_COUNT] = {
      L"&Undo\tCtrl+Z", L"&Redo\tCtrl+Y", L"Cu&t\tCtrl+X", L"&Copy\tCtrl+C", L"&Paste\tCtrl+V", L"Select &All\tCtrl+A"};
  HMENU popup = CreatePopupMenu();
  for (int index = 0; index < TACHYON_MENU_EDIT_COUNT; index += 1) {
    if (index == 2 || index == 5) AppendMenuW(popup, MF_SEPARATOR, 0, NULL);
    AppendMenuW(popup, MF_STRING, (UINT_PTR)(TACHYON_MENU_EDIT_COMMAND + index), labels[index]);
  }
  return popup;
}

static HMENU tachyon_menu_window_popup(void) {
  HMENU popup = CreatePopupMenu();
  AppendMenuW(popup, MF_STRING, TACHYON_MENU_MINIMIZE_COMMAND, L"Mi&nimize");
  AppendMenuW(popup, MF_STRING, TACHYON_MENU_MAXIMIZE_COMMAND, L"Ma&ximize");
  return popup;
}

static BYTE tachyon_menu_flags(int modifiers) {
  BYTE flags = FVIRTKEY;
  if (modifiers & TACHYON_MENU_PRIMARY) flags |= FCONTROL;
  if (modifiers & TACHYON_MENU_ALT) flags |= FALT;
  if (modifiers & TACHYON_MENU_SHIFT) flags |= FSHIFT;
  return flags;
}

static void tachyon_menus_install(HWND window) {
  HMENU bar = CreateMenu();
  HMENU file = tachyon_menu_popup(TACHYON_MENU_FILE);
  if (GetMenuItemCount(file) > 0) AppendMenuW(file, MF_SEPARATOR, 0, NULL);
  AppendMenuW(file, MF_STRING, TACHYON_MENU_CLOSE_COMMAND, L"&Close Window\tCtrl+W");
  AppendMenuW(file, MF_STRING, TACHYON_MENU_EXIT_COMMAND, L"E&xit\tCtrl+Q");
  AppendMenuW(bar, MF_POPUP, (UINT_PTR)file, L"&File");
  AppendMenuW(bar, MF_POPUP, (UINT_PTR)tachyon_menu_edit_popup(), L"&Edit");
  tachyon_menu_group(bar, TACHYON_MENU_VIEW, L"&View");
  AppendMenuW(bar, MF_POPUP, (UINT_PTR)tachyon_menu_window_popup(), L"&Window");
  tachyon_menu_group(bar, TACHYON_MENU_HELP, L"&Help");
  SetMenu(window, bar);
  ACCEL accelerators[TACHYON_MENU_LIMIT + 2];
  int count = 0;
  accelerators[count++] = (ACCEL){FVIRTKEY | FCONTROL, 'W', TACHYON_MENU_CLOSE_COMMAND};
  accelerators[count++] = (ACCEL){FVIRTKEY | FCONTROL, 'Q', TACHYON_MENU_EXIT_COMMAND};
  for (int index = 0; index < TACHYON_MENU_ITEM_COUNT && index < TACHYON_MENU_LIMIT; index += 1) {
    const TachyonMenuItem *item = &TACHYON_MENU_ITEMS[index];
    if (item->key == 0) continue;
    accelerators[count++] = (ACCEL){tachyon_menu_flags(item->modifiers), tachyon_menu_key(item), (WORD)(TACHYON_MENU_FIRST_COMMAND + index)};
  }
  g_menu_accelerators = CreateAcceleratorTableW(accelerators, count);
}

static void tachyon_menus_refresh(HMENU popup) {
  for (int index = 0; index < TACHYON_MENU_ITEM_COUNT && index < TACHYON_MENU_LIMIT; index += 1) {
    UINT state = tachyon_menu_enabled(&TACHYON_MENU_ITEMS[index]) ? MF_ENABLED : MF_GRAYED;
    EnableMenuItem(popup, (UINT)(TACHYON_MENU_FIRST_COMMAND + index), MF_BYCOMMAND | state);
  }
}

static BOOL tachyon_menus_command(HWND window, UINT command) {
  if (command == TACHYON_MENU_CLOSE_COMMAND || command == TACHYON_MENU_EXIT_COMMAND) {
    PostMessageW(window, WM_CLOSE, 0, 0);
    return TRUE;
  }
  if (command == TACHYON_MENU_MINIMIZE_COMMAND) {
    ShowWindow(window, SW_MINIMIZE);
    return TRUE;
  }
  if (command == TACHYON_MENU_MAXIMIZE_COMMAND) {
    ShowWindow(window, IsZoomed(window) ? SW_RESTORE : SW_MAXIMIZE);
    return TRUE;
  }
  const char *edit = tachyon_menu_edit_command(command);
  if (edit != NULL) {
    if (!tachyon_menu_edit(edit)) MessageBeep(MB_OK);
    return TRUE;
  }
  int index = (int)command - TACHYON_MENU_FIRST_COMMAND;
  if (index < 0 || index >= TACHYON_MENU_ITEM_COUNT) return FALSE;
  const TachyonMenuItem *item = &TACHYON_MENU_ITEMS[index];
  if (!tachyon_menu_enabled(item)) MessageBeep(MB_OK);
  else if (item->route != NULL) tachyon_menu_open_route(item->route);
  else if (!g_menu_perform(item->id)) MessageBeep(MB_OK);
  return TRUE;
}

static BOOL tachyon_menus_translate(HWND window, MSG *message) {
  return g_menu_accelerators != NULL && TranslateAcceleratorW(window, g_menu_accelerators, message) != 0;
}

static BOOL tachyon_menus_key(HWND window, UINT key) {
  if (g_menu_accelerators == NULL) return FALSE;
  int modifiers = (GetKeyState(VK_CONTROL) < 0 ? TACHYON_MENU_PRIMARY : 0) |
                  (GetKeyState(VK_MENU) < 0 ? TACHYON_MENU_ALT : 0) |
                  (GetKeyState(VK_SHIFT) < 0 ? TACHYON_MENU_SHIFT : 0);
  UINT command = 0;
  if (modifiers == TACHYON_MENU_PRIMARY && key == 'W') command = TACHYON_MENU_CLOSE_COMMAND;
  if (modifiers == TACHYON_MENU_PRIMARY && key == 'Q') command = TACHYON_MENU_EXIT_COMMAND;
  for (int index = 0; index < TACHYON_MENU_ITEM_COUNT && index < TACHYON_MENU_LIMIT; index += 1) {
    const TachyonMenuItem *item = &TACHYON_MENU_ITEMS[index];
    if (item->key != 0 && item->modifiers == modifiers && tachyon_menu_key(item) == key) command = (UINT)(TACHYON_MENU_FIRST_COMMAND + index);
  }
  if (command == 0) return FALSE;
  PostMessageW(window, WM_COMMAND, MAKEWPARAM(command, 1), 0);
  return TRUE;
}

static void tachyon_menus_destroy(void) {
  if (g_menu_accelerators != NULL) DestroyAcceleratorTable(g_menu_accelerators);
  g_menu_accelerators = NULL;
}
