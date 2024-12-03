cmake_minimum_required(VERSION 3.27)
project(libusb_podxt_test C)

set(CMAKE_C_STANDARD 11)

add_executable(libusb_podxt_test main.c)

find_package(PkgConfig REQUIRED)
pkg_check_modules(USB REQUIRED libusb-1.0)

target_link_libraries(libusb_podxt_test ${USB_LIBRARIES})
target_include_directories(libusb_podxt_test PUBLIC ${USB_INCLUDE_DIRS})
target_compile_options(libusb_podxt_test PUBLIC ${USB_CFLAGS_OTHER})
