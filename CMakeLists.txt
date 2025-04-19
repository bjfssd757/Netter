cmake_minimum_required(VERSION 3.16)

project(NetterUI VERSION 0.1 LANGUAGES CXX)

set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Установка каталогов сборки
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)

# Путь к Qt из MSYS2 (если не находит автоматически)
set(CMAKE_PREFIX_PATH "E:/msys2/ucrt64")

# Найти компоненты Qt
find_package(QT NAMES Qt6 REQUIRED COMPONENTS Widgets)
find_package(Qt${QT_VERSION_MAJOR} REQUIRED COMPONENTS Widgets)

# Исходные файлы проекта
set(PROJECT_SOURCES
    src/client/main.cpp
    src/client/mainwindow.cpp
    src/client/headers/mainwindow.h
    src/client/cli.cpp
    resources.qrc
    src/client/headers/Footer.h
    src/client/footer.cpp
    src/client/headers/cli.h
    src/client/headers/Header.h
    src/client/header.cpp
)

# Создание исполняемого файла
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

# Линковка библиотек
target_link_libraries(NetterUI PRIVATE Qt${QT_VERSION_MAJOR}::Widgets)

# Специфические флаги для MinGW
if(MINGW)
    # Решение проблемы с __imp___argc
    set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -municode")
    
    # Добавляем нужные флаги для MinGW
    target_link_options(NetterUI PRIVATE -Wl,--subsystem,windows)
endif()

set_target_properties(NetterUI PROPERTIES
    WIN32_EXECUTABLE TRUE
)

# Установка
include(GNUInstallDirs)
install(TARGETS NetterUI
    BUNDLE DESTINATION .
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)

if(QT_VERSION_MAJOR EQUAL 6)
    qt_finalize_executable(NetterUI)
endif()

# Деплой DLL-файлов для Windows
if(WIN32)
    add_custom_command(TARGET NetterUI POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E echo "Deploying Qt dependencies..."
        COMMAND "E:/msys2/ucrt64/bin/windeployqt6.exe" "$<TARGET_FILE:NetterUI>"
    )
endif()