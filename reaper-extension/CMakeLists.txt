cmake_minimum_required(VERSION 3.15)
project(reaper_toneforge VERSION 1.0.0)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# REAPER SDK path (kullanıcı düzenleyecek)
set(REAPER_SDK_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../external/reaper-sdk" CACHE PATH "Path to REAPER SDK")

# Include directories
include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${REAPER_SDK_DIR}/sdk
    ${CMAKE_CURRENT_SOURCE_DIR}/../external/cpp-httplib
    ${CMAKE_CURRENT_SOURCE_DIR}/../external/json/single_include
)

# Source files
set(SOURCES
    src/reaper_toneforge.cpp
)

# Create shared library (DLL on Windows, dylib on macOS)
add_library(reaper_toneforge SHARED ${SOURCES})

# Platform-specific settings
if(WIN32)
    # Windows: Create .dll
    set_target_properties(reaper_toneforge PROPERTIES
        PREFIX ""
        SUFFIX ".dll"
        RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin"
    )
    
    # Link Windows libraries
    target_link_libraries(reaper_toneforge
        ws2_32  # Winsock for networking
    )
    
    # MSVC specific
    if(MSVC)
        target_compile_options(reaper_toneforge PRIVATE /W4)
        target_compile_definitions(reaper_toneforge PRIVATE _CRT_SECURE_NO_WARNINGS)
    endif()
    
elseif(APPLE)
    # macOS: Create .dylib
    set_target_properties(reaper_toneforge PROPERTIES
        PREFIX ""
        SUFFIX ".dylib"
        LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin"
    )
    
    target_link_libraries(reaper_toneforge
        "-framework CoreFoundation"
    )
endif()

# Installation
if(WIN32)
    # Windows: Install to REAPER UserPlugins
    install(TARGETS reaper_toneforge
        RUNTIME DESTINATION "$ENV{APPDATA}/REAPER/UserPlugins"
    )
else()
    # macOS: Install to user plugins
    install(TARGETS reaper_toneforge
        LIBRARY DESTINATION "$ENV{HOME}/Library/Application Support/REAPER/UserPlugins"
    )
endif()

# Print build info
message(STATUS "ToneForge REAPER Extension")
message(STATUS "  Version: ${PROJECT_VERSION}")
message(STATUS "  REAPER SDK: ${REAPER_SDK_DIR}")
message(STATUS "  Build type: ${CMAKE_BUILD_TYPE}")
