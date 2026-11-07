#define UNICODE
#define _UNICODE
#define WIN32_LEAN_AND_MEAN
#define COBJMACROS
#include <windows.h>
#include <objbase.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
STDAPI GetAvailableCoreWebView2BrowserVersionString(PCWSTR browserExecutableFolder, LPWSTR* versionInfo);
STDAPI CompareBrowserVersions(PCWSTR version1, PCWSTR version2, int* result);

#define TACHYON_APP_NAME "Windows Host Check"

static wchar_t *tachyon_widen(const char *value) {
  int length = MultiByteToWideChar(CP_UTF8, 0, value, -1, NULL, 0);
  wchar_t *wide = length > 0 ? malloc((size_t)length * sizeof(wchar_t)) : NULL;
  if (wide != NULL) MultiByteToWideChar(CP_UTF8, 0, value, -1, wide, length);
  return wide;
}

