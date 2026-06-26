# Prebuilt ui_extractor.dll from build hook or cargo build --release.
cmake_minimum_required(VERSION 3.14)

set(PROJECT_NAME "ui_extractor")
project(${PROJECT_NAME} LANGUAGES C)

if(CMAKE_SYSTEM_PROCESSOR MATCHES "aarch64|ARM64")
  set(UI_EXTRACTOR_ARCH "arm64")
else()
  set(UI_EXTRACTOR_ARCH "x64")
endif()

set(_LIB_PATH
  "${CMAKE_CURRENT_SOURCE_DIR}/../native/windows/${UI_EXTRACTOR_ARCH}/lib/ui_extractor.dll"
)

if(EXISTS "${_LIB_PATH}")
  set(ui_extractor_bundled_libraries "${_LIB_PATH}" PARENT_SCOPE)
else()
  set(ui_extractor_bundled_libraries "" PARENT_SCOPE)
endif()
