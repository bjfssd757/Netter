cmake_minimum_required(VERSION 3.21)
project(Netter LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(MSVC)
    add_compile_options(/Zc:__cplusplus)
endif()

set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>DLL")

set(NETTER_PATH "/target/release/netter")
add_definitions(-DNETTER_PATH="${NETTER_PATH}")

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOMOC_PATH_PREFIX ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets)
message(STATUS "Found Qt ${Qt6_VERSION} in ${Qt6_DIR}")
message(STATUS "Using generator: ${CMAKE_GENERATOR}")
message(STATUS "Using C++ compiler: ${CMAKE_CXX_COMPILER}")

set(PROJECT_SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src/client)
set(PROJECT_SOURCES
    ${PROJECT_SOURCE_DIR}/main.cpp
    ${PROJECT_SOURCE_DIR}/main.cpp
    ${PROJECT_SOURCE_DIR}/ui/mainwindow.cpp
    ${PROJECT_SOURCE_DIR}/ui/mainwindow.h
    ${PROJECT_SOURCE_DIR}/ui/components/header.cpp
    ${PROJECT_SOURCE_DIR}/ui/components/header.h
    ${PROJECT_SOURCE_DIR}/ui/components/footer.cpp
    ${PROJECT_SOURCE_DIR}/ui/components/footer.h
    ${PROJECT_SOURCE_DIR}/ui/components/sidebar.cpp
    ${PROJECT_SOURCE_DIR}/ui/components/sidebar.h
    ${PROJECT_SOURCE_DIR}/ui/components/editor.cpp
    ${PROJECT_SOURCE_DIR}/ui/components/editor.h
    ${PROJECT_SOURCE_DIR}/ui/components/line_number_area.cpp
    ${PROJECT_SOURCE_DIR}/ui/components/line_number_area.h
    ${PROJECT_SOURCE_DIR}/ui/components/template_panel.cpp
    ${PROJECT_SOURCE_DIR}/ui/components/template_panel.h
    ${PROJECT_SOURCE_DIR}/ui/dialogs/error_dialog.cpp
    ${PROJECT_SOURCE_DIR}/ui/dialogs/error_dialog.h
    ${PROJECT_SOURCE_DIR}/ui/dialogs/settings_dialog.cpp
    ${PROJECT_SOURCE_DIR}/ui/dialogs/settings_dialog.h
    ${PROJECT_SOURCE_DIR}/models/route_model.cpp
    ${PROJECT_SOURCE_DIR}/models/route_model.h
    ${PROJECT_SOURCE_DIR}/models/template_model.cpp
    ${PROJECT_SOURCE_DIR}/models/template_model.h
    ${PROJECT_SOURCE_DIR}/core/cli_interface.cpp
    ${PROJECT_SOURCE_DIR}/core/cli_interface.h
    ${PROJECT_SOURCE_DIR}/core/settings_manager.h
    ${PROJECT_SOURCE_DIR}/core/settings_manager.cpp
    ${PROJECT_SOURCE_DIR}/utils/file_utils.cpp
    ${PROJECT_SOURCE_DIR}/utils/file_utils.h
    ${PROJECT_SOURCE_DIR}/utils/error_handler.cpp
    ${PROJECT_SOURCE_DIR}/utils/error_handler.h
    resources.qrc
)

add_executable(${PROJECT_NAME} ${PROJECT_SOURCES})

target_include_directories(${PROJECT_NAME} PRIVATE ${PROJECT_SOURCE_DIR})

target_link_libraries(${PROJECT_NAME} PRIVATE Qt6::Core Qt6::Gui Qt6::Widgets)

set_target_properties(${PROJECT_NAME} PROPERTIES RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}) # Place .exe/.app in build folder

add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy $<TARGET_RUNTIME_DLLS:${PROJECT_NAME}> $<TARGET_FILE_DIR:${PROJECT_NAME}>
    COMMAND_EXPAND_LISTS
    COMMENT "Copying Qt runtime dependencies for ${PROJECT_NAME}"
    VERBATIM
)

if(WIN32)

    find_program(WINDEPLOYQT_EXECUTABLE windeployqt HINTS "${CMAKE_PREFIX_PATH}/bin")
    if(WINDEPLOYQT_EXECUTABLE)
        add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
            COMMAND ${WINDEPLOYQT_EXECUTABLE} --no-translations --no-system-d3d-compiler "$<TARGET_FILE:${PROJECT_NAME}>"
            COMMENT "Running windeployqt to copy Qt dependencies..."
        )
    else()
        message(WARNING "windeployqt not found, Qt dependencies will not be automatically copied")
    endif()

elseif(UNIX AND NOT APPLE)

    find_program(LINUXDEPLOYQT_EXECUTABLE linuxdeployqt HINTS "${CMAKE_PREFIX_PATH}/bin")
    if(LINUXDEPLOYQT_EXECUTABLE)
        add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
            COMMAND ${LINUXDEPLOYQT_EXECUTABLE} "$<TARGET_FILE:${PROJECT_NAME}>" -always-overwrite -no-translations
            COMMENT "Running linuxdeployqt to copy Qt dependencies..."
        )
    else()
        message(STATUS "linuxdeployqt not found, attempting to use qt-deploy")

        file(WRITE "${CMAKE_BINARY_DIR}/qt-deploy.sh"
            "#!/bin/bash\n"
            "echo 'Copying Qt dependencies...'\n"
            "EXECUTABLE=\"$<TARGET_FILE:${PROJECT_NAME}>\"\n"
            "DEST_DIR=\"$(dirname \"$EXECUTABLE\")\"\n"
            "ldd \"$EXECUTABLE\" | grep -i qt | awk '{print $3}' | xargs -I{} cp -v {} \"$DEST_DIR\"\n"
        )
        execute_process(COMMAND chmod +x "${CMAKE_BINARY_DIR}/qt-deploy.sh")
        
        add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
            COMMAND "${CMAKE_BINARY_DIR}/qt-deploy.sh"
            COMMENT "Running qt-deploy script to copy Qt dependencies..."
        )
    endif()
endif()