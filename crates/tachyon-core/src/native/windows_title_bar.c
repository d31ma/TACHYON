static const BOOL TACHYON_TITLE_BAR_INTEGRATED = __TITLE_BAR_INTEGRATED__;
static const char *TACHYON_TITLE_BAR_PAGE_SCRIPT = "__TITLE_BAR_PAGE_SCRIPT__";
static const wchar_t *TACHYON_TITLE_BAR_HOST_CLASS = L"TachyonTitleBarWebHost";

static HWND g_title_bar_host = NULL;
static RECT g_title_bar_shape = {-1, -1, -1, -1};
static char g_title_bar_insets[96] = "{\"height\":0,\"leading\":0,\"trailing\":0}";

static BOOL tachyon_title_bar_captioned(HWND window) {
  return TACHYON_TITLE_BAR_INTEGRATED && (GetWindowLongPtrW(window, GWL_STYLE) & WS_CAPTION) == WS_CAPTION;
}

static UINT tachyon_title_bar_dpi(HWND window) {
  UINT dpi = GetDpiForWindow(window);
  return dpi == 0 ? USER_DEFAULT_SCREEN_DPI : dpi;
}

static int tachyon_title_bar_border(HWND window) {
  UINT dpi = tachyon_title_bar_dpi(window);
  return GetSystemMetricsForDpi(SM_CYFRAME, dpi) + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
}

static int tachyon_title_bar_height(HWND window) {
  return GetSystemMetricsForDpi(SM_CYCAPTION, tachyon_title_bar_dpi(window)) + tachyon_title_bar_border(window);
}

static void tachyon_title_bar_extend(HWND window) {
  if (!TACHYON_TITLE_BAR_INTEGRATED) return;
  MARGINS margins = {0, 0, tachyon_title_bar_captioned(window) ? tachyon_title_bar_height(window) : 0, 0};
  DwmExtendFrameIntoClientArea(window, &margins);
}

static BOOL tachyon_title_bar_calculate(HWND window, WPARAM wparam, LPARAM lparam) {
  if (!wparam || !tachyon_title_bar_captioned(window)) return FALSE;
  NCCALCSIZE_PARAMS *sizes = (NCCALCSIZE_PARAMS *)lparam;
  LONG top = sizes->rgrc[0].top;
  DefWindowProcW(window, WM_NCCALCSIZE, wparam, lparam);
  sizes->rgrc[0].top = top + (IsZoomed(window) ? tachyon_title_bar_border(window) : 0);
  return TRUE;
}

static BOOL tachyon_title_bar_buttons(HWND window, RECT *buttons) {
  RECT client, frame;
  if (!tachyon_title_bar_captioned(window) || !GetClientRect(window, &client) || !GetWindowRect(window, &frame)) return FALSE;
  POINT origin = {0, 0};
  ClientToScreen(window, &origin);
  if (SUCCEEDED(DwmGetWindowAttribute(window, DWMWA_CAPTION_BUTTON_BOUNDS, buttons, sizeof(*buttons))) && !IsRectEmpty(buttons)) {
    OffsetRect(buttons, frame.left - origin.x, frame.top - origin.y);
    return TRUE;
  }
  int width = 3 * GetSystemMetricsForDpi(SM_CXSIZE, tachyon_title_bar_dpi(window));
  SetRect(buttons, client.right - width, 0, client.right, tachyon_title_bar_height(window) - (origin.y - frame.top));
  return TRUE;
}

static LRESULT tachyon_title_bar_hit(HWND window, WPARAM wparam, LPARAM lparam, BOOL *handled) {
  LRESULT hit = HTNOWHERE;
  *handled = tachyon_title_bar_captioned(window);
  if (!*handled || DwmDefWindowProc(window, WM_NCHITTEST, wparam, lparam, &hit)) return hit;
  hit = DefWindowProcW(window, WM_NCHITTEST, wparam, lparam);
  POINT point = {(short)LOWORD(lparam), (short)HIWORD(lparam)};
  ScreenToClient(window, &point);
  if (hit != HTCLIENT || IsZoomed(window) || point.y >= tachyon_title_bar_border(window)) return hit;
  RECT client;
  GetClientRect(window, &client);
  if (point.x < tachyon_title_bar_border(window)) return HTTOPLEFT;
  if (point.x >= client.right - tachyon_title_bar_border(window)) return HTTOPRIGHT;
  return HTTOP;
}

static int tachyon_title_bar_points(HWND window, LONG pixels) {
  return pixels <= 0 ? 0 : MulDiv((int)pixels, USER_DEFAULT_SCREEN_DPI, (int)tachyon_title_bar_dpi(window));
}

static void tachyon_title_bar_publish(HWND window) {
  RECT client, buttons;
  if (!TACHYON_TITLE_BAR_INTEGRATED || IsIconic(window) || !GetClientRect(window, &client)) return;
  int height = 0, trailing = 0;
  if (tachyon_title_bar_buttons(window, &buttons)) {
    height = tachyon_title_bar_points(window, buttons.bottom);
    trailing = tachyon_title_bar_points(window, client.right - buttons.left);
  }
  char insets[sizeof(g_title_bar_insets)];
  snprintf(insets, sizeof(insets), "{\"height\":%d,\"leading\":0,\"trailing\":%d}", height, trailing);
  if (strcmp(insets, g_title_bar_insets) == 0) return;
  strcpy_s(g_title_bar_insets, sizeof(g_title_bar_insets), insets);
  if (g_webview == NULL || tachyon_current_route() == NULL) return;
  wchar_t script[160];
  if (_snwprintf_s(script, 160, _TRUNCATE, L"globalThis.__tachyonTitleBar?.apply(%hs)", insets) >= 0) ICoreWebView2_ExecuteScript(g_webview, script, NULL);
}

static char *tachyon_title_bar_insets(void) {
  if (!TACHYON_TITLE_BAR_INTEGRATED) return _strdup("{\"error\":\"Native title bar unavailable.\"}");
  return _strdup(g_title_bar_insets);
}

static void tachyon_title_bar_layout(HWND window) {
  RECT client, buttons = {0, 0, 0, 0};
  if (g_title_bar_host == NULL || !GetClientRect(window, &client)) return;
  tachyon_title_bar_extend(window);
  if (!tachyon_title_bar_buttons(window, &buttons)) SetRectEmpty(&buttons);
  RECT shape = {client.right, client.bottom, buttons.left, buttons.bottom};
  if (!EqualRect(&shape, &g_title_bar_shape)) {
    g_title_bar_shape = shape;
    SetWindowPos(g_title_bar_host, NULL, 0, 0, client.right, client.bottom, SWP_NOZORDER | SWP_NOACTIVATE);
    HRGN region = CreateRectRgn(0, 0, client.right, client.bottom), hole = CreateRectRgnIndirect(&buttons);
    if (region != NULL && hole != NULL) CombineRgn(region, region, hole, RGN_DIFF);
    if (hole != NULL) DeleteObject(hole);
    if (region != NULL && !SetWindowRgn(g_title_bar_host, region, TRUE)) DeleteObject(region);
  }
  tachyon_title_bar_publish(window);
}

static BOOL tachyon_title_bar_paint(HWND window) {
  if (!TACHYON_TITLE_BAR_INTEGRATED) return FALSE;
  PAINTSTRUCT paint;
  HDC context = BeginPaint(window, &paint);
  if (context != NULL) FillRect(context, &paint.rcPaint, (HBRUSH)GetStockObject(BLACK_BRUSH));
  EndPaint(window, &paint);
  return TRUE;
}

static void tachyon_title_bar_menus(HWND window) {
  HMENU bar = GetMenu(window), system = GetSystemMenu(window, FALSE);
  if (bar == NULL || system == NULL) return;
  AppendMenuW(system, MF_SEPARATOR, 0, NULL);
  wchar_t title[64];
  while (GetMenuItemCount(bar) > 0) {
    HMENU popup = GetSubMenu(bar, 0);
    if (popup == NULL || GetMenuStringW(bar, 0, title, 64, MF_BYPOSITION) <= 0 || !RemoveMenu(bar, 0, MF_BYPOSITION)) break;
    AppendMenuW(system, MF_POPUP, (UINT_PTR)popup, title);
  }
  SetMenu(window, NULL);
  DestroyMenu(bar);
}

static BOOL tachyon_title_bar_install(HWND window, HINSTANCE instance) {
  if (!TACHYON_TITLE_BAR_INTEGRATED) return TRUE;
  tachyon_title_bar_menus(window);
  WNDCLASSEXW host_class = {0};
  host_class.cbSize = sizeof(host_class);
  host_class.lpfnWndProc = DefWindowProcW;
  host_class.hInstance = instance;
  host_class.lpszClassName = TACHYON_TITLE_BAR_HOST_CLASS;
  host_class.hCursor = LoadCursorW(NULL, (LPCWSTR)IDC_ARROW);
  RegisterClassExW(&host_class);
  g_title_bar_host = CreateWindowExW(0, TACHYON_TITLE_BAR_HOST_CLASS, NULL, WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
                                     0, 0, 0, 0, window, NULL, instance, NULL);
  if (g_title_bar_host == NULL) return FALSE;
  SetWindowPos(window, NULL, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
  tachyon_title_bar_extend(window);
  tachyon_title_bar_layout(window);
  return TRUE;
}

static HWND tachyon_title_bar_web_parent(HWND window) {
  return g_title_bar_host != NULL ? g_title_bar_host : window;
}

static void tachyon_title_bar_settings(ICoreWebView2 *view) {
  ICoreWebView2Settings *settings = NULL;
  ICoreWebView2Settings9 *regions = NULL;
  if (!TACHYON_TITLE_BAR_INTEGRATED || FAILED(ICoreWebView2_get_Settings(view, &settings)) || settings == NULL) return;
  if (SUCCEEDED(ICoreWebView2Settings_QueryInterface(settings, &IID_ICoreWebView2Settings9, (void **)&regions)) && regions != NULL) {
    ICoreWebView2Settings9_put_IsNonClientRegionSupportEnabled(regions, TRUE);
    ICoreWebView2Settings9_Release(regions);
  } else {
    tachyon_record("window.dragRegions.unavailable");
  }
  ICoreWebView2Settings_Release(settings);
}
