#define TACHYON_WM_PAGE_MESSAGE (WM_APP + 3)
#define TACHYON_PAGE_MESSAGE_LIMIT 256

static wchar_t *g_page_messages[TACHYON_PAGE_MESSAGE_LIMIT];
static int g_page_message_first = 0;
static int g_page_message_count = 0;
static BOOL g_page_messages_running = FALSE;
static BOOL g_page_resume_held = FALSE;

static BOOL tachyon_page_busy(void) { return g_page_messages_running || tachyon_file_picker_busy(); }

static void tachyon_page_message_hold(const wchar_t *message) {
  if (g_window == NULL || g_page_message_count == TACHYON_PAGE_MESSAGE_LIMIT) return;
  wchar_t *copy = _wcsdup(message);
  if (copy == NULL) return;
  g_page_messages[(g_page_message_first + g_page_message_count) % TACHYON_PAGE_MESSAGE_LIMIT] = copy;
  g_page_message_count++;
  PostMessageW(g_window, TACHYON_WM_PAGE_MESSAGE, 0, 0);
}

static void tachyon_page_messages_resume(void) {
  if (tachyon_page_busy() || g_window == NULL) return;
  if (g_page_message_count > 0) PostMessageW(g_window, TACHYON_WM_PAGE_MESSAGE, 0, 0);
  if (g_page_resume_held) { g_page_resume_held = FALSE; PostMessageW(g_window, TACHYON_WM_RESUME, 0, 0); }
}

static void tachyon_page_messages_run(void) {
  if (tachyon_page_busy()) return;
  g_page_messages_running = TRUE;
  while (g_page_message_count > 0) {
    wchar_t *message = g_page_messages[g_page_message_first];
    g_page_message_first = (g_page_message_first + 1) % TACHYON_PAGE_MESSAGE_LIMIT;
    g_page_message_count--;
    tachyon_on_message(message);
    free(message);
  }
  g_page_messages_running = FALSE;
  tachyon_page_messages_resume();
}

static BOOL tachyon_page_resume_waits(void) {
  BOOL busy = tachyon_page_busy();
  if (busy) g_page_resume_held = TRUE;
  return busy;
}

static void tachyon_page_messages_clear(void) {
  while (g_page_message_count > 0) {
    free(g_page_messages[g_page_message_first]);
    g_page_message_first = (g_page_message_first + 1) % TACHYON_PAGE_MESSAGE_LIMIT;
    g_page_message_count--;
  }
}
