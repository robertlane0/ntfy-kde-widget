/*
    SPDX-License-Identifier: MIT

    One subscription, laid out like a macOS sidebar row: status dot, topic name
    and host on the left, unread pill and actions on the right.
*/
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import org.ntfy.widget

Item {
    id: root

    property var subscription: ({})
    property bool confirmRemove: false

    readonly property bool muted: subscription.state === "muted"
    readonly property color dotColor: Theme.stateColor(subscription.state || "muted")
    readonly property int unread: subscription.unread || 0

    implicitHeight: row.implicitHeight + 10

    // Hover highlight, inset like a macOS sidebar selection.
    Rectangle {
        anchors.fill: parent
        anchors.margins: 1
        radius: Theme.radiusRow
        color: {
            if (confirmRemove) {
                return Theme.error
            }
            if (hover.hovered) {
                return Theme.surfaceHover
            }
            return "transparent"
        }

        Behavior on color {
            ColorAnimation { duration: Theme.durationFast }
        }
    }

    Rectangle {
        anchors.fill: parent
        anchors.margins: 1
        radius: Theme.radiusRow
        color: "transparent"
        border.width: root.unread > 0 && !root.muted ? 1 : 0
        border.color: Theme.accentSoft
    }

    HoverHandler { id: hover }

    // Clicking anywhere on the row clears its unread count.
    TapHandler {
        enabled: root.unread > 0
        onTapped: Bridge.markRead(root.subscription.id)
    }

    RowLayout {
        id: row

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: Theme.padding
        anchors.rightMargin: Theme.padding - 2
        spacing: 9

        // Status dot, breathing gently while live.
        Item {
            Layout.preferredWidth: 9
            Layout.preferredHeight: 9

            Rectangle {
                anchors.centerIn: parent
                width: 7
                height: 7
                radius: 3.5
                color: root.dotColor
                opacity: root.muted ? 0.5 : 1
            }

            SequentialAnimation on scale {
                running: root.subscription.state === "live"
                loops: Animation.Infinite
                alwaysRunToEnd: true
                NumberAnimation { to: 1.5; duration: 1500; easing.type: Easing.InOutSine }
                NumberAnimation { to: 1.0; duration: 1500; easing.type: Easing.InOutSine }
            }

            Rectangle {
                anchors.centerIn: parent
                width: 7
                height: 7
                radius: 3.5
                color: "transparent"
                border.width: 1
                border.color: root.dotColor
                opacity: 0.45
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1

            RowLayout {
                Layout.fillWidth: true
                spacing: 5

                Text {
                    text: root.subscription.topic || ""
                    color: root.muted ? Theme.textTertiary : Theme.text
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontBody
                    font.weight: Font.DemiBold
                    elide: Text.ElideMiddle
                    Layout.maximumWidth: 200
                }

                Rectangle {
                    visible: root.subscription.secured === true
                    Layout.preferredWidth: lockRow.implicitWidth + 8
                    Layout.preferredHeight: 13
                    radius: 3
                    color: Theme.surfaceHover

                    RowLayout {
                        id: lockRow
                        anchors.centerIn: parent
                        spacing: 2

                        Text {
                            text: "🔒"
                            font.pixelSize: 8
                        }
                    }
                }
            }

            Text {
                Layout.fillWidth: true
                text: root.subscription.state === "failed" && root.subscription.detail
                    ? root.subscription.detail
                    : hostOf(root.subscription.server)
                color: root.subscription.state === "failed" ? Theme.error : Theme.textTertiary
                font.family: Theme.fontFamily
                font.pixelSize: Theme.fontCaption
                elide: Text.ElideRight
            }
        }

        // Unread pill.
        Rectangle {
            visible: root.unread > 0
            Layout.preferredWidth: Math.max(unreadText.implicitWidth + 11, 20)
            Layout.preferredHeight: 19
            radius: Theme.radiusPill
            color: root.muted ? Theme.muted : Theme.accent

            Text {
                id: unreadText
                anchors.centerIn: parent
                text: root.unread > 99 ? "99+" : root.unread
                color: "#ffffff"
                font.family: Theme.fontFamily
                font.pixelSize: Theme.fontBadge + 1
                font.weight: Font.DemiBold
            }
        }

        // Actions. Shown on hover, or always when there is a confirmation.
        RowLayout {
            spacing: 2

            ActionButton {
                name: root.muted ? "bell.slash" : "bell.slash.fill"
                iconColor: hover.hovered ? Theme.text : Theme.textTertiary
                tooltip: root.muted ? i18n("Unmute") : i18n("Mute")
                onClicked: Bridge.setEnabled(root.subscription.id, root.muted)
            }

            ActionButton {
                name: "trash"
                iconColor: root.confirmRemove ? Theme.error : (hover.hovered ? Theme.error : Theme.textTertiary)
                tooltip: root.confirmRemove ? i18n("Click again to delete") : i18n("Remove")
                onClicked: {
                    if (root.confirmRemove) {
                        Bridge.removeSubscription(root.subscription.id)
                        root.confirmRemove = false
                    } else {
                        root.confirmRemove = true
                        confirmReset.restart()
                    }
                }
            }

            Timer {
                id: confirmReset
                interval: 2600
                onTriggered: root.confirmRemove = false
            }
        }
    }

    function hostOf(server) {
        if (!server) {
            return ""
        }
        return server.replace(/^https?:\/\//, "").replace(/\/$/, "")
    }
}