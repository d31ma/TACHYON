#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#include <stdint.h>
#define _strdup strdup
#define TACHYON_MESSAGE_LIMIT 65536
#define FALSE 0
#define TRUE 1
#define SW_MINIMIZE 1
#define SW_RESTORE 2
#define SW_MAXIMIZE 3
#define WM_CLOSE 4
#define GWL_STYLE 5
#define SWP_NOMOVE 1
#define SWP_NOZORDER 2
#define SWP_NOOWNERZORDER 4
#define SWP_FRAMECHANGED 8
#define WS_OVERLAPPEDWINDOW 16
#define MONITOR_DEFAULTTONEAREST 1
#define HWND_TOP NULL
typedef intptr_t LONG_PTR;
typedef void *HWND;
typedef int BOOL;
typedef unsigned long DWORD;
typedef struct { int left, top, right, bottom; } RECT;
typedef struct { size_t cbSize; RECT rcMonitor; } MONITORINFO;
static void *g_window = (void *)1;
static int calls, owner_thread = 1;
static unsigned long GetWindowThreadProcessId(void *window, void *process) { (void)window; (void)process; return 1; }
static unsigned long GetCurrentThreadId(void) { return (unsigned long)owner_thread; }
static void ShowWindow(void *window, int action) { (void)window; (void)action; calls++; }
static int IsZoomed(void *window) { (void)window; return 0; }
static void PostMessageW(void *window, int message, int first, int second) { (void)window; (void)message; (void)first; (void)second; calls++; }
static LONG_PTR GetWindowLongPtrW(void *window, int style) { (void)window; (void)style; return 16; }
static void SetWindowLongPtrW(void *window, int style, LONG_PTR value) { (void)window; (void)style; (void)value; }
static void AdjustWindowRect(RECT *rect, DWORD style, int menu) { (void)rect; (void)style; (void)menu; }
static void SetWindowPos(void *window, void *previous, int x, int y, int width, int height, int flags) { (void)window; (void)previous; (void)x; (void)y; (void)width; (void)height; (void)flags; calls++; }
static void SetWindowTextW(void *window, const wchar_t *wide) { (void)window; const uint16_t *text = (const uint16_t *)wide; assert(text[0] == 'A' && text[1] == '"' && text[2] == 0xD83D && text[3] == 0xDE00); calls++; }
static int GetWindowRect(void *window, RECT *bounds) { (void)window; *bounds = (RECT){0,0,100,100}; return 1; }
static void *MonitorFromWindow(void *window, int mode) { (void)window; (void)mode; return NULL; }
static int GetMonitorInfoW(void *monitor, MONITORINFO *info) { (void)monitor; info->rcMonitor = (RECT){0,0,1000,1000}; return 1; }
__LOCAL_JSON__
__WINDOW_HOST__
static void check(const char *operation, const char *request, const char *expected) {
  char *answer = tachyon_window_control(operation, request);
  assert(strcmp(answer, expected) == 0);
  free(answer);
}
int main(void) {
  check("window.granted", "{\"control\":\"minimize\"}", "{\"granted\":true}");
  check("window.granted", "{\"control\":\"close\"}", "{\"granted\":false}");
  check("window.close", "{}", "{\"ok\":false,\"error\":\"Window control is not granted.\"}"); assert(calls == 0);
  check("window.minimize", "{}", "{\"ok\":true}"); assert(calls == 1);
  check("window.resize", "{\"width\":64,\"height\":16384}", "{\"ok\":true}"); assert(calls == 2);
  check("window.resize", "{\"width\":64.5,\"height\":100}", "{\"ok\":false,\"error\":\"Invalid window size.\"}");
  check("window.resize", "{\"width\":064,\"height\":100}", "{\"ok\":false,\"error\":\"Invalid window size.\"}");
  check("window.resize", "{\"width\":64,\"width\":128,\"height\":100}", "{\"ok\":false,\"error\":\"Invalid window size.\"}");
  check("window.title", "{\"title\":\"A\\\"\\uD83D\\uDE00\"}", "{\"ok\":true}"); assert(calls == 3);
  check("window.title", "{\"title\":\"\\u0000\"}", "{\"ok\":false,\"error\":\"Invalid window title.\"}");
  check("window.fullscreen", "{}", "{\"ok\":true}");
  check("window.fullscreen", "{}", "{\"ok\":true}"); assert(calls == 5);
  owner_thread = 2;
  check("window.minimize", "{}", "{\"ok\":false,\"error\":\"Native window is unavailable.\"}"); assert(calls == 5);
  return 0;
}
