/*
    SPDX-License-Identifier: MIT

    The panel button. Shows the ntfy bell plus a macOS style unread badge.
*/
import QtQuick

import org.kde.plasma.plasmoid

Item {
    id: root

    property int unread: 0

    signal activate()

    TapHandler {
        onTapped: root.activate()
    }

    onUnreadChanged: {
        if (unread > 0 && unread === 1) {
            nudge.restart()
        }
    }

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width, parent.height) - 4
        height: width
        radius: Theme.radiusRow
        color: unread > 0 ? Theme.accentSoft : "transparent"

        Behavior on color {
            ColorAnimation { duration: Theme.durationNormal }
        }
    }

    Icon {
        id: bell

        anchors.centerIn: parent
        width: Math.min(18, parent.height - 10)
        height: width
        name: root.unread > 0 ? "bell.fill" : "bell"
        color: root.unread > 0 ? Theme.accent : Theme.textSecondary

        Behavior on color {
            ColorAnimation { duration: Theme.durationNormal }
        }

        // A single friendly nudge when the count first goes up.
        SequentialAnimation {
            id: nudge

            running: false
            alwaysRunToEnd: true

            NumberAnimation { target: bell; property: "scale"; to: 1.16; duration: 140; easing.type: Easing.OutCubic }
            NumberAnimation { target: bell; property: "scale"; to: 1.0; duration: 200; easing.type: Easing.InOutSine }
        }
    }

    Item {
        id: badge

        property int count: root.unread

        visible: count > 0
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.rightMargin: 2
        anchors.topMargin: 2

        width: Math.max(label.implicitWidth + 9, 15)
        height: 15

        Rectangle {
            anchors.fill: parent
            radius: height / 2
            color: Theme.accent
            // The ring separates the badge from the panel behind it.
            border.width: 1.5
            border.color: Theme.surface
        }

        Text {
            id: label
            anchors.centerIn: parent
            text: badge.count > 99 ? "99+" : badge.count
            color: "#ffffff"
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontBadge
            font.weight: Font.DemiBold
        }

        SequentialAnimation on scale {
            running: badge.visible
            alwaysRunToEnd: true
            NumberAnimation { to: 1.12; duration: 130; easing.type: Easing.OutCubic }
            NumberAnimation { to: 1.0; duration: 190; easing.type: Easing.InOutSine }
        }
    }
}