cmake_minimum_required(VERSION 3.16)

project(NetterUI VERSION 0.1 LANGUAGES CXX)

set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)

set(CMAKE_PREFIX_PATH "E:/msys2/ucrt64")

find_package(QT NAMES Qt6 REQUIRED COMPONENTS Widgets)
find_package(Qt${QT_VERSION_MAJOR} REQUIRED COMPONENTS Widgets)

set(PROJECT_SOURCES
    src/client/main.cpp
    src/client/ui/mainwindow.cpp
    src/client/ui/mainwindow.h
    src/client/ui/components/header.cpp
    src/client/ui/components/header.h
    src/client/ui/components/footer.cpp
    src/client/ui/components/footer.h
    src/client/ui/components/sidebar.cpp
    src/client/ui/components/sidebar.h
    src/client/ui/components/editor.cpp
    src/client/ui/components/editor.h
    src/client/ui/components/line_number_area.cpp
    src/client/ui/components/line_number_area.h
    src/client/ui/components/template_panel.cpp
    src/client/ui/components/template_panel.h
    src/client/ui/dialogs/error_dialog.cpp
    src/client/ui/dialogs/error_dialog.h
    src/client/ui/dialogs/settings_dialog.cpp
    src/client/ui/dialogs/settings_dialog.h
    src/client/models/route_model.cpp
    src/client/models/route_model.h
    src/client/models/template_model.cpp
    src/client/models/template_model.h
    src/client/core/cli_interface.cpp
    src/client/core/cli_interface.h
    src/client/core/settings_manager.h
    src/client/core/settings_manager.cpp
    src/client/utils/file_utils.cpp
    src/client/utils/file_utils.h
    src/client/utils/error_handler.cpp
    src/client/utils/error_handler.h
    resources.qrc
)

add_definitions(-DQT_MESSAGELOGCONTEXT)

if(${QT_VERSION_MAJOR} GREATER_EQUAL 6)
    qt_add_executable(NetterUI
        MANUAL_FINALIZATION
        ${PROJECT_SOURCES}
    )
else()
    add_executable(NetterUI
        ${PROJECT_SOURCES}
    )
endif()

target_link_libraries(NetterUI PRIVATE Qt${QT_VERSION_MAJOR}::Widgets)

if(MINGW)
    set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -municode")
    
    target_link_options(NetterUI PRIVATE -Wl,--subsystem,windows)
endif()

set_target_properties(NetterUI PROPERTIES
    WIN32_EXECUTABLE TRUE
)

include(GNUInstallDirs)
install(TARGETS NetterUI
    BUNDLE DESTINATION .
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)

if(QT_VERSION_MAJOR EQUAL 6)
    qt_finalize_executable(NetterUI)
endif()

if(WIN32)
    add_custom_command(TARGET NetterUI POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E echo "Deploying Qt dependencies..."
        COMMAND "E:/msys2/ucrt64/bin/windeployqt6.exe" "$<TARGET_FILE:NetterUI>"
    )
endif()