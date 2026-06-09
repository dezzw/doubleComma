cmake_minimum_required(VERSION 3.20)

project(rootcall VERSION 0.1.0 LANGUAGES CXX)

add_executable(rootcall src/main.cpp)
set_target_properties(rootcall PROPERTIES OUTPUT_NAME ",,")
target_compile_features(rootcall PRIVATE cxx_std_20)

if(MSVC)
  target_compile_options(rootcall PRIVATE /W4)
else()
  target_compile_options(rootcall PRIVATE -Wall -Wextra -Wpedantic)
endif()
