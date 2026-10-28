#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <shobjidl_core.h>
#include <winrt/Windows.ApplicationModel.Core.h>
#include <winrt/Windows.Foundation.Collections.h>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Management.Deployment.h>
#include <chrono>
#include <cstdio>
#include <cwchar>
#include <string>
#include <vector>

using winrt::Windows::Foundation::AsyncStatus;
using winrt::Windows::Foundation::Uri;
using winrt::Windows::Management::Deployment::DeploymentOptions;
using winrt::Windows::Management::Deployment::PackageManager;

static int fail(const std::wstring &message) {
  fwprintf(stderr, L"WindowsPackageCheck: %ls\n", message.c_str());
  return 1;
}

template <typename Operation> static int finish(const Operation &operation, const wchar_t *step) {
  AsyncStatus status = operation.wait_for(std::chrono::minutes(5));
  if (status == AsyncStatus::Completed) return 0;
  if (status != AsyncStatus::Error) return fail(std::wstring(step) + L" did not finish in five minutes");
  return fail(std::wstring(step) + L": " + std::wstring(operation.GetResults().ErrorText()));
}

static int install(const wchar_t *package, const wchar_t *name) {
  PackageManager manager;
  if (finish(manager.AddPackageAsync(Uri(package), nullptr, DeploymentOptions::ForceApplicationShutdown), L"AddPackageAsync") != 0) return 1;
  for (const auto &installed : manager.FindPackagesForUser(L"")) {
    if (installed.Id().Name() != name) continue;
    auto entries = installed.GetAppListEntries();
    if (entries.Size() == 0) return fail(L"the installed package lists no app");
    wprintf(L"{\"package\":\"%ls\",\"app\":\"%ls\"}\n", installed.Id().FullName().c_str(), entries.GetAt(0).AppUserModelId().c_str());
    return 0;
  }
  return fail(L"the package installed, but Windows lists no package named " + std::wstring(name) + L" for this user");
}

static int uninstall(const wchar_t *package) {
  PackageManager manager;
  if (finish(manager.RemovePackageAsync(package), L"RemovePackageAsync") != 0) return 1;
  wprintf(L"{\"removed\":\"%ls\"}\n", package);
  return 0;
}

static int tile(const wchar_t *app, const wchar_t *edge_text) {
  int edge = (int)wcstol(edge_text, nullptr, 10);
  std::wstring item = std::wstring(L"shell:AppsFolder\\") + app;
  winrt::com_ptr<IShellItemImageFactory> factory;
  if (FAILED(SHCreateItemFromParsingName(item.c_str(), nullptr, IID_PPV_ARGS(factory.put())))) return fail(L"the shell has no item " + item);
  HBITMAP bitmap = nullptr;
  if (FAILED(factory->GetImage(SIZE{edge, edge}, SIIGBF_ICONONLY, &bitmap)) || bitmap == nullptr) return fail(L"the shell drew no icon for " + item);
  BITMAP shape = {};
  GetObjectW(bitmap, sizeof(shape), &shape);
  BITMAPINFO info = {};
  info.bmiHeader.biSize = sizeof(info.bmiHeader);
  info.bmiHeader.biWidth = shape.bmWidth;
  info.bmiHeader.biHeight = -shape.bmHeight;
  info.bmiHeader.biPlanes = 1;
  info.bmiHeader.biBitCount = 32;
  info.bmiHeader.biCompression = BI_RGB;
  std::vector<BYTE> pixels((size_t)shape.bmWidth * (size_t)shape.bmHeight * 4);
  HDC screen = GetDC(nullptr);
  int rows = GetDIBits(screen, bitmap, 0, (UINT)shape.bmHeight, pixels.data(), &info, DIB_RGB_COLORS);
  ReleaseDC(nullptr, screen);
  DeleteObject(bitmap);
  if (rows != shape.bmHeight || pixels.empty()) return fail(L"the shell icon could not be read");
  const BYTE *pixel = pixels.data() + ((size_t)(shape.bmHeight / 2) * (size_t)shape.bmWidth + (size_t)(shape.bmWidth / 2)) * 4;
  wprintf(L"{\"tile\":[%d,%d,%d],\"alpha\":%d,\"edge\":%d}\n", pixel[2], pixel[1], pixel[0], pixel[3], shape.bmWidth);
  return 0;
}

int wmain(int argc, wchar_t **argv) {
  try {
    if (argc == 4 && wcscmp(argv[1], L"install") == 0) {
      winrt::init_apartment();
      return install(argv[2], argv[3]);
    }
    if (argc == 3 && wcscmp(argv[1], L"remove") == 0) {
      winrt::init_apartment();
      return uninstall(argv[2]);
    }
    if (argc == 4 && wcscmp(argv[1], L"tile") == 0) {
      SetProcessDPIAware();
      winrt::init_apartment(winrt::apartment_type::single_threaded);
      return tile(argv[2], argv[3]);
    }
  } catch (const winrt::hresult_error &error) {
    return fail(std::wstring(error.message()));
  }
  return fail(L"usage: WindowsPackageCheck install <msix> <identity name> | WindowsPackageCheck remove <package full name> | WindowsPackageCheck tile <app user model id> <edge>");
}
