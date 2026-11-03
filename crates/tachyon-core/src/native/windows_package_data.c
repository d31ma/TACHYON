typedef LONG (WINAPI *TachyonPackageFamilyName)(UINT32 *length, PWSTR name);

static wchar_t *tachyon_package_folder_for(const wchar_t *family) {
  wchar_t local[MAX_PATH];
  DWORD written = GetEnvironmentVariableW(L"LOCALAPPDATA", local, MAX_PATH);
  if (written == 0 || written >= MAX_PATH) return NULL;
  size_t size = MAX_PATH * 2;
  wchar_t *folder = calloc(size, sizeof(wchar_t));
  if (folder == NULL) return NULL;
  if (swprintf(folder, size, L"%ls\\Packages\\%ls\\LocalState\\WebView2", local, family) < 0) {
    free(folder);
    return NULL;
  }
  return folder;
}

static wchar_t *tachyon_package_data_folder(void) {
  HMODULE kernel = GetModuleHandleW(L"kernel32.dll");
  if (kernel == NULL) return NULL;
  TachyonPackageFamilyName family_name =
      (TachyonPackageFamilyName)(void (*)(void))GetProcAddress(kernel, "GetCurrentPackageFamilyName");
  if (family_name == NULL) return NULL;
  wchar_t family[256];
  UINT32 length = 256;
  if (family_name(&length, family) != ERROR_SUCCESS) return NULL;
  return tachyon_package_folder_for(family);
}
