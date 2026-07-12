cmake_minimum_required(VERSION 3.21)

project(FubukiBrowserAlpha LANGUAGES CXX OBJCXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_OSX_DEPLOYMENT_TARGET "12.0" CACHE STRING "Minimum macOS version")
set(USE_SANDBOX ON CACHE BOOL "Enable the CEF sandbox for browser processes")

if(USE_SANDBOX)
  set(FUBUKI_SANDBOX_DEFINES CEF_USE_SANDBOX)
else()
  set(FUBUKI_SANDBOX_DEFINES)
endif()

set(CEF_ROOT "" CACHE PATH "Path to CEF binary distribution")
if(NOT CEF_ROOT AND DEFINED ENV{CEF_ROOT})
  set(CEF_ROOT "$ENV{CEF_ROOT}")
endif()
if(NOT CEF_ROOT)
  set(CEF_ROOT "${CMAKE_SOURCE_DIR}/../third_party/cef")
endif()
if(NOT EXISTS "${CEF_ROOT}/cmake/cef_variables.cmake")
  message(FATAL_ERROR "CEF_ROOT must point to a CEF binary distribution. Current value: ${CEF_ROOT}")
endif()

list(APPEND CMAKE_MODULE_PATH "${CEF_ROOT}/cmake")
find_package(CEF REQUIRED)

set(FUBUKI_CEF_COMPILER_FLAGS ${CEF_COMPILER_FLAGS})
list(REMOVE_ITEM FUBUKI_CEF_COMPILER_FLAGS -Werror)

set(FUBUKI_SOURCES
  src/main.cc
  src/app/FubukiApp.cc
  src/cef/FubukiCefApp.cc
  src/cef/FubukiClient.cc
  src/cef/FubukiRenderApp.cc
  src/cef/FubukiSchemeHandler.cc
  src/browser/BrowserAppController.cc
  src/browser/BrowserWindow.mm
  src/browser/FrostStore.cc
  src/browser/Tab.cc
  src/browser/TabManager.cc
  src/bridge/FrostBridge.cc
  src/bridge/NativeBridge.cc
  src/commands/CommandRegistry.cc
  src/events/EventBus.cc
  src/utils/UrlUtils.cc
  src/platform/mac/AppDelegate.mm
  src/platform/mac/MacMenu.mm
)

set(FUBUKI_HELPER_SOURCES
  src/platform/mac/ProcessHelper.cc
  src/cef/FubukiRenderApp.cc
)

add_subdirectory("${CEF_LIBCEF_DLL_WRAPPER_PATH}" libcef_dll_wrapper)

if(CMAKE_BUILD_TYPE STREQUAL "Debug")
  set(FUBUKI_RUST_PROFILE "debug")
  set(FUBUKI_CARGO_PROFILE_ARG "")
else()
  set(FUBUKI_RUST_PROFILE "release")
  set(FUBUKI_CARGO_PROFILE_ARG "--release")
endif()
set(FUBUKI_FROST_FFI_LIB "${CMAKE_SOURCE_DIR}/../target/${FUBUKI_RUST_PROFILE}/libfrost_ffi.a")
add_custom_command(
  OUTPUT "${FUBUKI_FROST_FFI_LIB}"
  COMMAND ${CMAKE_COMMAND} -E env "MACOSX_DEPLOYMENT_TARGET=${CMAKE_OSX_DEPLOYMENT_TARGET}" cargo build -p frost-ffi ${FUBUKI_CARGO_PROFILE_ARG}
  WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}/.."
  DEPENDS
    "${CMAKE_SOURCE_DIR}/../Cargo.toml"
    "${CMAKE_SOURCE_DIR}/../crates/frost-ffi/src/lib.rs"
    "${CMAKE_SOURCE_DIR}/../crates/frost-core/src/lib.rs"
    "${CMAKE_SOURCE_DIR}/../crates/frost-protocol/src/lib.rs"
    "${CMAKE_SOURCE_DIR}/../crates/frost-store/src/lib.rs"
  VERBATIM
)
add_custom_target(frost_ffi ALL DEPENDS "${FUBUKI_FROST_FFI_LIB}")
add_library(frost_ffi_static STATIC IMPORTED GLOBAL)
set_target_properties(frost_ffi_static PROPERTIES IMPORTED_LOCATION "${FUBUKI_FROST_FFI_LIB}")

add_executable(FubukiBrowserAlpha MACOSX_BUNDLE ${FUBUKI_SOURCES})
add_dependencies(FubukiBrowserAlpha frost_ffi)
set_target_properties(FubukiBrowserAlpha PROPERTIES
  OUTPUT_NAME "Fubuki Browser Alpha"
  MACOSX_BUNDLE_INFO_PLIST "${CMAKE_CURRENT_SOURCE_DIR}/resources/Info.plist.in"
)

target_include_directories(FubukiBrowserAlpha PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/src"
  "${CEF_ROOT}"
)
target_compile_definitions(FubukiBrowserAlpha PRIVATE
  ${CEF_COMPILER_DEFINES}
  ${FUBUKI_SANDBOX_DEFINES}
)
target_compile_definitions(FubukiBrowserAlpha PRIVATE
  FUBUKI_UI_DIST="${CMAKE_CURRENT_SOURCE_DIR}/../ui/dist"
)
target_compile_options(FubukiBrowserAlpha PRIVATE
  ${FUBUKI_CEF_COMPILER_FLAGS}
  -Wall
  -Wextra
  -Wpedantic
  -Wno-gnu-zero-variadic-macro-arguments
)
target_link_options(FubukiBrowserAlpha PRIVATE ${CEF_LINKER_FLAGS})
target_link_libraries(FubukiBrowserAlpha PRIVATE libcef_dll_wrapper frost_ffi_static ${CEF_STANDARD_LIBS} sqlite3 "-framework QuartzCore")

set(FUBUKI_APP_BUNDLE "$<TARGET_BUNDLE_DIR:FubukiBrowserAlpha>")
COPY_MAC_FRAMEWORK(FubukiBrowserAlpha "${CEF_BINARY_DIR}" "${FUBUKI_APP_BUNDLE}")

foreach(_suffix_list ${CEF_HELPER_APP_SUFFIXES})
  string(REPLACE ":" ";" _suffix_list ${_suffix_list})
  list(GET _suffix_list 0 _name_suffix)
  list(GET _suffix_list 1 _target_suffix)
  list(GET _suffix_list 2 _plist_suffix)

  set(_helper_target "FubukiBrowserAlphaHelper${_target_suffix}")
  set(_helper_output_name "Fubuki Browser Alpha Helper${_name_suffix}")
  set(_helper_info_plist "${CMAKE_CURRENT_BINARY_DIR}/helper-Info${_target_suffix}.plist")

  file(READ "${CMAKE_CURRENT_SOURCE_DIR}/resources/helper-Info.plist.in" _plist_contents)
  string(REPLACE "\${EXECUTABLE_NAME}" "${_helper_output_name}" _plist_contents "${_plist_contents}")
  string(REPLACE "\${PRODUCT_NAME}" "${_helper_output_name}" _plist_contents "${_plist_contents}")
  string(REPLACE "\${BUNDLE_ID_SUFFIX}" "${_plist_suffix}" _plist_contents "${_plist_contents}")
  string(REPLACE "\${CMAKE_OSX_DEPLOYMENT_TARGET}" "${CMAKE_OSX_DEPLOYMENT_TARGET}" _plist_contents "${_plist_contents}")
  file(WRITE "${_helper_info_plist}" "${_plist_contents}")

  add_executable(${_helper_target} MACOSX_BUNDLE ${FUBUKI_HELPER_SOURCES})
  target_include_directories(${_helper_target} PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/src"
    "${CEF_ROOT}"
  )
  target_compile_definitions(${_helper_target} PRIVATE
    ${CEF_COMPILER_DEFINES}
    ${FUBUKI_SANDBOX_DEFINES}
  )
  target_compile_options(${_helper_target} PRIVATE
    ${FUBUKI_CEF_COMPILER_FLAGS}
    -Wno-gnu-zero-variadic-macro-arguments
  )
  target_link_options(${_helper_target} PRIVATE ${CEF_LINKER_FLAGS})
  target_link_libraries(${_helper_target} PRIVATE libcef_dll_wrapper ${CEF_STANDARD_LIBS})
  set_target_properties(${_helper_target} PROPERTIES
    MACOSX_BUNDLE_INFO_PLIST "${_helper_info_plist}"
    OUTPUT_NAME "${_helper_output_name}"
  )

  add_dependencies(FubukiBrowserAlpha ${_helper_target})
  add_custom_command(
    TARGET FubukiBrowserAlpha
    POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
      "$<TARGET_BUNDLE_DIR:${_helper_target}>"
      "${FUBUKI_APP_BUNDLE}/Contents/Frameworks/${_helper_output_name}.app"
    VERBATIM
  )
endforeach()
