#ifdef TACHYON_CHECK_RUNTIME_LAYOUT
#include <stddef.h>
#include <shobjidl_core.h>
#include <windows.data.xml.dom.h>
#include <windows.foundation.h>
#include <windows.ui.notifications.h>
#include <windows.applicationmodel.datatransfer.h>
#define TACHYON_SAME_SLOT(ours, theirs, method) _Static_assert(offsetof(ours, method) == offsetof(theirs, method), #ours " " #method)
TACHYON_SAME_SLOT(TachyonToastManagerMethods, __x_ABI_CWindows_CUI_CNotifications_CIToastNotificationManagerStaticsVtbl, CreateToastNotifierWithId);
TACHYON_SAME_SLOT(TachyonToastFactoryMethods, __x_ABI_CWindows_CUI_CNotifications_CIToastNotificationFactoryVtbl, CreateToastNotification);
TACHYON_SAME_SLOT(TachyonToastNotifierMethods, __x_ABI_CWindows_CUI_CNotifications_CIToastNotifierVtbl, Show);
TACHYON_SAME_SLOT(TachyonToastNotifierMethods, __x_ABI_CWindows_CUI_CNotifications_CIToastNotifierVtbl, get_Setting);
TACHYON_SAME_SLOT(TachyonXmlDocumentInputMethods, __x_ABI_CWindows_CData_CXml_CDom_CIXmlDocumentIOVtbl, LoadXml);
TACHYON_SAME_SLOT(TachyonShareInteropMethods, IDataTransferManagerInteropVtbl, GetForWindow);
TACHYON_SAME_SLOT(TachyonShareInteropMethods, IDataTransferManagerInteropVtbl, ShowShareUIForWindow);
TACHYON_SAME_SLOT(TachyonShareManagerMethods, __x_ABI_CWindows_CApplicationModel_CDataTransfer_CIDataTransferManagerVtbl, add_DataRequested);
TACHYON_SAME_SLOT(TachyonShareArgumentsMethods, __x_ABI_CWindows_CApplicationModel_CDataTransfer_CIDataRequestedEventArgsVtbl, get_Request);
TACHYON_SAME_SLOT(TachyonShareRequestMethods, __x_ABI_CWindows_CApplicationModel_CDataTransfer_CIDataRequestVtbl, get_Data);
TACHYON_SAME_SLOT(TachyonDataPackageMethods, __x_ABI_CWindows_CApplicationModel_CDataTransfer_CIDataPackageVtbl, get_Properties);
TACHYON_SAME_SLOT(TachyonDataPackageMethods, __x_ABI_CWindows_CApplicationModel_CDataTransfer_CIDataPackageVtbl, SetText);
TACHYON_SAME_SLOT(TachyonDataPackageLinksMethods, __x_ABI_CWindows_CApplicationModel_CDataTransfer_CIDataPackage2Vtbl, SetWebLink);
TACHYON_SAME_SLOT(TachyonDataPropertiesMethods, __x_ABI_CWindows_CApplicationModel_CDataTransfer_CIDataPackagePropertySetVtbl, put_Title);
TACHYON_SAME_SLOT(TachyonUriFactoryMethods, __x_ABI_CWindows_CFoundation_CIUriRuntimeClassFactoryVtbl, CreateUri);
#endif

static int g_failures = 0;
static int g_toast_answer = 0;
static int g_shares = 0;
static wchar_t g_shown_xml[512];

static BOOL check_toast_ready(void) { return g_toast_answer; }

static BOOL check_toast_show(const wchar_t *xml) {
  size_t length = wcslen(xml);
  if (length >= 512) return FALSE;
  memcpy(g_shown_xml, xml, (length + 1) * sizeof(wchar_t));
  return TRUE;
}

static BOOL check_share_show(HWND window) {
  (void)window;
  g_shares++;
  return TRUE;
}

static void check(const char *name, char *answer, const char *expected) {
  if (answer == NULL || strcmp(answer, expected) != 0) {
    fprintf(stderr, "%s answered %s\n", name, answer != NULL ? answer : "nothing");
    g_failures++;
  }
  free(answer);
}

static void check_text(const char *name, const wchar_t *value, const wchar_t *expected) {
  BOOL same = value == NULL || expected == NULL ? value == expected : wcscmp(value, expected) == 0;
  if (!same) {
    fprintf(stderr, "%s is %ls\n", name, value != NULL ? value : L"missing");
    g_failures++;
  }
}

static void check_state(const char *name, COREWEBVIEW2_PERMISSION_STATE state, COREWEBVIEW2_PERMISSION_STATE expected) {
  if (state != expected) {
    fprintf(stderr, "%s permission is %d\n", name, (int)state);
    g_failures++;
  }
}

static void check_put_clipboard(const wchar_t *text) {
  size_t bytes = (wcslen(text) + 1) * sizeof(wchar_t);
  HGLOBAL memory = GlobalAlloc(GMEM_MOVEABLE, bytes);
  void *target = memory != NULL ? GlobalLock(memory) : NULL;
  if (target != NULL) { memcpy(target, text, bytes); GlobalUnlock(memory); }
  if (target == NULL || !OpenClipboard(NULL)) { g_failures++; return; }
  EmptyClipboard();
  if (SetClipboardData(CF_UNICODETEXT, memory) == NULL) g_failures++;
  CloseClipboard();
}

static void check_granted(void) {
  check_put_clipboard(L"from another app");
  check("clipboard read of another app's text", tachyon_browser_api("browserApi.clipboard.readText", "{}"), "{\"value\":\"from another app\"}");
  check("clipboard write", tachyon_browser_api("browserApi.clipboard.writeText", "{\"text\":\"copied \\\"by\\\" the page\\n\\u00e9\"}"), "{\"value\":null}");
  check("clipboard read of the page's text", tachyon_browser_api("browserApi.clipboard.readText", "{}"), "{\"value\":\"copied \\\"by\\\" the page\\u000a\xc3\xa9\"}");
  check("clipboard write of a number", tachyon_browser_api("browserApi.clipboard.writeText", "{\"text\":5}"), "{\"error\":\"Clipboard text must be a string.\"}");
  check("clipboard write of nothing", tachyon_browser_api("browserApi.clipboard.writeText", "{\"text\":\"\"}"), "{\"value\":null}");
  check("clipboard read of nothing", tachyon_browser_api("browserApi.clipboard.readText", "{}"), "{\"value\":\"\"}");

  g_toast_answer = 0;
  check("unpackaged permission", tachyon_browser_api("browserApi.notifications.requestPermission", "{}"), "{\"value\":\"denied\"}");
  check("unpackaged notification", tachyon_browser_api("browserApi.notifications.show", "{\"title\":\"Order\",\"body\":\"Shipped\"}"),
        "{\"error\":\"Windows shows notifications only for the installed MSIX package, with notifications turned on for the app.\"}");
  g_toast_answer = 1;
  check("packaged permission", tachyon_browser_api("browserApi.notifications.requestPermission", "{}"), "{\"value\":\"granted\"}");
  check("notification", tachyon_browser_api("browserApi.notifications.show", "{\"title\":\"Order <1> & \\\"co\\\"\",\"body\":\"Shipped 'today'\"}"), "{\"value\":null}");
  check_text("toast", g_shown_xml, L"<toast><visual><binding template=\"ToastGeneric\"><text>Order &lt;1&gt; &amp; &quot;co&quot;</text><text>Shipped &apos;today&apos;</text></binding></visual></toast>");
  check("notification without a body", tachyon_browser_api("browserApi.notifications.show", "{\"title\":\"Hi\"}"), "{\"value\":null}");
  check_text("toast without a body", g_shown_xml, L"<toast><visual><binding template=\"ToastGeneric\"><text>Hi</text><text></text></binding></visual></toast>");
  check("notification without a title", tachyon_browser_api("browserApi.notifications.show", "{\"body\":\"No title\"}"), "{\"error\":\"A notification needs a title.\"}");

  check("share", tachyon_browser_api("browserApi.share.open", "{\"title\":\"Catalog\",\"text\":\"Look at this\",\"url\":\"https://example.com/item\"}"), "{\"value\":null}");
  check_text("shared title", g_share.title, L"Catalog");
  check_text("shared text", g_share.text, L"Look at this");
  check_text("shared url", g_share.url, L"https://example.com/item");
  check("share of a url", tachyon_browser_api("browserApi.share.open", "{\"title\":null,\"url\":\"https://example.com/x\"}"), "{\"value\":null}");
  check_text("default share title", g_share.title, L"Notes");
  check_text("no shared text", g_share.text, NULL);
  check("share of nothing", tachyon_browser_api("browserApi.share.open", "{\"title\":\"\",\"text\":null}"), "{\"error\":\"Share needs a title, text or url.\"}");
  if (g_shares != 2) { fprintf(stderr, "the share sheet opened %d times\n", g_shares); g_failures++; }

  check_state("own geolocation", tachyon_permission_state(COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION, TRUE), COREWEBVIEW2_PERMISSION_STATE_ALLOW);
  check_state("foreign geolocation", tachyon_permission_state(COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION, FALSE), COREWEBVIEW2_PERMISSION_STATE_DENY);
  check_state("own clipboard read", tachyon_permission_state(COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ, TRUE), COREWEBVIEW2_PERMISSION_STATE_ALLOW);
  check_state("web notifications", tachyon_permission_state(COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS, TRUE), COREWEBVIEW2_PERMISSION_STATE_DENY);
  check_state("camera", tachyon_permission_state(COREWEBVIEW2_PERMISSION_KIND_CAMERA, TRUE), COREWEBVIEW2_PERMISSION_STATE_DENY);
}

static void check_refused(void) {
  check_put_clipboard(L"from another app");
  check("refused clipboard read", tachyon_browser_api("browserApi.clipboard.readText", "{}"), "{\"error\":\"tac.browserApis does not grant clipboard.\"}");
  check("refused clipboard write", tachyon_browser_api("browserApi.clipboard.writeText", "{\"text\":\"x\"}"), "{\"error\":\"tac.browserApis does not grant clipboard.\"}");
  g_toast_answer = 1;
  check("refused permission", tachyon_browser_api("browserApi.notifications.requestPermission", "{}"), "{\"value\":\"denied\"}");
  check("refused notification", tachyon_browser_api("browserApi.notifications.show", "{\"title\":\"Order\"}"), "{\"error\":\"tac.browserApis does not grant notifications.\"}");
  check("refused share", tachyon_browser_api("browserApi.share.open", "{\"url\":\"https://example.com/\"}"), "{\"error\":\"tac.browserApis does not grant share.\"}");
  if (g_shares != 0) g_failures++;
  check_state("refused geolocation", tachyon_permission_state(COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION, TRUE), COREWEBVIEW2_PERMISSION_STATE_DENY);
  check_state("refused clipboard read", tachyon_permission_state(COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ, TRUE), COREWEBVIEW2_PERMISSION_STATE_DENY);
}

static void check_runtime(void) {
  if (tachyon_toast_identity() != NULL || tachyon_toast_enabled()) {
    fprintf(stderr, "an unpackaged program has a toast identity\n");
    g_failures++;
  }
  printf("runtime: %s\n", tachyon_runtime_ready() ? "loaded" : "missing");
#ifdef TACHYON_CHECK_RUNTIME_LAYOUT
  void *document = tachyon_toast_document(L"<toast><visual><binding template=\"ToastGeneric\"><text>A &amp; B</text></binding></visual></toast>");
  if (document == NULL) { fprintf(stderr, "Windows did not load the toast XML\n"); g_failures++; }
  tachyon_runtime_release(document);
  TachyonUriFactory *factory = NULL;
  void *uri = NULL;
  HSTRING address = tachyon_runtime_text(L"https://example.com/item");
  if (FAILED(tachyon_runtime_factory(L"Windows.Foundation.Uri", TACHYON_URI_FACTORY, (void **)&factory)) ||
      FAILED(factory->lpVtbl->CreateUri(factory, address, &uri))) { fprintf(stderr, "Windows did not make a Uri\n"); g_failures++; }
  tachyon_runtime_release(uri);
  tachyon_runtime_release(factory);
  tachyon_runtime_free_text(address);
#endif
  printf("share sheet: %s\n", tachyon_share_sheet(g_window) ? "opened" : "unavailable");
}

static LRESULT CALLBACK check_window(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
  return DefWindowProcW(window, message, wparam, lparam);
}

int main(void) {
  (void)tachyon_local_path;
  (void)tachyon_bundle_path;
  (void)tachyon_payload_route_matches;
  (void)tachyon_window_size;
  if (FAILED(CoInitializeEx(NULL, COINIT_APARTMENTTHREADED))) return 2;
  WNDCLASSW kind = {0};
  kind.lpfnWndProc = check_window;
  kind.lpszClassName = L"TachyonBrowserApiCheck";
  kind.hInstance = GetModuleHandleW(NULL);
  RegisterClassW(&kind);
  g_window = CreateWindowExW(0, kind.lpszClassName, L"", 0, 0, 0, 0, 0, HWND_MESSAGE, NULL, kind.hInstance, NULL);
  g_toast_ready = check_toast_ready;
  g_toast_show = check_toast_show;
  g_share_show = check_share_show;
  void (*checks[])(void) = {check_refused, check_granted};
#ifdef TACHYON_EXPECT_GRANTED
  checks[1]();
#else
  checks[0]();
#endif
  check_runtime();
  CoUninitialize();
  if (g_failures == 0) printf("windows browser APIs passed\n");
  return g_failures == 0 ? 0 : 1;
}
