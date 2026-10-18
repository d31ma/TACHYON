#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#define FALSE 0
#define TRUE 1
#define GWL_STYLE 5
#define GWL_EXSTYLE 6
#define SWP_NOMOVE 1
#define SWP_NOZORDER 2
#define SWP_NOOWNERZORDER 4
#define SWP_FRAMECHANGED 8
#define SWP_NOACTIVATE 16
#define WS_OVERLAPPEDWINDOW 16
#define MONITOR_DEFAULTTONEAREST 1
#define HWND_TOP NULL
#define USER_DEFAULT_SCREEN_DPI 96
#define SW_SHOWNORMAL 1
#define SW_SHOWMAXIMIZED 3
#define RGB(red, green, blue) ((COLORREF)((unsigned)(red) | ((unsigned)(green) << 8) | ((unsigned)(blue) << 16)))
typedef intptr_t LONG_PTR;
typedef unsigned long DWORD;
typedef unsigned int UINT;
typedef DWORD COLORREF;
typedef void *HWND;
typedef int BOOL;
typedef struct { int left, top, right, bottom; } RECT;
typedef struct { long cx, cy; } SIZE;
typedef struct { long x, y; } POINT;
typedef struct { POINT ptReserved, ptMaxSize, ptMaxPosition, ptMinTrackSize, ptMaxTrackSize; } MINMAXINFO;
typedef struct { size_t cbSize; RECT rcMonitor; } MONITORINFO;
static UINT dpi = 144;
static int menu_present = 1, placed_width, placed_height, placed_flags, fullscreen_moves;
static LONG_PTR style = WS_OVERLAPPEDWINDOW | 32;
static DWORD caption_attribute, text_attribute;
static COLORREF caption_color, text_color;
static UINT GetDpiForWindow(HWND window) { (void)window; return dpi; }
static int MulDiv(int number, int numerator, int denominator) { return (int)(((long long)number * numerator + denominator / 2) / denominator); }
static void *GetMenu(HWND window) { (void)window; return menu_present ? (void *)1 : NULL; }
static LONG_PTR GetWindowLongPtrW(HWND window, int index) { (void)window; return index == GWL_STYLE ? style : 0; }
static void SetWindowLongPtrW(HWND window, int index, LONG_PTR value) { (void)window; (void)index; style = value; }
static BOOL AdjustWindowRectExForDpi(RECT *bounds, DWORD frame, BOOL menu, DWORD extended, UINT scale) {
  (void)frame; (void)extended;
  int border = MulDiv(8, (int)scale, 96), caption = MulDiv(31, (int)scale, 96), bar = menu ? MulDiv(20, (int)scale, 96) : 0;
  bounds->left -= border; bounds->right += border; bounds->top -= caption + bar; bounds->bottom += border;
  return TRUE;
}
static void SetWindowPos(HWND window, void *after, int x, int y, int width, int height, int flags) {
  (void)window; (void)after; (void)x; (void)y;
  if (flags & SWP_FRAMECHANGED) { fullscreen_moves++; return; }
  placed_width = width; placed_height = height; placed_flags = flags;
}
static int GetWindowRect(HWND window, RECT *bounds) { (void)window; *bounds = (RECT){0, 0, 100, 100}; return 1; }
static void *MonitorFromWindow(HWND window, int mode) { (void)window; (void)mode; return NULL; }
static int GetMonitorInfoW(void *monitor, MONITORINFO *info) { (void)monitor; info->rcMonitor = (RECT){0, 0, 1000, 1000}; return 1; }
static long DwmSetWindowAttribute(HWND window, DWORD attribute, const void *value, DWORD size) {
  (void)window; assert(size == sizeof(COLORREF));
  if (caption_attribute == 0) { caption_attribute = attribute; caption_color = *(const COLORREF *)value; }
  else { text_attribute = attribute; text_color = *(const COLORREF *)value; }
  return 0;
}
__WINDOW_FULLSCREEN__
__WINDOW_FRAME__
int main(void) {
  HWND window = (HWND)1;
  int show = tachyon_window_open(window, SW_SHOWMAXIMIZED);
  assert(placed_width == 1200 + 2 * 12 && placed_height == 900 + 47 + 30 + 12);
  assert(placed_flags == (SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE));
  assert(caption_attribute == 35 && caption_color == RGB(15, 108, 189));
  assert(text_attribute == 36 && text_color == RGB(255, 255, 255));
  assert(fullscreen_moves == 1 && (style & WS_OVERLAPPEDWINDOW) == 0 && show == SW_SHOWNORMAL);
  MINMAXINFO limits = {0};
  limits.ptMinTrackSize.x = 100; limits.ptMinTrackSize.y = 5000;
  dpi = 96; menu_present = 0; style = WS_OVERLAPPEDWINDOW;
  tachyon_window_limits(window, &limits);
  assert(limits.ptMinTrackSize.x == 480 + 16 && limits.ptMinTrackSize.y == 5000);
  limits.ptMinTrackSize.y = 0;
  tachyon_window_limits(window, &limits);
  assert(limits.ptMinTrackSize.y == 360 + 31 + 8);
  return 0;
}
