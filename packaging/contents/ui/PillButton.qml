/*
    SPDX-License-Identifier: MIT

    Pill button with an optional icon. Every labelled action uses this, which
    keeps the glyph and the label centred together whatever the button width
    ends up being.
*/
import QtQuick
import QtQuick.Controls

import org.ntfy.widget

Button {
    id: root

    property string iconName: ""
    property bool primary: false

    implicitHeight: 30
    padding: 0

    background: Rectangle {
        radius: Theme.radiusControl
        color: {
            if (root.primary) {
                if (!root.enabled) {
                    return Qt.rgba(Theme.textTertiary.r, Theme.textTertiary.g, Theme.textTertiary.b, 0.4)
                }
                if (root.down) {
                    return Qt.darker(Theme.accent, 1.14)
                }
                return root.hovered ? Qt.lighter(Theme.accent, 1.08) : Theme.accent
            }
            if (root.down) {
                return Theme.surfaceActive
            }
            return root.hovered ? Theme.surfaceHover : "transparent"
        }
        border.width: root.primary ? 0 : 1
        border.color: Theme.border

        Behavior on color {
            ColorAnimation { duration: Theme.durationFast }
        }
    }

    // A Button stretches its contentItem across the whole content area, so the
    // row needs an Item to centre itself in.
    contentItem: Item {
        Row {
            id: row

            anchors.centerIn: parent
            spacing: root.iconName.length > 0 ? 6 : 0

            Icon {
                implicitWidth: root.iconName.length > 0 ? 13 : 0
                implicitHeight: implicitWidth
                name: root.iconName
                color: root.primary ? "#ffffff" : Theme.textSecondary
            }

            Text {
                text: root.text
                color: root.primary ? "#ffffff" : Theme.textSecondary
                font.family: Theme.fontFamily
                font.pixelSize: Theme.fontBody
                font.weight: root.primary ? Font.DemiBold : Font.Normal
            }
        }
    }

    implicitWidth: row.implicitWidth + (root.primary ? 26 : 20)
}