#!/usr/bin/env bash
# Runs the Win32 tests that need Wine, and the MinGW compile checks, in the
# image from scripts/linux/wine.Dockerfile. On arm64 the Wine tests build ARM64
# Windows programs with llvm-mingw and run them under Wine 11.0 for aarch64;
# the compile checks stay on x86_64 MinGW GCC.
set -euo pipefail

wine_tests=(
  native_deep_link_tests::a_second_windows_instance_forwards_its_link_to_the_first
  native_external_link_tests::the_windows_link_opener_compiles_against_win32_and_opens_only_external_links
  native_menu_action_tests::windows_menu_actions_enable_with_a_live_listener_and_tell_the_page_under_wine
  native_rust_task_tests::native_rust_http_transport_reaches_a_local_yon_under_wine
  native_tray_tests::windows_tray_code_adds_an_icon_hides_on_close_and_quits_under_wine
  windows_browser_api_tests::the_windows_browser_apis_follow_the_manifest_grants_under_wine
  windows_file_picker_tests::the_windows_file_picker_keeps_the_contract_under_wine
  windows_file_picker_tests::the_windows_file_picker_selects_a_file_in_the_wine_dialog
  windows_package_tests::the_package_folder_helper_runs_unpackaged_under_wine
  windows_task_resume_tests::windows_task_resume_posts_one_message_per_wake_burst
  windows_title_bar_tests::the_windows_title_bar_drives_a_win32_window_under_wine
  windows_window_background_tests::the_windows_window_background_paints_a_win32_window_under_wine
)
compile_checks=(
  native_development_preview_tests::the_preview_windows_host_compiles_against_the_win32_and_webview2_headers
  native_rust_task_tests::native_rust_http_transport_compiles_for_windows
  windows_host_tests::windows_application_state_code_compiles_against_the_win32_headers
  windows_host_tests::windows_menu_code_compiles_and_links_against_the_win32_headers
  windows_title_bar_tests::the_windows_host_with_each_title_bar_compiles_against_the_win32_and_webview2_headers
  windows_web_engine_check::tests::the_webview2_check_compiles_against_the_win32_headers
  windows_window_background_tests::the_windows_host_with_background_color_compiles_against_the_win32_and_webview2_headers
  windows_window_frame_tests::the_windows_window_frame_compiles_and_links_against_the_win32_headers
)

wine --version
for compiler in aarch64-w64-mingw32-gcc x86_64-w64-mingw32-gcc; do
  if command -v "$compiler" >/dev/null; then
    echo "$compiler: $("$compiler" --version | head -n 1)"
  fi
done

selected=("${wine_tests[@]}")
if [[ ${1:-} == --with-compile-checks ]]; then
  selected+=("${compile_checks[@]}")
fi

export LANG=C.UTF-8 WINEDEBUG=-all
xvfb-run -a -s "-screen 0 1920x1080x24" \
  cargo test --locked -p tachyon-core --lib -- \
  --include-ignored --exact --test-threads=1 "${selected[@]/#/native::}"
