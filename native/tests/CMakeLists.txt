cmake_minimum_required(VERSION 3.21)
project(FubukiTests LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(FetchContent)
FetchContent_Declare(
  googletest
  GIT_REPOSITORY https://github.com/google/googletest.git
  GIT_TAG v1.15.2
)
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(googletest)

enable_testing()

add_executable(fubuki_tests
  UrlUtilsTest.cc
  JsonUtilsTest.cc
  HostCommandRoutingTest.cc
  SidebarLayoutStateTest.cc
  TabContextMenuStateTest.cc
  # Add more test files here as the project grows
  "${CMAKE_CURRENT_SOURCE_DIR}/../src/utils/UrlUtils.cc"
)

target_include_directories(fubuki_tests PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/../src"
)

target_link_libraries(fubuki_tests PRIVATE
  GTest::gtest_main
)

include(GoogleTest)
gtest_discover_tests(fubuki_tests)
