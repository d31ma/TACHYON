ty bundle --target all
ty preview --target web
ty preview --target macos
TAC_BUILD_JOBS=1 ty bundle
ty cache status
