/*
    SPDX-License-Identifier: MIT

    Small square icon button used for the per-row actions. Fades in on hover so
    the list stays calm until you point at it.
*/
import QtQuick
import QtQuick.Controls

import org.ntfy.widget

Button {
    id: root

    property string name: "trash"
    property color iconColor: Theme.textTertiary
    property string tooltip: ""

    implicitWidth: 24
    implicitHeight: 24
    hoverEnabled: true
    padding: 0

    background: Rectangle {
        radius: Theme.radiusControl
        color: root.down ? Theme.surfaceActive : (root.hovered ? Theme.surfaceHover : "transparent")

        Behavior on color {
            ColorAnimation { duration: Theme.durationFast }
        }
    }

    contentItem: Icon {
        width: 14
        height: 14
        name: root.name
        color: root.iconColor
        opacity: root.hovered || root.down || root.name.endsWith("fill") ? 1 : 0.62

        Behavior on opacity {
            NumberAnimation { duration: Theme.durationFast }
        }
    }

    ToolTip.visible: root.tooltip.length > 0 && root.hovered
    ToolTip.delay: 600
    ToolTip.text: root.tooltip
}