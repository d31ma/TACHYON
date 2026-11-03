#define TACHYON_WM_RESUME (WM_APP + 2)

typedef void (*TachyonTaskDispatch)(void);
typedef void (*TachyonCompanionSetDispatch)(TachyonTaskDispatch);

static LONG g_resume_posted = 0;
static BOOL g_task_bound[3] = {FALSE, FALSE, FALSE};

static void tachyon_task_dispatch(void) {
  HWND window = (HWND)InterlockedCompareExchangePointer(&g_publish_window, NULL, NULL);
  if (window == NULL || InterlockedExchange(&g_resume_posted, 1) == 1) return;
  if (!PostMessageW(window, TACHYON_WM_RESUME, 0, 0)) InterlockedExchange(&g_resume_posted, 0);
}

static void tachyon_task_bind(HMODULE library, int language) {
  TachyonCompanionSetDispatch set_dispatch =
      (TachyonCompanionSetDispatch)(void (*)(void))GetProcAddress(library, "tac_native_set_dispatch");
  if (set_dispatch == NULL) return;
  g_task_bound[language] = TRUE;
  set_dispatch(tachyon_task_dispatch);
}

static void tachyon_task_resume(void) {
  InterlockedExchange(&g_resume_posted, 0);
  for (int language = 1; language <= 2; language++) {
    if (!g_task_bound[language] || g_companion_invoke[language] == NULL) continue;
    const char *answer = g_companion_invoke[language]("{\"op\":\"host-resume\"}");
    if (answer != NULL && g_companion_free[language] != NULL) g_companion_free[language](answer);
  }
}
