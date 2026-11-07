#define UNICODE
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <shellapi.h>
#include <stdio.h>
#include <stdlib.h>
#include <wchar.h>

typedef struct {
  DWORD process;
  HWND window;
} WindowSearch;

typedef struct {
  HDC screen;
  HDC memory;
  HBITMAP bitmap;
  HGDIOBJ previous;
  BYTE *pixels;
  int width;
  int height;
} Canvas;

static int fail(const char *message) {
  fprintf(stderr, "WindowsIconCheck: %s\n", message);
  return 1;
}

static BOOL canvas_open(Canvas *canvas, int width, int height) {
  BITMAPINFO info = {0};
  info.bmiHeader.biSize = sizeof(info.bmiHeader);
  info.bmiHeader.biWidth = width;
  info.bmiHeader.biHeight = -height;
  info.bmiHeader.biPlanes = 1;
  info.bmiHeader.biBitCount = 32;
  info.bmiHeader.biCompression = BI_RGB;
  void *bits = NULL;
  canvas->screen = GetDC(NULL);
  canvas->memory = CreateCompatibleDC(canvas->screen);
  canvas->bitmap = CreateDIBSection(canvas->memory, &info, DIB_RGB_COLORS, &bits, NULL, 0);
  if (canvas->bitmap == NULL) return FALSE;
  canvas->previous = SelectObject(canvas->memory, canvas->bitmap);
  canvas->pixels = (BYTE *)bits;
  canvas->width = width;
  canvas->height = height;
  ZeroMemory(canvas->pixels, (size_t)width * (size_t)height * 4);
  return TRUE;
}

static void canvas_close(Canvas *canvas) {
  if (canvas->bitmap != NULL) {
    SelectObject(canvas->memory, canvas->previous);
    DeleteObject(canvas->bitmap);
  }
  DeleteDC(canvas->memory);
  ReleaseDC(NULL, canvas->screen);
}

static BYTE *canvas_pixel(const Canvas *canvas, int x, int y) {
  return canvas->pixels + ((size_t)y * (size_t)canvas->width + (size_t)x) * 4;
}

static BOOL icon_center(HICON icon, int color[3]) {
  Canvas canvas = {0};
  BOOL drawn = canvas_open(&canvas, 64, 64) && DrawIconEx(canvas.memory, 0, 0, icon, 64, 64, 0, NULL, DI_NORMAL);
  if (drawn) {
    BYTE *pixel = canvas_pixel(&canvas, 32, 32);
    color[0] = pixel[2];
    color[1] = pixel[1];
    color[2] = pixel[0];
  }
  canvas_close(&canvas);
  return drawn;
}

static BOOL CALLBACK window_of_process(HWND window, LPARAM data) {
  WindowSearch *search = (WindowSearch *)data;
  DWORD process = 0;
  GetWindowThreadProcessId(window, &process);
  if (process != search->process || !IsWindowVisible(window) || GetWindow(window, GW_OWNER) != NULL) return TRUE;
  search->window = window;
  return FALSE;
}

static int taskbar_pixels(const int color[3]) {
  RECT work = {0};
  SystemParametersInfoW(SPI_GETWORKAREA, 0, &work, 0);
  int width = GetSystemMetrics(SM_CXSCREEN);
  int height = GetSystemMetrics(SM_CYSCREEN);
  int top = max(work.bottom, height - 120);
  int band = height - top;
  if (band <= 0) return 0;
  Canvas canvas = {0};
  int count = 0;
  if (canvas_open(&canvas, width, band) && BitBlt(canvas.memory, 0, 0, width, band, canvas.screen, 0, top, SRCCOPY)) {
    for (int y = 0; y < band; y++) {
      for (int x = 0; x < width; x++) {
        BYTE *pixel = canvas_pixel(&canvas, x, y);
        BOOL matches = abs(pixel[2] - color[0]) <= 40 && abs(pixel[1] - color[1]) <= 40 && abs(pixel[0] - color[2]) <= 40;
        count += matches ? 1 : 0;
      }
    }
  }
  canvas_close(&canvas);
  return count;
}

static int check_file(const wchar_t *path) {
  HICON icon = NULL;
  if (ExtractIconExW(path, 0, &icon, NULL, 1) == 0 || icon == NULL) return fail("the file has no icon resource");
  int color[3] = {0};
  BOOL drawn = icon_center(icon, color);
  DestroyIcon(icon);
  if (!drawn) return fail("the file icon could not be drawn");
  printf("{\"file\":[%d,%d,%d]}\n", color[0], color[1], color[2]);
  return 0;
}

static int check_window(wchar_t **arguments) {
  WindowSearch search = {(DWORD)wcstoul(arguments[0], NULL, 10), NULL};
  EnumWindows(window_of_process, (LPARAM)&search);
  if (search.window == NULL) return fail("the process has no visible top-level window");
  HICON icon = (HICON)GetClassLongPtrW(search.window, GCLP_HICON);
  HICON small_icon = (HICON)GetClassLongPtrW(search.window, GCLP_HICONSM);
  icon = icon != NULL ? icon : (HICON)SendMessageW(search.window, WM_GETICON, ICON_BIG, 0);
  if (icon == NULL) return fail("the window has no icon");
  int color[3] = {0};
  if (!icon_center(icon, color)) return fail("the window icon could not be drawn");
  int expected[3] = {(int)wcstol(arguments[1], NULL, 10), (int)wcstol(arguments[2], NULL, 10), (int)wcstol(arguments[3], NULL, 10)};
  printf("{\"window\":[%d,%d,%d],\"small\":%s,\"taskbar\":%d}\n", color[0], color[1], color[2], small_icon != NULL ? "true" : "false", taskbar_pixels(expected));
  return 0;
}

int wmain(int argc, wchar_t **argv) {
  SetProcessDPIAware();
  if (argc == 3 && wcscmp(argv[1], L"file") == 0) return check_file(argv[2]);
  if (argc == 6 && wcscmp(argv[1], L"window") == 0) return check_window(argv + 2);
  return fail("usage: WindowsIconCheck file <exe> | WindowsIconCheck window <process id> <red> <green> <blue>");
}
