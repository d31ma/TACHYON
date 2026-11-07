#include <winstring.h>

#define TACHYON_TOAST_MANAGER L"{50ac103f-d235-4598-bbef-98fe4d1a3ad4}"
#define TACHYON_TOAST_FACTORY L"{04124b20-82c6-4229-b109-fd9ed4662b53}"
#define TACHYON_XML_DOCUMENT L"{f7f3a506-1e87-42d6-bcfb-b8c809fa5494}"
#define TACHYON_XML_DOCUMENT_IO L"{6cd0e74e-ee65-4489-9ebf-ca43e87ba637}"
#define TACHYON_SHARE_INTEROP L"{3a3dcd6c-3eab-43dc-bcde-45671ce800c8}"
#define TACHYON_SHARE_MANAGER L"{a5caee9b-8708-49d1-8d36-67d25a8da00c}"
#define TACHYON_SHARE_REQUESTED L"{ec6f9cc8-46d0-5e0e-b4d2-7d7773ae37a0}"
#define TACHYON_DATA_PACKAGE_LINKS L"{041c1fe9-2409-45e1-a538-4c53eeee04a7}"
#define TACHYON_URI_FACTORY L"{44a9796f-723e-4fdf-a218-033e75b0c084}"
#define TACHYON_AGILE_OBJECT L"{94ea2b94-e9cc-49e0-c0ff-ee64ca8f5b90}"
#define TACHYON_UNKNOWN_OBJECT L"{00000000-0000-0000-c000-000000000046}"

typedef HRESULT (WINAPI *TachyonActivationFactory)(HSTRING, REFIID, void **);
typedef HRESULT (WINAPI *TachyonActivateInstance)(HSTRING, void **);
typedef HRESULT (WINAPI *TachyonCreateString)(const wchar_t *, UINT32, HSTRING *);
typedef HRESULT (WINAPI *TachyonDeleteString)(HSTRING);
typedef LONG (WINAPI *TachyonApplicationModelId)(UINT32 *, PWSTR);
typedef BOOL (*TachyonShareShow)(HWND);
typedef BOOL (*TachyonToastReady)(void);
typedef BOOL (*TachyonToastShow)(const wchar_t *);

typedef struct {
  HRESULT (STDMETHODCALLTYPE *QueryInterface)(void *, REFIID, void **);
  ULONG (STDMETHODCALLTYPE *AddRef)(void *);
  ULONG (STDMETHODCALLTYPE *Release)(void *);
} TachyonUnknownMethods;
typedef struct { const TachyonUnknownMethods *lpVtbl; } TachyonRuntimeObject;

typedef struct {
  TachyonUnknownMethods unknown;
  void *inspectable[3];
  HRESULT (STDMETHODCALLTYPE *CreateToastNotifier)(void *, void **);
  HRESULT (STDMETHODCALLTYPE *CreateToastNotifierWithId)(void *, HSTRING, void **);
} TachyonToastManagerMethods;
typedef struct { const TachyonToastManagerMethods *lpVtbl; } TachyonToastManager;

typedef struct {
  TachyonUnknownMethods unknown;
  void *inspectable[3];
  HRESULT (STDMETHODCALLTYPE *CreateToastNotification)(void *, void *, void **);
} TachyonToastFactoryMethods;
typedef struct { const TachyonToastFactoryMethods *lpVtbl; } TachyonToastFactory;

typedef struct {
  TachyonUnknownMethods unknown;
  void *inspectable[3];
  HRESULT (STDMETHODCALLTYPE *Show)(void *, void *);
  HRESULT (STDMETHODCALLTYPE *Hide)(void *, void *);
  HRESULT (STDMETHODCALLTYPE *get_Setting)(void *, int *);
} TachyonToastNotifierMethods;
typedef struct { const TachyonToastNotifierMethods *lpVtbl; } TachyonToastNotifier;

typedef struct {
  TachyonUnknownMethods unknown;
  void *inspectable[3];
  HRESULT (STDMETHODCALLTYPE *LoadXml)(void *, HSTRING);
} TachyonXmlDocumentInputMethods;
typedef struct { const TachyonXmlDocumentInputMethods *lpVtbl; } TachyonXmlDocumentInput;

typedef struct {
  TachyonUnknownMethods unknown;
  HRESULT (STDMETHODCALLTYPE *GetForWindow)(void *, HWND, REFIID, void **);
  HRESULT (STDMETHODCALLTYPE *ShowShareUIForWindow)(void *, HWND);
} TachyonShareInteropMethods;
typedef struct { const TachyonShareInteropMethods *lpVtbl; } TachyonShareInterop;

typedef struct {
  TachyonUnknownMethods unknown;
  void *inspectable[3];
  HRESULT (STDMETHODCALLTYPE *add_DataRequested)(void *, void *, INT64 *);
} TachyonShareManagerMethods;
typedef struct { const TachyonShareManagerMethods *lpVtbl; } TachyonShareManager;

typedef struct {
  TachyonUnknownMethods unknown;
  void *inspectable[3];
  HRESULT (STDMETHODCALLTYPE *get_Request)(void *, void **);
} TachyonShareArgumentsMethods;
typedef struct { const TachyonShareArgumentsMethods *lpVtbl; } TachyonShareArguments;

typedef struct {
  TachyonUnknownMethods unknown;
  void *inspectable[3];
  HRESULT (STDMETHODCALLTYPE *get_Data)(void *, void **);
} TachyonShareRequestMethods;
typedef struct { const TachyonShareRequestMethods *lpVtbl; } TachyonShareRequest;

typedef struct {
  TachyonUnknownMethods unknown;
  void *inspectable[3];
  void *view;
  HRESULT (STDMETHODCALLTYPE *get_Properties)(void *, void **);
  void *operations_and_providers[8];
  HRESULT (STDMETHODCALLTYPE *SetText)(void *, HSTRING);
} TachyonDataPackageMethods;
typedef struct { const TachyonDataPackageMethods *lpVtbl; } TachyonDataPackage;

typedef struct {
  TachyonUnknownMethods unknown;
  void *inspectable[3];
  void *application_link;
  HRESULT (STDMETHODCALLTYPE *SetWebLink)(void *, void *);
} TachyonDataPackageLinksMethods;
typedef struct { const TachyonDataPackageLinksMethods *lpVtbl; } TachyonDataPackageLinks;

typedef struct {
  TachyonUnknownMethods unknown;
  void *inspectable[3];
  void *title;
  HRESULT (STDMETHODCALLTYPE *put_Title)(void *, HSTRING);
} TachyonDataPropertiesMethods;
typedef struct { const TachyonDataPropertiesMethods *lpVtbl; } TachyonDataProperties;

typedef struct {
  TachyonUnknownMethods unknown;
  void *inspectable[3];
  HRESULT (STDMETHODCALLTYPE *CreateUri)(void *, HSTRING, void **);
} TachyonUriFactoryMethods;
typedef struct { const TachyonUriFactoryMethods *lpVtbl; } TachyonUriFactory;

typedef struct {
  HRESULT (STDMETHODCALLTYPE *QueryInterface)(void *, REFIID, void **);
  ULONG (STDMETHODCALLTYPE *AddRef)(void *);
  ULONG (STDMETHODCALLTYPE *Release)(void *);
  HRESULT (STDMETHODCALLTYPE *Invoke)(void *, void *, void *);
} TachyonShareHandlerMethods;
typedef struct { const TachyonShareHandlerMethods *lpVtbl; } TachyonShareHandler;

typedef struct {
  HMODULE library;
  TachyonActivationFactory factory;
  TachyonActivateInstance activate;
  TachyonCreateString create;
  TachyonDeleteString remove;
} TachyonRuntime;

typedef struct { wchar_t *title; wchar_t *text; wchar_t *url; } TachyonShareValues;

static TachyonRuntime g_runtime = {NULL, NULL, NULL, NULL, NULL};
static TachyonShareValues g_share = {NULL, NULL, NULL};
static TachyonShareManager *g_share_manager = NULL;

static BOOL tachyon_browser_api_granted(const char *api) {
  (void)api;
  __GRANTED_BROWSER_APIS__
  return FALSE;
}

static COREWEBVIEW2_PERMISSION_STATE tachyon_permission_state(COREWEBVIEW2_PERMISSION_KIND kind, BOOL own_page) {
  const char *api = kind == COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION ? "geolocation"
    : kind == COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ ? "clipboard" : NULL;
  return api != NULL && own_page && tachyon_browser_api_granted(api) ? COREWEBVIEW2_PERMISSION_STATE_ALLOW : COREWEBVIEW2_PERMISSION_STATE_DENY;
}

static BOOL tachyon_runtime_ready(void) {
  if (g_runtime.library != NULL) return g_runtime.factory != NULL && g_runtime.activate != NULL && g_runtime.create != NULL && g_runtime.remove != NULL;
  g_runtime.library = LoadLibraryExW(L"combase.dll", NULL, LOAD_LIBRARY_SEARCH_SYSTEM32);
  if (g_runtime.library == NULL) return FALSE;
  g_runtime.factory = (TachyonActivationFactory)(void (*)(void))GetProcAddress(g_runtime.library, "RoGetActivationFactory");
  g_runtime.activate = (TachyonActivateInstance)(void (*)(void))GetProcAddress(g_runtime.library, "RoActivateInstance");
  g_runtime.create = (TachyonCreateString)(void (*)(void))GetProcAddress(g_runtime.library, "WindowsCreateString");
  g_runtime.remove = (TachyonDeleteString)(void (*)(void))GetProcAddress(g_runtime.library, "WindowsDeleteString");
  return g_runtime.factory != NULL && g_runtime.activate != NULL && g_runtime.create != NULL && g_runtime.remove != NULL;
}

static HSTRING tachyon_runtime_text(const wchar_t *text) {
  HSTRING value = NULL;
  if (text == NULL || text[0] == L'\0' || FAILED(g_runtime.create(text, (UINT32)wcslen(text), &value))) return NULL;
  return value;
}

static void tachyon_runtime_free_text(HSTRING value) {
  if (value != NULL) g_runtime.remove(value);
}

static void tachyon_runtime_release(void *object) {
  if (object != NULL) ((TachyonRuntimeObject *)object)->lpVtbl->Release(object);
}

static HRESULT tachyon_runtime_query(void *object, const wchar_t *interface_id, void **found) {
  IID identifier;
  *found = NULL;
  HRESULT result = IIDFromString(interface_id, &identifier);
  if (SUCCEEDED(result)) result = ((TachyonRuntimeObject *)object)->lpVtbl->QueryInterface(object, &identifier, found);
  return result;
}

static HRESULT tachyon_runtime_factory(const wchar_t *class_name, const wchar_t *interface_id, void **factory) {
  IID identifier;
  *factory = NULL;
  HSTRING name = tachyon_runtime_text(class_name);
  HRESULT result = name == NULL ? E_OUTOFMEMORY : IIDFromString(interface_id, &identifier);
  if (SUCCEEDED(result)) result = g_runtime.factory(name, &identifier, factory);
  tachyon_runtime_free_text(name);
  return result;
}

static HRESULT tachyon_runtime_set_text(HRESULT (STDMETHODCALLTYPE *set)(void *, HSTRING), void *object, const wchar_t *text) {
  HSTRING value = tachyon_runtime_text(text);
  HRESULT result = value == NULL ? E_INVALIDARG : set(object, value);
  tachyon_runtime_free_text(value);
  return result;
}

static wchar_t *tachyon_payload_wide(const char *payload, const char *name) {
  size_t capacity = strlen(payload) + 1;
  uint16_t *value = (uint16_t *)calloc(capacity, sizeof(uint16_t));
  if (value != NULL && !tachyon_window_text(payload, name, value, capacity)) { free(value); value = NULL; }
  return (wchar_t *)value;
}

static wchar_t *tachyon_payload_given(const char *payload, const char *name) {
  wchar_t *value = tachyon_payload_wide(payload, name);
  if (value != NULL && value[0] == L'\0') { free(value); value = NULL; }
  return value;
}

static BOOL tachyon_clipboard_open(void) {
  for (int attempt = 0; attempt < 5; attempt++) {
    if (OpenClipboard(g_window)) return TRUE;
    Sleep(20);
  }
  return FALSE;
}

static char *tachyon_clipboard_write(const char *payload) {
  if (!tachyon_browser_api_granted("clipboard")) return _strdup("{\"error\":\"tac.browserApis does not grant clipboard.\"}");
  wchar_t *text = tachyon_payload_wide(payload, "text");
  if (text == NULL) return _strdup("{\"error\":\"Clipboard text must be a string.\"}");
  size_t bytes = (wcslen(text) + 1) * sizeof(wchar_t);
  HGLOBAL memory = GlobalAlloc(GMEM_MOVEABLE, bytes);
  void *target = memory != NULL ? GlobalLock(memory) : NULL;
  if (target != NULL) { memcpy(target, text, bytes); GlobalUnlock(memory); }
  free(text);
  BOOL copied = FALSE;
  if (target != NULL && tachyon_clipboard_open()) {
    copied = EmptyClipboard() && SetClipboardData(CF_UNICODETEXT, memory) != NULL;
    CloseClipboard();
  }
  if (!copied && memory != NULL) GlobalFree(memory);
  return _strdup(copied ? "{\"value\":null}" : "{\"error\":\"The clipboard is in use by another app.\"}");
}

static char *tachyon_json_text_answer(const wchar_t *text) {
  char *narrow = tachyon_narrow(text);
  if (narrow == NULL) return NULL;
  size_t length = strlen(narrow);
  char *answer = length <= TACHYON_MESSAGE_LIMIT ? (char *)calloc(length * 6 + 16, 1) : NULL;
  if (answer == NULL) { free(narrow); return NULL; }
  size_t used = (size_t)snprintf(answer, 16, "{\"value\":\"");
  for (size_t index = 0; index < length; index++) {
    unsigned char ch = (unsigned char)narrow[index];
    if (ch == '"' || ch == '\\') { answer[used++] = '\\'; answer[used++] = (char)ch; }
    else if (ch < 0x20) used += (size_t)snprintf(answer + used, 7, "\\u%04x", ch);
    else answer[used++] = (char)ch;
  }
  memcpy(answer + used, "\"}", 3);
  free(narrow);
  return answer;
}

static char *tachyon_clipboard_read(void) {
  if (!tachyon_browser_api_granted("clipboard")) return _strdup("{\"error\":\"tac.browserApis does not grant clipboard.\"}");
  if (!tachyon_clipboard_open()) return _strdup("{\"error\":\"The clipboard is in use by another app.\"}");
  HANDLE data = IsClipboardFormatAvailable(CF_UNICODETEXT) ? GetClipboardData(CF_UNICODETEXT) : NULL;
  const wchar_t *text = data != NULL ? (const wchar_t *)GlobalLock(data) : NULL;
  char *answer = tachyon_json_text_answer(text != NULL ? text : L"");
  if (text != NULL) GlobalUnlock(data);
  CloseClipboard();
  return answer != NULL ? answer : _strdup("{\"error\":\"The clipboard text is too large for the app.\"}");
}

static BOOL tachyon_share_link(TachyonDataPackage *package, const wchar_t *url) {
  TachyonDataPackageLinks *links = NULL;
  TachyonUriFactory *factory = NULL;
  void *uri = NULL;
  HSTRING address = tachyon_runtime_text(url);
  HRESULT result = address == NULL ? E_INVALIDARG : tachyon_runtime_query(package, TACHYON_DATA_PACKAGE_LINKS, (void **)&links);
  if (SUCCEEDED(result)) result = tachyon_runtime_factory(L"Windows.Foundation.Uri", TACHYON_URI_FACTORY, (void **)&factory);
  if (SUCCEEDED(result)) result = factory->lpVtbl->CreateUri(factory, address, &uri);
  if (SUCCEEDED(result)) result = links->lpVtbl->SetWebLink(links, uri);
  tachyon_runtime_release(uri);
  tachyon_runtime_release(factory);
  tachyon_runtime_release(links);
  tachyon_runtime_free_text(address);
  return SUCCEEDED(result);
}

static void tachyon_share_fill(TachyonDataPackage *package) {
  TachyonDataProperties *properties = NULL;
  if (SUCCEEDED(package->lpVtbl->get_Properties(package, (void **)&properties))) {
    tachyon_runtime_set_text(properties->lpVtbl->put_Title, properties, g_share.title);
    tachyon_runtime_release(properties);
  }
  if (g_share.text != NULL) tachyon_runtime_set_text(package->lpVtbl->SetText, package, g_share.text);
  BOOL linked = g_share.url != NULL && tachyon_share_link(package, g_share.url);
  if (!linked && g_share.text == NULL && g_share.url != NULL) tachyon_runtime_set_text(package->lpVtbl->SetText, package, g_share.url);
}

static HRESULT STDMETHODCALLTYPE share_requested_query(void *self, REFIID iid, void **object) {
  wchar_t identifier[40];
  if (object == NULL || iid == NULL) return E_POINTER;
  *object = NULL;
  if (StringFromGUID2(iid, identifier, 40) == 0) return E_NOINTERFACE;
  if (_wcsicmp(identifier, TACHYON_SHARE_REQUESTED) != 0 && _wcsicmp(identifier, TACHYON_AGILE_OBJECT) != 0 &&
      _wcsicmp(identifier, TACHYON_UNKNOWN_OBJECT) != 0) return E_NOINTERFACE;
  *object = self;
  return S_OK;
}
static ULONG STDMETHODCALLTYPE share_requested_add_ref(void *self) { (void)self; return 1; }
static ULONG STDMETHODCALLTYPE share_requested_release(void *self) { (void)self; return 1; }
static HRESULT STDMETHODCALLTYPE share_requested_invoke(void *self, void *sender, void *raw_arguments) {
  (void)self; (void)sender;
  TachyonShareArguments *arguments = (TachyonShareArguments *)raw_arguments;
  TachyonShareRequest *request = NULL;
  TachyonDataPackage *package = NULL;
  if (arguments == NULL || g_share.title == NULL) return S_OK;
  HRESULT result = arguments->lpVtbl->get_Request(arguments, (void **)&request);
  if (SUCCEEDED(result)) result = request->lpVtbl->get_Data(request, (void **)&package);
  if (SUCCEEDED(result)) tachyon_share_fill(package);
  tachyon_runtime_release(package);
  tachyon_runtime_release(request);
  return S_OK;
}
static const TachyonShareHandlerMethods g_share_handler_methods = {share_requested_query, share_requested_add_ref, share_requested_release, share_requested_invoke};
static TachyonShareHandler g_share_handler = {&g_share_handler_methods};

static BOOL tachyon_share_sheet(HWND window) {
  TachyonShareInterop *interop = NULL;
  IID manager_id;
  HRESULT result = window != NULL && tachyon_runtime_ready()
    ? tachyon_runtime_factory(L"Windows.ApplicationModel.DataTransfer.DataTransferManager", TACHYON_SHARE_INTEROP, (void **)&interop) : E_FAIL;
  if (SUCCEEDED(result) && g_share_manager == NULL) {
    TachyonShareManager *manager = NULL;
    INT64 token = 0;
    result = IIDFromString(TACHYON_SHARE_MANAGER, &manager_id);
    if (SUCCEEDED(result)) result = interop->lpVtbl->GetForWindow(interop, window, &manager_id, (void **)&manager);
    if (SUCCEEDED(result)) result = manager->lpVtbl->add_DataRequested(manager, &g_share_handler, &token);
    if (SUCCEEDED(result)) g_share_manager = manager;
    else tachyon_runtime_release(manager);
  }
  if (SUCCEEDED(result)) result = interop->lpVtbl->ShowShareUIForWindow(interop, window);
  tachyon_runtime_release(interop);
  return SUCCEEDED(result);
}

static TachyonShareShow g_share_show = tachyon_share_sheet;

static char *tachyon_share_open(const char *payload) {
  if (!tachyon_browser_api_granted("share")) return _strdup("{\"error\":\"tac.browserApis does not grant share.\"}");
  wchar_t *title = tachyon_payload_given(payload, "title");
  wchar_t *text = tachyon_payload_given(payload, "text");
  wchar_t *url = tachyon_payload_given(payload, "url");
  if (title == NULL && text == NULL && url == NULL) return _strdup("{\"error\":\"Share needs a title, text or url.\"}");
  if (title == NULL) title = tachyon_widen(TACHYON_APP_NAME);
  free(g_share.title); free(g_share.text); free(g_share.url);
  g_share.title = title; g_share.text = text; g_share.url = url;
  return _strdup(g_share_show(g_window) ? "{\"value\":null}" : "{\"error\":\"The Windows share sheet could not open.\"}");
}

static wchar_t *tachyon_toast_identity(void) {
  HMODULE kernel = GetModuleHandleW(L"kernel32.dll");
  TachyonApplicationModelId model_id = kernel == NULL ? NULL
    : (TachyonApplicationModelId)(void (*)(void))GetProcAddress(kernel, "GetCurrentApplicationUserModelId");
  wchar_t identity[256];
  UINT32 length = 256;
  if (model_id == NULL || model_id(&length, identity) != ERROR_SUCCESS) return NULL;
  return _wcsdup(identity);
}

static TachyonToastNotifier *tachyon_toast_notifier(void) {
  wchar_t *identity = tachyon_toast_identity();
  TachyonToastManager *manager = NULL;
  TachyonToastNotifier *notifier = NULL;
  HRESULT result = identity != NULL && tachyon_runtime_ready()
    ? tachyon_runtime_factory(L"Windows.UI.Notifications.ToastNotificationManager", TACHYON_TOAST_MANAGER, (void **)&manager) : E_FAIL;
  HSTRING id = SUCCEEDED(result) ? tachyon_runtime_text(identity) : NULL;
  if (id != NULL) manager->lpVtbl->CreateToastNotifierWithId(manager, id, (void **)&notifier);
  tachyon_runtime_free_text(id);
  tachyon_runtime_release(manager);
  free(identity);
  return notifier;
}

static BOOL tachyon_toast_enabled(void) {
  TachyonToastNotifier *notifier = tachyon_toast_notifier();
  int setting = 1;
  if (notifier != NULL && FAILED(notifier->lpVtbl->get_Setting(notifier, &setting))) setting = 1;
  tachyon_runtime_release(notifier);
  return setting == 0;
}

static void *tachyon_toast_document(const wchar_t *xml) {
  void *instance = NULL, *document = NULL;
  TachyonXmlDocumentInput *input = NULL;
  HSTRING name = tachyon_runtime_text(L"Windows.Data.Xml.Dom.XmlDocument");
  HSTRING content = tachyon_runtime_text(xml);
  HRESULT result = name == NULL || content == NULL ? E_OUTOFMEMORY : g_runtime.activate(name, &instance);
  if (SUCCEEDED(result)) result = tachyon_runtime_query(instance, TACHYON_XML_DOCUMENT_IO, (void **)&input);
  if (SUCCEEDED(result)) result = input->lpVtbl->LoadXml(input, content);
  if (SUCCEEDED(result)) tachyon_runtime_query(instance, TACHYON_XML_DOCUMENT, &document);
  tachyon_runtime_release(input);
  tachyon_runtime_release(instance);
  tachyon_runtime_free_text(content);
  tachyon_runtime_free_text(name);
  return document;
}

static BOOL tachyon_toast_send(const wchar_t *xml) {
  TachyonToastNotifier *notifier = tachyon_toast_notifier();
  void *document = notifier != NULL ? tachyon_toast_document(xml) : NULL;
  TachyonToastFactory *factory = NULL;
  void *toast = NULL;
  HRESULT result = document == NULL ? E_FAIL
    : tachyon_runtime_factory(L"Windows.UI.Notifications.ToastNotification", TACHYON_TOAST_FACTORY, (void **)&factory);
  if (SUCCEEDED(result)) result = factory->lpVtbl->CreateToastNotification(factory, document, &toast);
  if (SUCCEEDED(result)) result = notifier->lpVtbl->Show(notifier, toast);
  tachyon_runtime_release(toast);
  tachyon_runtime_release(factory);
  tachyon_runtime_release(document);
  tachyon_runtime_release(notifier);
  return SUCCEEDED(result);
}

static TachyonToastReady g_toast_ready = tachyon_toast_enabled;
static TachyonToastShow g_toast_show = tachyon_toast_send;

static void tachyon_toast_append(wchar_t *xml, const wchar_t *text, BOOL escaped) {
  size_t used = wcslen(xml);
  for (; *text != L'\0'; text++) {
    const wchar_t *entity = !escaped ? NULL : *text == L'&' ? L"&amp;" : *text == L'<' ? L"&lt;" : *text == L'>' ? L"&gt;"
      : *text == L'"' ? L"&quot;" : *text == L'\'' ? L"&apos;" : NULL;
    size_t length = entity != NULL ? wcslen(entity) : 1;
    memcpy(xml + used, entity != NULL ? entity : text, length * sizeof(wchar_t));
    used += length;
  }
  xml[used] = L'\0';
}

static wchar_t *tachyon_toast_xml(const wchar_t *title, const wchar_t *body) {
  size_t capacity = (wcslen(title) + wcslen(body)) * 6 + 160;
  wchar_t *xml = (wchar_t *)calloc(capacity, sizeof(wchar_t));
  if (xml == NULL) return NULL;
  tachyon_toast_append(xml, L"<toast><visual><binding template=\"ToastGeneric\"><text>", FALSE);
  tachyon_toast_append(xml, title, TRUE);
  tachyon_toast_append(xml, L"</text><text>", FALSE);
  tachyon_toast_append(xml, body, TRUE);
  tachyon_toast_append(xml, L"</text></binding></visual></toast>", FALSE);
  return xml;
}

static char *tachyon_notification_permission(void) {
  return _strdup(tachyon_browser_api_granted("notifications") && g_toast_ready() ? "{\"value\":\"granted\"}" : "{\"value\":\"denied\"}");
}

static char *tachyon_notification_show(const char *payload) {
  if (!tachyon_browser_api_granted("notifications")) return _strdup("{\"error\":\"tac.browserApis does not grant notifications.\"}");
  wchar_t *title = tachyon_payload_given(payload, "title");
  wchar_t *body = tachyon_payload_wide(payload, "body");
  wchar_t *xml = title != NULL ? tachyon_toast_xml(title, body != NULL ? body : L"") : NULL;
  free(title); free(body);
  if (xml == NULL) return _strdup("{\"error\":\"A notification needs a title.\"}");
  const char *answer = !g_toast_ready()
    ? "{\"error\":\"Windows shows notifications only for the installed MSIX package, with notifications turned on for the app.\"}"
    : g_toast_show(xml) ? "{\"value\":null}" : "{\"error\":\"Windows could not show the notification.\"}";
  free(xml);
  return _strdup(answer);
}

static char *tachyon_browser_api(const char *capability, const char *payload) {
  if (strcmp(capability, "browserApi.clipboard.writeText") == 0) return tachyon_clipboard_write(payload);
  if (strcmp(capability, "browserApi.clipboard.readText") == 0) return tachyon_clipboard_read();
  if (strcmp(capability, "browserApi.share.open") == 0) return tachyon_share_open(payload);
  if (strcmp(capability, "browserApi.notifications.requestPermission") == 0) return tachyon_notification_permission();
  if (strcmp(capability, "browserApi.notifications.show") == 0) return tachyon_notification_show(payload);
  return _strdup("{\"error\":\"The Windows app has no bridge for this browser API.\"}");
}
