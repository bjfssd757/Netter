cmake_minimum_required(VERSION 3.16)

if(WIN32)
    execute_process(
        COMMAND powershell -Command "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8"
        COMMAND cmd /c chcp 65001
    )
endif()

project(NetterUI VERSION 0.1 LANGUAGES CXX)

include(FetchContent)
include(ExternalProject)

set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)

set(MSYS2_INSTALL_DIR "E:/msys2" CACHE PATH "Path to MSYS2 installation")

set(CMAKE_PREFIX_PATH ${CMAKE_PREFIX_PATH} 
    "C:/Qt"
    "E:/Qt"
    "D:/Qt"
    ${MSYS2_INSTALL_DIR}/ucrt64
)

find_package(QT NAMES Qt6 Qt5 QUIET)
if(QT_FOUND)
    message(STATUS "Qt${QT_VERSION_MAJOR} найден в: ${Qt${QT_VERSION_MAJOR}_DIR}")
    set(QT_DIR ${Qt${QT_VERSION_MAJOR}_DIR})
    set(QT_INSTALLED TRUE)
else()
    message(STATUS "Qt не найден в стандартных путях, ищем в других местах...")
    set(QT_INSTALLED FALSE)
endif()

function(check_msys2_installed RESULT)
    if(EXISTS "${MSYS2_INSTALL_DIR}/ucrt64/bin/gcc.exe")
        set(${RESULT} TRUE PARENT_SCOPE)
        message(STATUS "MSYS2 найден в: ${MSYS2_INSTALL_DIR}")
    else()
        set(${RESULT} FALSE PARENT_SCOPE)
        message(STATUS "MSYS2 не найден в: ${MSYS2_INSTALL_DIR}")
    endif()
endfunction()

function(find_qt_installation RESULT)

    set(QT_VERSION_DIRS "6.6.2" "6.6.1" "6.6.0" "6.5.3" "6.5.2" "6.5.1" "6.5.0" "6.4.3")
    set(QT_COMPILER_DIRS "msvc2019_64" "msvc2019" "mingw_64" "mingw" "gcc_64" "clang_64")
    
    foreach(QT_ROOT "C:/Qt" "D:/Qt" "E:/Qt" "$ENV{ProgramFiles}/Qt" "$ENV{ProgramFiles\(x86\)}/Qt")
        if(EXISTS "${QT_ROOT}")
            foreach(VERSION ${QT_VERSION_DIRS})
                foreach(COMPILER ${QT_COMPILER_DIRS})
                    if(EXISTS "${QT_ROOT}/${VERSION}/${COMPILER}/bin")
                        message(STATUS "Найден Qt в: ${QT_ROOT}/${VERSION}/${COMPILER}")
                        set(QT_DIR "${QT_ROOT}/${VERSION}/${COMPILER}" CACHE PATH "Директория установки Qt" FORCE)
                        list(APPEND CMAKE_PREFIX_PATH "${QT_DIR}")
                        set(CMAKE_PREFIX_PATH "${CMAKE_PREFIX_PATH}" CACHE PATH "Путь префикса CMake" FORCE)
                        set(${RESULT} TRUE PARENT_SCOPE)
                        return()
                    endif()
                endforeach()
            endforeach()
        endif()
    endforeach()

    if(EXISTS "${MSYS2_INSTALL_DIR}/ucrt64/bin/qmake6.exe" OR 
       EXISTS "${MSYS2_INSTALL_DIR}/ucrt64/bin/qmake.exe")
        message(STATUS "Найден Qt в MSYS2: ${MSYS2_INSTALL_DIR}/ucrt64")
        set(QT_DIR "${MSYS2_INSTALL_DIR}/ucrt64" CACHE PATH "Директория установки Qt" FORCE)
        list(APPEND CMAKE_PREFIX_PATH "${QT_DIR}")
        set(CMAKE_PREFIX_PATH "${CMAKE_PREFIX_PATH}" CACHE PATH "Путь префикса CMake" FORCE)
        set(${RESULT} TRUE PARENT_SCOPE)
        return()
    endif()
    
    set(${RESULT} FALSE PARENT_SCOPE)
endfunction()

function(install_msys2)
    message(STATUS "Загружаем установщик MSYS2...")

    set(MSYS2_URL "https://github.com/msys2/msys2-installer/releases/download/2023-05-26/msys2-x86_64-20230526.exe")
    set(MSYS2_INSTALLER "${CMAKE_BINARY_DIR}/msys2-installer.exe")

    file(DOWNLOAD 
        "${MSYS2_URL}" 
        "${MSYS2_INSTALLER}" 
        SHOW_PROGRESS 
        STATUS DOWNLOAD_STATUS)
    
    list(GET DOWNLOAD_STATUS 0 STATUS_CODE)
    if(NOT STATUS_CODE EQUAL 0)
        message(FATAL_ERROR "Ошибка загрузки установщика MSYS2: ${DOWNLOAD_STATUS}")
    endif()
    
    message(STATUS "Устанавливаем MSYS2 в ${MSYS2_INSTALL_DIR}...")

    execute_process(
        COMMAND "${MSYS2_INSTALLER}" --platform minimal --installdir "${MSYS2_INSTALL_DIR}" --confirm-command
        RESULT_VARIABLE INSTALLER_RESULT
    )
    
    if(NOT INSTALLER_RESULT EQUAL 0)
        message(FATAL_ERROR "Установка MSYS2 завершилась с ошибкой: ${INSTALLER_RESULT}")
    endif()
    
    message(STATUS "MSYS2 установлен успешно!")

    message(STATUS "Обновляем пакеты MSYS2...")
    execute_process(
        COMMAND "${MSYS2_INSTALL_DIR}/usr/bin/bash" -lc "pacman -Syu --noconfirm"
        RESULT_VARIABLE UPDATE_RESULT
    )
    
    if(NOT UPDATE_RESULT EQUAL 0)
        message(WARNING "Обновление MSYS2 завершилось с кодом: ${UPDATE_RESULT}")
    endif()
endfunction()

function(install_qt_msys2)
    message(STATUS "Устанавливаем Qt6 через MSYS2...")

    execute_process(
        COMMAND "${MSYS2_INSTALL_DIR}/usr/bin/bash" -lc "pacman -S --noconfirm mingw-w64-ucrt-x86_64-qt6-base mingw-w64-ucrt-x86_64-qt6-tools mingw-w64-ucrt-x86_64-qt6-declarative mingw-w64-ucrt-x86_64-qt6-svg mingw-w64-ucrt-x86_64-gcc"
        RESULT_VARIABLE QT_INSTALL_RESULT
        OUTPUT_VARIABLE QT_INSTALL_OUTPUT
    )

    string(REPLACE "\n" ";" QT_INSTALL_OUTPUT_LIST "${QT_INSTALL_OUTPUT}")
    foreach(LINE ${QT_INSTALL_OUTPUT_LIST})
        if(LINE MATCHES "downloading|installing|upgrading")
            message(STATUS "${LINE}")
        endif()
    endforeach()
    
    if(NOT QT_INSTALL_RESULT EQUAL 0)
        message(FATAL_ERROR "Установка Qt6 завершилась с ошибкой: ${QT_INSTALL_RESULT}")
    endif()
    
    message(STATUS "Qt6 установлен успешно!")
endfunction()

check_msys2_installed(MSYS2_FOUND)
if(NOT MSYS2_FOUND)
    message(STATUS "MSYS2 не найден, устанавливаем...")
    install_msys2()

    check_msys2_installed(MSYS2_INSTALLED)
    if(NOT MSYS2_INSTALLED)
        message(FATAL_ERROR "Установка MSYS2 завершилась неудачно!")
    endif()
endif()

if(NOT QT_INSTALLED)

    find_qt_installation(QT_FOUND)
    
    if(NOT QT_FOUND)
        message(STATUS "Qt не найден, устанавливаем через MSYS2...")
        install_qt_msys2()
    else()
        message(STATUS "Используем Qt из: ${QT_DIR}")
    endif()
endif()

find_package(QT NAMES Qt6 Qt5 REQUIRED COMPONENTS Widgets)
find_package(Qt${QT_VERSION_MAJOR} REQUIRED COMPONENTS Widgets)

message(STATUS "Используем Qt ${Qt${QT_VERSION_MAJOR}_VERSION} из ${Qt${QT_VERSION_MAJOR}_DIR}")

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

    find_program(WINDEPLOYQT
        NAMES windeployqt6.exe windeployqt.exe
        PATHS
            "${QT_DIR}/bin"
            "${MSYS2_INSTALL_DIR}/ucrt64/bin"
            "${MSYS2_INSTALL_DIR}/mingw64/bin"
    )

    file(WRITE "${CMAKE_BINARY_DIR}/run_netter_ui.bat" 
        "@echo off\n"
        "chcp 65001 > nul\n"
        "cd /d \"%~dp0bin\"\n"
        "NetterUI.exe\n"
    )

    add_custom_command(TARGET NetterUI POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E echo "Копирование библиотек MSYS2/MinGW..."

        COMMAND ${CMAKE_COMMAND} -E copy_if_different
            "${QT_DIR}/bin/libgcc_s_seh-1.dll"
            "${QT_DIR}/bin/libstdc++-6.dll"
            "${QT_DIR}/bin/libwinpthread-1.dll"
            "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}"
    )

    foreach(LIB "zlib1.dll" "libbz2-1.dll" "libpng16-16.dll" "libharfbuzz-0.dll" "libfreetype-6.dll" "libiconv-2.dll")
        if(EXISTS "${QT_DIR}/bin/${LIB}")
            add_custom_command(TARGET NetterUI POST_BUILD
                COMMAND ${CMAKE_COMMAND} -E copy_if_different
                    "${QT_DIR}/bin/${LIB}"
                    "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}"
            )
        endif()
    endforeach()

    if(WINDEPLOYQT)
        message(STATUS "Найден windeployqt: ${WINDEPLOYQT}")
        add_custom_command(TARGET NetterUI POST_BUILD
            COMMAND ${CMAKE_COMMAND} -E echo "Разворачиваем Qt зависимости с помощью ${WINDEPLOYQT}..."
            COMMAND "${WINDEPLOYQT}" --verbose --no-compiler-runtime "$<TARGET_FILE:NetterUI>"
        )
    else()
        message(STATUS "windeployqt не найден, копируем основные Qt библиотеки вручную")

        foreach(QT_LIB Core Gui Widgets)
            if(EXISTS "${QT_DIR}/bin/Qt6${QT_LIB}.dll")
                add_custom_command(TARGET NetterUI POST_BUILD
                    COMMAND ${CMAKE_COMMAND} -E copy_if_different
                        "${QT_DIR}/bin/Qt6${QT_LIB}.dll"
                        "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}"
                )
            elseif(EXISTS "${QT_DIR}/bin/Qt5${QT_LIB}.dll")
                add_custom_command(TARGET NetterUI POST_BUILD
                    COMMAND ${CMAKE_COMMAND} -E copy_if_different
                        "${QT_DIR}/bin/Qt5${QT_LIB}.dll"
                        "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}"
                )
            endif()
        endforeach()

        add_custom_command(TARGET NetterUI POST_BUILD
            COMMAND ${CMAKE_COMMAND} -E make_directory
                "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/platforms"
                "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/styles"
                "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/imageformats"
        )

        if(EXISTS "${QT_DIR}/plugins/platforms/qwindows.dll")
            add_custom_command(TARGET NetterUI POST_BUILD
                COMMAND ${CMAKE_COMMAND} -E copy_if_different
                    "${QT_DIR}/plugins/platforms/qwindows.dll"
                    "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/platforms/"
            )
        endif()

        if(EXISTS "${QT_DIR}/plugins/styles/qwindowsvistastyle.dll")
            add_custom_command(TARGET NetterUI POST_BUILD
                COMMAND ${CMAKE_COMMAND} -E copy_if_different
                    "${QT_DIR}/plugins/styles/qwindowsvistastyle.dll"
                    "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/styles/"
            )
        endif()

        foreach(FORMAT qgif qico qjpeg qsvg)
            if(EXISTS "${QT_DIR}/plugins/imageformats/${FORMAT}.dll")
                add_custom_command(TARGET NetterUI POST_BUILD
                    COMMAND ${CMAKE_COMMAND} -E copy_if_different
                        "${QT_DIR}/plugins/imageformats/${FORMAT}.dll"
                        "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/imageformats/"
                )
            endif()
        endforeach()
    endif()

    add_custom_command(TARGET NetterUI POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E echo "Сборка завершена! Исполняемый файл находится в ${CMAKE_RUNTIME_OUTPUT_DIRECTORY}"
        COMMAND ${CMAKE_COMMAND} -E echo "Вы можете запустить приложение, используя run_netter_ui.bat в директории сборки"
    )
endif()