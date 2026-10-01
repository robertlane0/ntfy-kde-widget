/*
    SPDX-License-Identifier: MIT

    Shared design tokens. Everything visual reads from here so the panel button
    and the popup stay in step.
*/
pragma Singleton

import QtQuick

QtObject {
    id: root

    readonly property bool dark: {
        const scheme = Qt.styleHints.colorScheme
        if (scheme === Qt.Dark) {
            return true
        }
        if (scheme === Qt.Light) {
            return false
        }
        // ColorSchemeUnknown: fall back to the brightness of the window colour.
        return Qt.application.palette.window().color().lightness() < 128
    }

    // Accent used for the add button, unread pills and the active dot.
    readonly property color accent: dark ? Qt.rgba(0.42, 0.72, 1.0, 1.0) : Qt.rgba(0.0, 0.478, 1.0, 1.0)

    readonly property color accentSoft: Qt.rgba(accent.r, accent.g, accent.b, dark ? 0.22 : 0.14)

    // Text ramp, tuned for legibility on both schemes.
    readonly property color text: dark ? "#f2f3f5" : "#1d1d1f"
    readonly property color textSecondary: dark ? "#9a9ba0" : "#6b6b70"
    readonly property color textTertiary: dark ? "#6d6e74" : "#9a9aa0"

    // Surfaces: the popup itself, then the raised cards that sit on top of it.
    readonly property color surface: dark ? "#1f2023" : "#fbfbfd"
    readonly property color surfaceRaised: dark ? "#2a2b2f" : "#ffffff"
    readonly property color surfaceHover: dark ? Qt.rgba(1, 1, 1, 0.07) : Qt.rgba(0, 0, 0, 0.045)
    readonly property color surfaceActive: dark ? Qt.rgba(1, 1, 1, 0.12) : Qt.rgba(0, 0, 0, 0.08)

    readonly property color separator: dark ? Qt.rgba(1, 1, 1, 0.09) : Qt.rgba(0, 0, 0, 0.09)
    readonly property color border: dark ? Qt.rgba(1, 1, 1, 0.12) : Qt.rgba(0, 0, 0, 0.11)

    // Status colours.
    readonly property color live: dark ? "#32d74b" : "#28c840"
    readonly property color pending: dark ? "#ffd60a" : "#f5a623"
    readonly property color error: dark ? "#ff6961" : "#e5484d"
    readonly property color muted: dark ? "#7d7e84" : "#a0a0a6"

    // Radii: macOS uses generously rounded corners on almost everything.
    readonly property int radiusCard: 12
    readonly property int radiusRow: 9
    readonly property int radiusControl: 7
    readonly property int radiusPill: 999

    readonly property int unit: 8
    readonly property int padding: 14
    readonly property int rowHeight: 46

    // Type ramp.
    readonly property string fontFamily: Qt.fontFamilies().indexOf("SF Pro Text") >= 0
        ? "SF Pro Text"
        : (Qt.fontFamilies().indexOf("Noto Sans") >= 0 ? "Noto Sans" : Qt.application.font.family)

    readonly property int fontTitle: 13
    readonly property int fontBody: 12
    readonly property int fontCaption: 10
    readonly property int fontBadge: 9

    readonly property int durationFast: 110
    readonly property int durationNormal: 190

    /// Maps a subscription's state onto a colour.
    function stateColor(state) {
        switch (state) {
        case "live":
            return live
        case "connecting":
        case "retrying":
            return pending
        case "failed":
            return error
        default:
            return muted
        }
    }

    /// Human readable state text for the status line under each topic.
    function stateLabel(state) {
        switch (state) {
        case "live":
            return "Live"
        case "connecting":
            return "Connecting…"
        case "retrying":
            return "Reconnecting…"
        case "failed":
            return "Not connected"
        default:
            return "Muted"
        }
    }
}