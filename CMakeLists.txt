cmake_minimum_required(VERSION 3.22)

project(ntfy-kde-widget
    VERSION 0.1.0
    DESCRIPTION "Plasma 6 panel widget that raises ntfy notifications"
    LANGUAGES CXX
)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_INCLUDE_CURRENT_DIR ON)

find_package(Qt6 6.6 REQUIRED COMPONENTS Core Gui Qml Quick DBus)
find_package(KF6Config REQUIRED)
find_package(KF6CoreAddons REQUIRED)
find_package(KF6KirigamiPlatform REQUIRED)
find_package(Plasma 6.0 REQUIRED)

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
# QML extension plugin. It exports the Bridge singleton, which is the only
# thing the applet's QML needs from C++. Shipping it as a normal QML module
# keeps the applet itself a plain QML KPackage, so there is no plugin loading
# to arrange on Plasma's side.
# ---------------------------------------------------------------------------

add_library(ntfywidget SHARED cpp/bridge.cpp cpp/plugin.cpp)
add_dependencies(ntfywidget ntfy_core_build)

set(NTFY_MODULE_URI org.ntfy.widget)
file(WRITE ${CMAKE_CURRENT_BINARY_DIR}/qmldir
     "module ${NTFY_MODULE_URI}\nplugin ntfywidget\n"
)

target_include_directories(ntfywidget PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/cpp)

target_link_libraries(ntfywidget
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Qml
        Qt6::Quick
        Qt6::DBus
        KF6::CoreAddons
        ${NTFY_CORE_LIB}
)

set_target_properties(ntfywidget PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/module
    OUTPUT_NAME ntfywidget
)

# ---------------------------------------------------------------------------
# Assemble the pieces: a QML module and a KPackage applet that imports it.
# ---------------------------------------------------------------------------

set(PACKAGE_ID org.ntfy.widget)
set(PACKAGE_STAGE ${CMAKE_CURRENT_BINARY_DIR}/${PACKAGE_ID})
set(MODULE_STAGE ${CMAKE_CURRENT_BINARY_DIR}/module/org/ntfy/widget)

# Qt only looks in its own import directory for QML modules, so this one has to
# be staged there rather than under the user's data directory.
find_program(QMAKE6_EXECUTABLE qmake6)
if(QMAKE6_EXECUTABLE)
    execute_process(COMMAND ${QMAKE6_EXECUTABLE} -query QT_INSTALL_QML
                    OUTPUT_VARIABLE QT_INSTALL_QML OUTPUT_STRIP_TRAILING_WHITESPACE)
endif()
if(NOT QT_INSTALL_QML)
    set(QT_INSTALL_QML ${QT6_INSTALL_PREFIX}/${QT6_INSTALL_PREFIX}/qml)
endif()
set(NTFY_MODULE_INSTALL_DIR ${QT_INSTALL_QML}/org/ntfy/widget)

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
# Install targets
# ---------------------------------------------------------------------------

find_program(KPACKAGETOOL kpackagetool6)
if(KPACKAGETOOL)
    add_custom_target(dev-install
        COMMAND ${KPACKAGETOOL} --type Plasma/Applet --upgrade ${PACKAGE_STAGE}
        COMMAND sudo ${CMAKE_COMMAND} -E copy_directory
                ${MODULE_STAGE} ${NTFY_MODULE_INSTALL_DIR}
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

install(DIRECTORY ${PACKAGE_STAGE}
        DESTINATION ${KDE_INSTALL_DATADIR}/plasma/plasmoids)
install(DIRECTORY ${MODULE_STAGE}
        DESTINATION ${QT_INSTALL_QML}/org/ntfy)

message(STATUS "Applet id: ${PACKAGE_ID}")
message(STATUS "Staged package: ${PACKAGE_STAGE}")
# ---------------------------------------------------------------------------
# Development helper: probe how Plasma resolves the installed package.
# ---------------------------------------------------------------------------

add_executable(ntfyprobe tools/probe.cpp)
target_link_libraries(ntfyprobe PRIVATE Qt6::Core Qt6::Qml Qt6::Quick KF6::Package)
