#ifndef UNICODE
#define UNICODE
#endif
#define WIN32_LEAN_AND_MEAN
#define COBJMACROS
#include <windows.h>
#include <objbase.h>
#include <shlwapi.h>
#include <shobjidl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#define TACHYON_MESSAGE_LIMIT 65536
#define TACHYON_WM_RESUME (WM_APP + 2)
static HWND g_window = NULL;
__LOCAL_JSON__
__WINDOW_PAYLOAD__
__FILE_PICKER__
static void tachyon_on_message(const wchar_t *raw);
__PAGE_MESSAGES__

static int failures;
static char g_log[512];
static int g_chooser_calls;
static wchar_t g_chosen[MAX_PATH];
static HRESULT g_chooser_result;
static ULONGLONG g_now = 1000000;
static const char *OWNER = "00000000-0000-4000-8000-000000000000";
static const char *OTHER = "00000000-0000-4000-8000-000000000001";

static void expect(int condition, const char *what) {
  if (!condition) { fprintf(stderr, "FAILED: %s\n", what); failures++; }
}

static char *tachyon_narrow(const wchar_t *value) {
  int length = WideCharToMultiByte(CP_UTF8, 0, value, -1, NULL, 0, NULL, NULL);
  char *narrow = length > 0 ? (char *)calloc((size_t)length, 1) : NULL;
  if (narrow != NULL) WideCharToMultiByte(CP_UTF8, 0, value, -1, narrow, length, NULL, NULL);
  return narrow;
}

static ULONGLONG WINAPI fake_clock(void) { return g_now; }

static void logged(const char *event) {
  if (g_log[0] != '\0') strcat_s(g_log, sizeof(g_log), " ");
  strcat_s(g_log, sizeof(g_log), event);
}

static void pump(void) {
  MSG message;
  while (PeekMessageW(&message, NULL, 0, 0, PM_REMOVE)) { TranslateMessage(&message); DispatchMessageW(&message); }
}

static HRESULT fake_chooser(HWND window, int types, wchar_t **path) {
  (void)window; (void)types;
  g_chooser_calls++;
  if (FAILED(g_chooser_result)) return g_chooser_result;
  return SHStrDupW(g_chosen, path);
}

static char *call(const char *operation, const char *owner, const char *extra) {
  char payload[512];
  snprintf(payload, sizeof(payload), "{\"owner\":\"%s\"%s}", owner, extra);
  return tachyon_file_picker(operation, payload);
}

static int answers(const char *operation, const char *owner, const char *extra, const char *expected) {
  char *answer = call(operation, owner, extra);
  int same = answer != NULL && strcmp(answer, expected) == 0;
  if (!same) fprintf(stderr, "%s answered %s, expected %s\n", operation, answer != NULL ? answer : "(null)", expected);
  free(answer);
  return same;
}

static int opened(const char *owner, char *handle) {
  char *answer = call("open", owner, ",\"types\":[\"text/plain\"]");
  const char *at = answer != NULL ? strstr(answer, "\"handle\":\"") : NULL;
  if (at != NULL) { memcpy(handle, at + 10, 36); handle[36] = '\0'; }
  free(answer);
  return at != NULL;
}

static char *handle_field(const char *handle) {
  static char field[64];
  snprintf(field, sizeof(field), ",\"handle\":\"%s\"", handle);
  return field;
}

static void write_file(const wchar_t *path, const char *data, size_t length) {
  HANDLE file = CreateFileW(path, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
  DWORD written = 0;
  WriteFile(file, data, (DWORD)length, &written, NULL);
  CloseHandle(file);
}

static void tachyon_on_message(const wchar_t *raw) {
  char *text = tachyon_narrow(raw);
  if (text != NULL && strcmp(text, "open") == 0) {
    logged("open<");
    expect(answers("open", OWNER, ",\"types\":[]", "{\"status\":\"cancelled\"}"), "a nested open answers cancelled");
    logged(">open");
  } else if (text != NULL) logged(text);
  free(text);
}

static HRESULT nested_chooser(HWND window, int types, wchar_t **path) {
  (void)types; (void)path;
  tachyon_page_message_hold(L"c");
  PostMessageW(window, TACHYON_WM_RESUME, 0, 0);
  pump();
  logged("chooser");
  return HRESULT_FROM_WIN32(ERROR_CANCELLED);
}

static LRESULT CALLBACK procedure(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
  if (message == TACHYON_WM_PAGE_MESSAGE) { tachyon_page_messages_run(); return 0; }
  if (message == TACHYON_WM_RESUME) { if (!tachyon_page_resume_waits()) logged("resume"); return 0; }
  return DefWindowProcW(window, message, wparam, lparam);
}

static DWORD WINAPI elsewhere(void *unused) {
  (void)unused;
  char *answer = call("available", OWNER, "");
  int refused = answer != NULL && strcmp(answer, "{\"status\":\"unavailable\"}") == 0;
  free(answer);
  return (DWORD)refused;
}

static void contract(const wchar_t *folder) {
  wchar_t text[MAX_PATH], large[MAX_PATH], broken[MAX_PATH];
  swprintf(text, MAX_PATH, L"%ls\\café notes.txt", folder);
  swprintf(large, MAX_PATH, L"%ls\\large.txt", folder);
  swprintf(broken, MAX_PATH, L"%ls\\broken.txt", folder);
  const char content[] = "say \"hi\"\\\n\tcaf\xc3\xa9\x01";
  write_file(text, content, sizeof(content) - 1);
  char *bytes = (char *)malloc(TACHYON_FILE_PICKER_BYTES + 1);
  memset(bytes, 'a', TACHYON_FILE_PICKER_BYTES + 1);
  write_file(large, bytes, TACHYON_FILE_PICKER_BYTES);
  write_file(broken, "\xc3\x28", 2);
  g_file_picker_choose = fake_chooser;
  g_file_picker_clock = fake_clock;

  expect(answers("available", "not-a-uuid", "", "{\"status\":\"invalid\"}"), "an owner that is not a UUID is invalid");
  expect(answers("available", OWNER, "", "{\"status\":\"available\"}"), "a granted picker is available");
  expect(answers("save", OWNER, "", "{\"status\":\"invalid\"}"), "an unknown operation is invalid");
  expect(answers("open", OWNER, ",\"types\":[\"image/png\"]", "{\"status\":\"invalid\"}"), "an unknown type is invalid");
  expect(answers("open", OWNER, ",\"types\":[\"text/plain\",\"application/json\",\"text/plain\"]", "{\"status\":\"invalid\"}"), "three types are invalid");
  expect(answers("open", OWNER, "", "{\"status\":\"invalid\"}"), "an open without types is invalid");
  expect(g_chooser_calls == 0, "an invalid open shows no picker");

  wcscpy_s(g_chosen, MAX_PATH, text);
  char *answer = call("open", OWNER, ",\"types\":[\"text/plain\",\"application/json\"]");
  expect(answer != NULL && strstr(answer, "\"status\":\"selected\"") != NULL, "a chosen file is selected");
  expect(answer != NULL && strstr(answer, "\"name\":\"caf\xc3\xa9 notes.txt\"}") != NULL, "a selection names only the file");
  expect(answer != NULL && strstr(answer, "\\\\") == NULL && strstr(answer, ":\\") == NULL, "a selection has no path");
  free(answer);
  char handle[37] = {0};
  expect(opened(OWNER, handle), "a second file opens");
  expect(answers("readText", OWNER, handle_field(handle), "{\"status\":\"read\",\"text\":\"say \\\"hi\\\"\\\\\\n\\tcaf\xc3\xa9\\u0001\"}"), "readText answers escaped UTF-8 text");
  expect(answers("readText", OTHER, handle_field(handle), "{\"status\":\"invalid\"}"), "another owner cannot read the handle");
  expect(answers("readText", OWNER, handle_field("00000000-0000-4000-8000-00000000ffff"), "{\"status\":\"invalid\"}"), "an unknown handle is invalid");
  expect(answers("release", OWNER, handle_field(handle), "{\"status\":\"released\"}"), "a handle is released");
  expect(answers("release", OWNER, handle_field(handle), "{\"status\":\"invalid\"}"), "a released handle is invalid");

  wcscpy_s(g_chosen, MAX_PATH, large);
  expect(opened(OWNER, handle), "the 65536-byte file opens");
  answer = call("readText", OWNER, handle_field(handle));
  expect(answer != NULL && strncmp(answer, "{\"status\":\"read\"", 16) == 0 && strlen(answer) == TACHYON_FILE_PICKER_BYTES + 27, "65536 bytes read");
  free(answer);
  write_file(large, bytes, TACHYON_FILE_PICKER_BYTES + 1);
  expect(answers("readText", OWNER, handle_field(handle), "{\"status\":\"tooLarge\"}"), "65537 bytes are too large");
  wcscpy_s(g_chosen, MAX_PATH, broken);
  expect(opened(OWNER, handle), "the broken file opens");
  expect(answers("readText", OWNER, handle_field(handle), "{\"status\":\"invalid\"}"), "text that is not UTF-8 is invalid");
  free(bytes);

  expect(answers("dispose", OWNER, "", "{\"status\":\"disposed\"}"), "an owner is disposed");
  expect(tachyon_file_picker_count(NULL) == 0, "disposal removes the owner's handles");
  wcscpy_s(g_chosen, MAX_PATH, text);
  char owner[37];
  for (int index = 0; index < TACHYON_FILE_PICKER_HANDLES; index++) {
    snprintf(owner, sizeof(owner), "00000000-0000-4000-8000-%012d", index / TACHYON_FILE_PICKER_OWNER_HANDLES);
    opened(owner, handle);
  }
  expect(tachyon_file_picker_count(NULL) == TACHYON_FILE_PICKER_HANDLES, "the fixture fills every handle");
  int before = g_chooser_calls;
  expect(answers("open", "00000000-0000-4000-8000-000000000099", ",\"types\":[]", "{\"status\":\"unavailable\"}"), "the handle after the limit is refused");
  expect(g_chooser_calls == before && tachyon_file_picker_count(NULL) == TACHYON_FILE_PICKER_HANDLES, "a refused open shows no picker and adds no handle");
  tachyon_file_picker_dispose_all();
  for (int index = 0; index < TACHYON_FILE_PICKER_OWNER_HANDLES; index++) opened(OWNER, handle);
  expect(answers("open", OWNER, ",\"types\":[]", "{\"status\":\"unavailable\"}"), "one owner cannot pass its own limit");
  tachyon_file_picker_dispose_all();

  expect(opened(OWNER, handle), "a file opens for the clock check");
  g_now += TACHYON_FILE_PICKER_LIFETIME - 1;
  expect(answers("readText", OWNER, handle_field(handle), "{\"status\":\"read\",\"text\":\"say \\\"hi\\\"\\\\\\n\\tcaf\xc3\xa9\\u0001\"}"), "a handle works until its lifetime ends");
  g_now += 1;
  expect(answers("readText", OWNER, handle_field(handle), "{\"status\":\"invalid\"}"), "an expired handle is invalid");
  expect(tachyon_file_picker_count(NULL) == 0, "an expired handle is removed");

  g_chooser_result = HRESULT_FROM_WIN32(ERROR_CANCELLED);
  expect(answers("open", OWNER, ",\"types\":[]", "{\"status\":\"cancelled\"}"), "a cancelled picker answers cancelled");
  g_chooser_result = E_FAIL;
  expect(answers("open", OWNER, ",\"types\":[]", "{\"status\":\"unavailable\"}"), "a failed picker answers unavailable");
  g_chooser_result = S_OK;

  expect(opened(OWNER, handle), "a file opens before the grant is removed");
  g_file_picker_granted = FALSE;
  expect(answers("available", OWNER, "", "{\"status\":\"denied\"}"), "a picker without the grant is denied");
  expect(tachyon_file_picker_count(NULL) == 0, "a denied owner loses its handles");
  g_file_picker_granted = TRUE;

  HANDLE thread = CreateThread(NULL, 0, elsewhere, NULL, 0, NULL);
  DWORD refused = 0;
  WaitForSingleObject(thread, INFINITE);
  GetExitCodeThread(thread, &refused);
  CloseHandle(thread);
  expect(refused == 1, "a call from another thread is unavailable");
  printf("windows file picker contract passed\n");
}

static void queue(void) {
  g_log[0] = '\0';
  g_file_picker_choose = nested_chooser;
  tachyon_page_message_hold(L"a");
  tachyon_page_message_hold(L"open");
  tachyon_page_message_hold(L"b");
  pump();
  expect(strcmp(g_log, "a open< chooser >open b c resume") == 0, "page messages wait for the picker and keep their order");
  if (strcmp(g_log, "a open< chooser >open b c resume") != 0) fprintf(stderr, "log: %s\n", g_log);
  g_log[0] = '\0';
  tachyon_page_message_hold(L"dropped");
  tachyon_page_messages_clear();
  pump();
  expect(g_log[0] == '\0', "cleared page messages are not handled");
  printf("windows page messages passed\n");
}

static BOOL CALLBACK dialog_window(HWND window, LPARAM found) {
  wchar_t name[16];
  if (GetWindow(window, GW_OWNER) == g_window && GetClassNameW(window, name, 16) && wcscmp(name, L"#32770") == 0) {
    *(HWND *)found = window;
    return FALSE;
  }
  return TRUE;
}

static BOOL CALLBACK file_name_box(HWND window, LPARAM found) {
  wchar_t name[16], parent[16];
  if (GetClassNameW(window, name, 16) && wcscmp(name, L"Edit") == 0 && IsWindowVisible(window) &&
      GetClassNameW(GetParent(window), parent, 16) && (wcscmp(parent, L"ComboBox") == 0 || wcscmp(parent, L"#32770") == 0)) {
    *(HWND *)found = window;
    return FALSE;
  }
  return TRUE;
}

static DWORD WINAPI choose_in_dialog(void *path) {
  for (int attempt = 0; attempt < 200; attempt++) {
    HWND dialog = NULL, box = NULL;
    EnumWindows(dialog_window, (LPARAM)&dialog);
    if (dialog != NULL) EnumChildWindows(dialog, file_name_box, (LPARAM)&box);
    if (box != NULL) {
      Sleep(500);
      SendMessageW(box, WM_SETTEXT, 0, (LPARAM)path);
      PostMessageW(dialog, WM_COMMAND, MAKEWPARAM(IDOK, BN_CLICKED), (LPARAM)GetDlgItem(dialog, IDOK));
      return 0;
    }
    Sleep(100);
  }
  return 1;
}

static void dialog(const wchar_t *folder) {
  wchar_t path[MAX_PATH];
  swprintf(path, MAX_PATH, L"%ls\\chosen.txt", folder);
  write_file(path, "chosen in the dialog", 20);
  g_file_picker_choose = tachyon_file_picker_dialog;
  g_file_picker_clock = tachyon_file_picker_tick;
  HANDLE helper = CreateThread(NULL, 0, choose_in_dialog, path, 0, NULL);
  char *answer = call("open", OWNER, ",\"types\":[\"text/plain\"]");
  WaitForSingleObject(helper, INFINITE);
  CloseHandle(helper);
  const char *at = answer != NULL ? strstr(answer, "\"handle\":\"") : NULL;
  expect(at != NULL && strstr(answer, "\"name\":\"chosen.txt\"") != NULL, "the real dialog selects the typed file");
  if (at == NULL) fprintf(stderr, "open answered %s\n", answer != NULL ? answer : "(null)");
  char handle[37] = {0};
  if (at != NULL) memcpy(handle, at + 10, 36);
  free(answer);
  expect(answers("readText", OWNER, handle_field(handle), "{\"status\":\"read\",\"text\":\"chosen in the dialog\"}"), "the dialog's file reads back");
  g_file_picker_patience = 1500;
  ULONGLONG started = GetTickCount64();
  expect(answers("open", OWNER, ",\"types\":[]", "{\"status\":\"cancelled\"}"), "an unanswered dialog cancels after its patience");
  expect(GetTickCount64() - started < 15000, "the dialog closes soon after its patience");
  printf("windows file dialog passed\n");
}

int wmain(int count, wchar_t **arguments) {
  wchar_t folder[MAX_PATH];
  DWORD length = GetCurrentDirectoryW(MAX_PATH, folder);
  if (count < 2 || length == 0 || length > MAX_PATH - 64 || FAILED(CoInitializeEx(NULL, COINIT_APARTMENTTHREADED))) return 2;
  WNDCLASSW type = {0};
  type.lpfnWndProc = procedure;
  type.hInstance = GetModuleHandleW(NULL);
  type.lpszClassName = L"TachyonFilePickerCheck";
  RegisterClassW(&type);
  g_window = CreateWindowExW(0, L"TachyonFilePickerCheck", L"File picker check", WS_OVERLAPPEDWINDOW,
                             CW_USEDEFAULT, CW_USEDEFAULT, 640, 480, NULL, NULL, type.hInstance, NULL);
  if (g_window == NULL) return 2;
  if (wcscmp(arguments[1], L"dialog") == 0) { ShowWindow(g_window, SW_SHOWNORMAL); dialog(folder); }
  else { contract(folder); queue(); }
  DestroyWindow(g_window);
  CoUninitialize();
  return failures == 0 ? 0 : 1;
}
