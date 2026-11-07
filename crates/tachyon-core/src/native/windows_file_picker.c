#define TACHYON_FILE_PICKER_BYTES 65536
#define TACHYON_FILE_PICKER_HANDLES 128
#define TACHYON_FILE_PICKER_OWNER_HANDLES 8
#define TACHYON_FILE_PICKER_LIFETIME 300000ULL
#define TACHYON_FILE_PICKER_PATIENCE 30000
#define TACHYON_FILE_PICKER_TEXT 1
#define TACHYON_FILE_PICKER_JSON 2

typedef struct {
  char owner[37];
  char handle[37];
  wchar_t *path;
  ULONGLONG expiry;
} TachyonPickedFile;

typedef HRESULT (*TachyonFileChooser)(HWND window, int types, wchar_t **path);

static char *tachyon_narrow(const wchar_t *value);
static void tachyon_page_messages_resume(void);
static HRESULT tachyon_file_picker_dialog(HWND window, int types, wchar_t **path);

static ULONGLONG WINAPI tachyon_file_picker_tick(void) { return GetTickCount64(); }

static TachyonPickedFile g_picked_files[TACHYON_FILE_PICKER_HANDLES];
static BOOL g_file_picker_granted = __FILE_PICKER_GRANTED__;
static BOOL g_file_picker_open = FALSE;
static BOOL g_file_picker_cancelled = FALSE;
static char g_file_picker_owner[37];
static IFileDialog *g_file_dialog = NULL;
static ULONGLONG (WINAPI *g_file_picker_clock)(void) = tachyon_file_picker_tick;
static TachyonFileChooser g_file_picker_choose = tachyon_file_picker_dialog;
static UINT g_file_picker_patience = TACHYON_FILE_PICKER_PATIENCE;

static BOOL tachyon_file_picker_busy(void) { return g_file_picker_open; }

static char *tachyon_file_picker_answer(const char *status) {
  char *answer = (char *)malloc(64);
  if (answer != NULL) snprintf(answer, 64, "{\"status\":\"%s\"}", status);
  return answer;
}

static int tachyon_file_picker_id(const char *json, const char *key, char *output) {
  size_t index = tachyon_window_field(json, key);
  if (!index || json[index++] != '"') return 0;
  for (int at = 0; at < 36; at++, index++) {
    char ch = json[index];
    int dash = at == 8 || at == 13 || at == 18 || at == 23;
    int hex = (ch >= '0' && ch <= '9') || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F');
    if (dash ? ch != '-' : !hex) return 0;
    output[at] = ch;
  }
  output[36] = '\0';
  if (json[index] != '"') return 0;
  index = tachyon_skip_space(json, index + 1);
  return json[index] == ',' || json[index] == '}';
}

static int tachyon_file_picker_types(const char *json) {
  size_t index = tachyon_window_field(json, "types");
  int types = 0, count = 0;
  if (!index || json[index] != '[') return -1;
  index = tachyon_skip_space(json, index + 1);
  while (json[index] != ']') {
    if (count == 2 || (count > 0 && json[index++] != ',')) return -1;
    index = tachyon_skip_space(json, index);
    if (strncmp(json + index, "\"text/plain\"", 12) == 0) { types |= TACHYON_FILE_PICKER_TEXT; index += 12; }
    else if (strncmp(json + index, "\"application/json\"", 18) == 0) { types |= TACHYON_FILE_PICKER_JSON; index += 18; }
    else return -1;
    index = tachyon_skip_space(json, index);
    count++;
  }
  index = tachyon_skip_space(json, index + 1);
  return json[index] == ',' || json[index] == '}' ? types : -1;
}

static void tachyon_file_picker_remove(int slot) {
  CoTaskMemFree(g_picked_files[slot].path);
  memset(&g_picked_files[slot], 0, sizeof(g_picked_files[slot]));
}

static void tachyon_file_picker_prune(void) {
  ULONGLONG now = g_file_picker_clock();
  for (int slot = 0; slot < TACHYON_FILE_PICKER_HANDLES; slot++) {
    if (g_picked_files[slot].path != NULL && g_picked_files[slot].expiry <= now) tachyon_file_picker_remove(slot);
  }
}

static int tachyon_file_picker_count(const char *owner) {
  int count = 0;
  for (int slot = 0; slot < TACHYON_FILE_PICKER_HANDLES; slot++) {
    if (g_picked_files[slot].path != NULL && (owner == NULL || strcmp(g_picked_files[slot].owner, owner) == 0)) count++;
  }
  return count;
}

static int tachyon_file_picker_find(const char *owner, const char *json) {
  char handle[37];
  if (!tachyon_file_picker_id(json, "handle", handle)) return -1;
  for (int slot = 0; slot < TACHYON_FILE_PICKER_HANDLES; slot++) {
    TachyonPickedFile *file = &g_picked_files[slot];
    if (file->path != NULL && strcmp(file->handle, handle) == 0) return strcmp(file->owner, owner) == 0 ? slot : -1;
  }
  return -1;
}

static void tachyon_file_picker_cancel(void) {
  g_file_picker_cancelled = TRUE;
  if (g_file_dialog != NULL) IFileDialog_Close(g_file_dialog, HRESULT_FROM_WIN32(ERROR_CANCELLED));
}

static void tachyon_file_picker_dispose(const char *owner) {
  if (g_file_picker_open && strcmp(g_file_picker_owner, owner) == 0) tachyon_file_picker_cancel();
  for (int slot = 0; slot < TACHYON_FILE_PICKER_HANDLES; slot++) {
    if (g_picked_files[slot].path != NULL && strcmp(g_picked_files[slot].owner, owner) == 0) tachyon_file_picker_remove(slot);
  }
}

static void tachyon_file_picker_dispose_all(void) {
  if (g_file_picker_open) tachyon_file_picker_cancel();
  for (int slot = 0; slot < TACHYON_FILE_PICKER_HANDLES; slot++) {
    if (g_picked_files[slot].path != NULL) tachyon_file_picker_remove(slot);
  }
}

static char *tachyon_file_picker_json_text(const char *text, size_t length) {
  char *escaped = (char *)malloc(length * 6 + 1);
  size_t used = 0;
  if (escaped == NULL) return NULL;
  for (size_t index = 0; index < length; index++) {
    unsigned char ch = (unsigned char)text[index];
    if (ch == '"' || ch == '\\') { escaped[used++] = '\\'; escaped[used++] = (char)ch; }
    else if (ch == '\n') { escaped[used++] = '\\'; escaped[used++] = 'n'; }
    else if (ch == '\r') { escaped[used++] = '\\'; escaped[used++] = 'r'; }
    else if (ch == '\t') { escaped[used++] = '\\'; escaped[used++] = 't'; }
    else if (ch < 0x20) used += (size_t)snprintf(escaped + used, 7, "\\u%04x", ch);
    else escaped[used++] = (char)ch;
  }
  escaped[used] = '\0';
  return escaped;
}

static char *tachyon_file_picker_keep(const char *owner, wchar_t *path) {
  int slot = 0;
  while (slot < TACHYON_FILE_PICKER_HANDLES && g_picked_files[slot].path != NULL) slot++;
  GUID identifier = {0};
  char *name = tachyon_narrow(PathFindFileNameW(path));
  char *escaped = name != NULL ? tachyon_file_picker_json_text(name, strlen(name)) : NULL;
  char *answer = escaped != NULL ? (char *)malloc(strlen(escaped) + 128) : NULL;
  if (slot == TACHYON_FILE_PICKER_HANDLES || answer == NULL || FAILED(CoCreateGuid(&identifier))) {
    CoTaskMemFree(path); free(name); free(escaped); free(answer);
    return tachyon_file_picker_answer("unavailable");
  }
  TachyonPickedFile *file = &g_picked_files[slot];
  snprintf(file->handle, sizeof(file->handle), "%08lX-%04X-%04X-%02X%02X-%02X%02X%02X%02X%02X%02X",
           (unsigned long)identifier.Data1, identifier.Data2, identifier.Data3, identifier.Data4[0], identifier.Data4[1],
           identifier.Data4[2], identifier.Data4[3], identifier.Data4[4], identifier.Data4[5], identifier.Data4[6], identifier.Data4[7]);
  memcpy(file->owner, owner, sizeof(file->owner));
  file->path = path;
  file->expiry = g_file_picker_clock() + TACHYON_FILE_PICKER_LIFETIME;
  snprintf(answer, strlen(escaped) + 128, "{\"status\":\"selected\",\"handle\":\"%s\",\"name\":\"%s\"}", file->handle, escaped);
  free(name); free(escaped);
  return answer;
}

static char *tachyon_file_picker_open(const char *owner, const char *json) {
  if (g_file_picker_open || tachyon_file_picker_count(NULL) >= TACHYON_FILE_PICKER_HANDLES ||
      tachyon_file_picker_count(owner) >= TACHYON_FILE_PICKER_OWNER_HANDLES) return tachyon_file_picker_answer("unavailable");
  int types = tachyon_file_picker_types(json);
  if (types < 0) return tachyon_file_picker_answer("invalid");
  wchar_t *path = NULL;
  g_file_picker_open = TRUE;
  g_file_picker_cancelled = FALSE;
  memcpy(g_file_picker_owner, owner, sizeof(g_file_picker_owner));
  HRESULT chosen = g_file_picker_choose(g_window, types, &path);
  BOOL cancelled = g_file_picker_cancelled || chosen == HRESULT_FROM_WIN32(ERROR_CANCELLED);
  g_file_picker_open = FALSE;
  g_file_picker_owner[0] = '\0';
  tachyon_page_messages_resume();
  if (cancelled || FAILED(chosen) || path == NULL) {
    CoTaskMemFree(path);
    return tachyon_file_picker_answer(cancelled ? "cancelled" : "unavailable");
  }
  if (!g_file_picker_granted) { CoTaskMemFree(path); return tachyon_file_picker_answer("denied"); }
  return tachyon_file_picker_keep(owner, path);
}

static char *tachyon_file_picker_read(const char *owner, const char *json) {
  int slot = tachyon_file_picker_find(owner, json);
  if (slot < 0) return tachyon_file_picker_answer("invalid");
  HANDLE stream = CreateFileW(g_picked_files[slot].path, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                              NULL, OPEN_EXISTING, FILE_FLAG_SEQUENTIAL_SCAN, NULL);
  char *data = stream != INVALID_HANDLE_VALUE ? (char *)malloc(TACHYON_FILE_PICKER_BYTES + 1) : NULL;
  DWORD total = 0, count = 1;
  BOOL readable = data != NULL;
  while (readable && count > 0 && total <= TACHYON_FILE_PICKER_BYTES) {
    readable = ReadFile(stream, data + total, TACHYON_FILE_PICKER_BYTES + 1 - total, &count, NULL);
    total += readable ? count : 0;
  }
  if (stream != INVALID_HANDLE_VALUE) CloseHandle(stream);
  if (!readable) { free(data); return tachyon_file_picker_answer("unavailable"); }
  if (total > TACHYON_FILE_PICKER_BYTES) { free(data); return tachyon_file_picker_answer("tooLarge"); }
  if (total > 0 && MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, data, (int)total, NULL, 0) == 0) {
    free(data);
    return tachyon_file_picker_answer("invalid");
  }
  char *escaped = tachyon_file_picker_json_text(data, total);
  free(data);
  char *answer = escaped != NULL ? (char *)malloc(strlen(escaped) + 32) : NULL;
  if (answer != NULL) snprintf(answer, strlen(escaped) + 32, "{\"status\":\"read\",\"text\":\"%s\"}", escaped);
  free(escaped);
  return answer != NULL ? answer : tachyon_file_picker_answer("unavailable");
}

static char *tachyon_file_picker_release(const char *owner, const char *json) {
  int slot = tachyon_file_picker_find(owner, json);
  if (slot < 0) return tachyon_file_picker_answer("invalid");
  tachyon_file_picker_remove(slot);
  return tachyon_file_picker_answer("released");
}

static char *tachyon_file_picker(const char *operation, const char *payload) {
  char owner[37];
  if (!tachyon_file_picker_id(payload, "owner", owner)) return tachyon_file_picker_answer("invalid");
  if (g_window == NULL || GetWindowThreadProcessId(g_window, NULL) != GetCurrentThreadId()) return tachyon_file_picker_answer("unavailable");
  tachyon_file_picker_prune();
  if (strcmp(operation, "dispose") == 0) { tachyon_file_picker_dispose(owner); return tachyon_file_picker_answer("disposed"); }
  if (!g_file_picker_granted) { tachyon_file_picker_dispose(owner); return tachyon_file_picker_answer("denied"); }
  if (strcmp(operation, "available") == 0) return tachyon_file_picker_answer("available");
  if (strcmp(operation, "open") == 0) return tachyon_file_picker_open(owner, payload);
  if (strcmp(operation, "readText") == 0) return tachyon_file_picker_read(owner, payload);
  if (strcmp(operation, "release") == 0) return tachyon_file_picker_release(owner, payload);
  return tachyon_file_picker_answer("invalid");
}

static void CALLBACK tachyon_file_picker_timeout(HWND window, UINT message, UINT_PTR timer, DWORD time) {
  (void)window; (void)message; (void)time;
  IOleWindow *shown = NULL;
  HWND dialog = NULL;
  if (g_file_dialog == NULL) { KillTimer(NULL, timer); return; }
  SetTimer(NULL, timer, 250, tachyon_file_picker_timeout);
  IFileDialog_Close(g_file_dialog, HRESULT_FROM_WIN32(ERROR_CANCELLED));
  if (SUCCEEDED(IFileDialog_QueryInterface(g_file_dialog, &IID_IOleWindow, (void **)&shown))) {
    if (SUCCEEDED(IOleWindow_GetWindow(shown, &dialog)) && dialog != NULL) PostMessageW(dialog, WM_COMMAND, IDCANCEL, 0);
    IOleWindow_Release(shown);
  }
}

static HRESULT tachyon_file_picker_dialog(HWND window, int types, wchar_t **path) {
  static const COMDLG_FILTERSPEC filters[] = {
      {L"Text", L"*.txt;*.text"}, {L"JSON", L"*.json"}, {L"Text and JSON", L"*.txt;*.text;*.json"}};
  IFileOpenDialog *dialog = NULL;
  FILEOPENDIALOGOPTIONS options = 0;
  IShellItem *item = NULL;
  HRESULT result = CoCreateInstance(&CLSID_FileOpenDialog, NULL, CLSCTX_INPROC_SERVER, &IID_IFileOpenDialog, (void **)&dialog);
  if (FAILED(result)) return result;
  if (SUCCEEDED(IFileOpenDialog_GetOptions(dialog, &options))) {
    options |= FOS_FORCEFILESYSTEM | FOS_FILEMUSTEXIST | FOS_PATHMUSTEXIST | FOS_NOCHANGEDIR | FOS_DONTADDTORECENT;
    result = IFileOpenDialog_SetOptions(dialog, options & ~(FILEOPENDIALOGOPTIONS)FOS_ALLOWMULTISELECT);
  }
  if (SUCCEEDED(result) && types != 0) result = IFileOpenDialog_SetFileTypes(dialog, 1, &filters[types - 1]);
  if (SUCCEEDED(result)) {
    g_file_dialog = (IFileDialog *)dialog;
    UINT_PTR timer = SetTimer(NULL, 0, g_file_picker_patience, tachyon_file_picker_timeout);
    result = g_file_picker_cancelled ? HRESULT_FROM_WIN32(ERROR_CANCELLED) : IFileOpenDialog_Show(dialog, window);
    if (timer != 0) KillTimer(NULL, timer);
    g_file_dialog = NULL;
  }
  if (SUCCEEDED(result)) result = IFileOpenDialog_GetResult(dialog, &item);
  if (SUCCEEDED(result)) {
    result = IShellItem_GetDisplayName(item, SIGDN_FILESYSPATH, path);
    IShellItem_Release(item);
  }
  IFileOpenDialog_Release(dialog);
  return result;
}
