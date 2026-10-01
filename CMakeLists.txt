cmake_minimum_required(VERSION 3.22)

project(ntfy-kde-widget
    VERSION 0.1.0
    DESCRIPTION "Plasma 6 panel widget that raises ntfy notifications"
    LANGUAGES CXX
)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 6.6 REQUIRED COMPONENTS Core DBus Qml)

include(CTest)

# ---------------------------------------------------------------------------
# Rust core: a static library linked straight into the QML extension plugin.
# ---------------------------------------------------------------------------

find_program(CARGO_EXECUTABLE cargo REQUIRED)

set(NTFY_CORE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/rust)
set(NTFY_CORE_TARGET_DIR ${CMAKE_CURRENT_BINARY_DIR}/cargo)
set(NTFY_CORE_LIB ${NTFY_CORE_TARGET_DIR}/release/libntfy_core.a)

file(GLOB_RECURSE NTFY_CORE_SOURCES CONFIGURE_DEPENDS ${NTFY_CORE_DIR}/src/*.rs)

add_custom_command(
    OUTPUT ${NTFY_CORE_LIB}
    COMMAND ${CMAKE_COMMAND} -E env
            CARGO_TARGET_DIR=${NTFY_CORE_TARGET_DIR}
            ${CARGO_EXECUTABLE} build --release --manifest-path ${NTFY_CORE_DIR}/Cargo.toml
    DEPENDS ${NTFY_CORE_SOURCES} ${NTFY_CORE_DIR}/Cargo.toml
    COMMENT "Building the Rust core"
    VERBATIM
)
add_custom_target(ntfy_core_build DEPENDS ${NTFY_CORE_LIB})

# ---------------------------------------------------------------------------
# QML extension plugin. It exports the Bridge singleton, the only thing the
# applet's QML needs from C++. Shipping it as an ordinary QML module keeps the
# applet itself a plain QML KPackage, so there is no plugin loading to arrange
# on Plasma's side.
# ---------------------------------------------------------------------------

add_library(ntfywidget SHARED cpp/bridge.cpp cpp/plugin.cpp)
add_dependencies(ntfywidget ntfy_core_build)

target_include_directories(ntfywidget PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/cpp)

target_link_libraries(ntfywidget
    PRIVATE
        Qt6::Core
        Qt6::DBus
        Qt6::Qml
        ${NTFY_CORE_LIB}
)

set_target_properties(ntfywidget PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/module
    OUTPUT_NAME ntfywidget
)

set(NTFY_MODULE_URI org.ntfy.widget)
string(REPLACE "." "/" NTFY_MODULE_PATH ${NTFY_MODULE_URI})
file(WRITE ${CMAKE_CURRENT_BINARY_DIR}/qmldir
     "module ${NTFY_MODULE_URI}\nplugin ntfywidget\n"
)

# ---------------------------------------------------------------------------
# Stage the pieces: a QML module and a KPackage applet that imports it.
# ---------------------------------------------------------------------------

set(PACKAGE_ID ${NTFY_MODULE_URI})
set(PACKAGE_STAGE ${CMAKE_CURRENT_BINARY_DIR}/${PACKAGE_ID})
set(MODULE_STAGE ${CMAKE_CURRENT_BINARY_DIR}/module/${NTFY_MODULE_PATH})

# Qt resolves QML modules by URI inside its own import directory, so this one
# goes there rather than under the user's data directory.
find_program(QMAKE6_EXECUTABLE qmake6)
if(QMAKE6_EXECUTABLE)
    execute_process(COMMAND ${QMAKE6_EXECUTABLE} -query QT_INSTALL_QML
                    OUTPUT_VARIABLE QT_INSTALL_QML OUTPUT_STRIP_TRAILING_WHITESPACE)
endif()
if(NOT QT_INSTALL_QML)
    set(QT_INSTALL_QML ${QT6_INSTALL_PREFIX}/${QT6_INSTALL_PREFIX}/qml)
endif()
set(MODULE_INSTALL_DIR ${QT_INSTALL_QML}/${NTFY_MODULE_PATH})

file(GLOB_RECURSE PACKAGING_FILES CONFIGURE_DEPENDS
     ${CMAKE_CURRENT_SOURCE_DIR}/packaging/*
)

add_custom_target(ntfy_package ALL
    COMMAND ${CMAKE_COMMAND} -E copy_directory
            ${CMAKE_CURRENT_SOURCE_DIR}/packaging ${PACKAGE_STAGE}
    COMMAND ${CMAKE_COMMAND} -E make_directory ${MODULE_STAGE}
    COMMAND ${CMAKE_COMMAND} -E copy
            $<TARGET_FILE:ntfywidget> ${MODULE_STAGE}/
    COMMAND ${CMAKE_COMMAND} -E copy
            ${CMAKE_CURRENT_BINARY_DIR}/qmldir ${MODULE_STAGE}/qmldir
    DEPENDS ntfywidget ${PACKAGING_FILES}
    COMMENT "Staging the applet package and its QML module"
    VERBATIM
)

# ---------------------------------------------------------------------------
# Install
# ---------------------------------------------------------------------------

install(DIRECTORY ${PACKAGE_STAGE} DESTINATION ${KDE_INSTALL_DATADIR}/plasma/plasmoids)
install(DIRECTORY ${MODULE_STAGE} DESTINATION ${QT_INSTALL_QML}/${NTFY_MODULE_PATH})

find_program(KPACKAGETOOL kpackagetool6)
if(KPACKAGETOOL)
    # Installing the QML module system wide needs root; the applet itself is
    # installed for the current user only.
    add_custom_target(dev-install
        COMMAND ${KPACKAGETOOL} --type Plasma/Applet --upgrade ${PACKAGE_STAGE}
        COMMAND sudo ${CMAKE_COMMAND} -E copy_directory ${MODULE_STAGE} ${MODULE_INSTALL_DIR}
        DEPENDS ntfy_package
        COMMENT "Installing the applet and its QML module"
        VERBATIM
    )
    add_custom_target(dev-uninstall
        COMMAND ${KPACKAGETOOL} --remove ${PACKAGE_ID}
        COMMAND sudo ${CMAKE_COMMAND} -E remove_directory ${QT_INSTALL_QML}/org/ntfy
        COMMENT "Removing the applet and its QML module"
        VERBATIM
    )
endif()

# ---------------------------------------------------------------------------
# Development helpers. Off by default so the widget itself keeps a small
# dependency list.
# ---------------------------------------------------------------------------

option(NTFY_BUILD_TOOLS "Build the development helpers" OFF)

if(NTFY_BUILD_TOOLS)
    find_package(Qt6 REQUIRED COMPONENTS Gui Quick QuickControls2)
    find_package(KF6Package REQUIRED)

    # Reports how Plasma resolves the installed package.
    add_executable(ntfyprobe tools/probe.cpp)
    target_link_libraries(ntfyprobe PRIVATE Qt6::Core Qt6::Qml Qt6::Quick KF6::Package)

    # Shows any QML file in a window, for checking UI changes.
    add_executable(ntfypreview tools/preview.cpp)
    target_link_libraries(ntfypreview PRIVATE
        Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick Qt6::QuickControls2)
endif()

message(STATUS "Applet id: ${PACKAGE_ID}")
message(STATUS "Staged package: ${PACKAGE_STAGE}")
