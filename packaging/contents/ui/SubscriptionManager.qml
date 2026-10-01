/*
    SPDX-License-Identifier: MIT

    The popup body: a macOS style panel with a header, the subscription list
    and a footer that opens the add-subscription sheet.
*/
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import org.ntfy.widget

Item {
    id: root

    property var model: ({ subscriptions: [], recent: [], unread: 0 })
    property bool adding: false

    readonly property var subs: model.subscriptions || []
    readonly property int totalUnread: model.unread || 0

    Layout.minimumWidth: 380
    Layout.preferredWidth: 380
    Layout.minimumHeight: 320
    Layout.maximumWidth: 460

    function countUnread() {
        let sum = 0
        for (let i = 0; i < subs.length; ++i) {
            sum += subs[i].unread || 0
        }
        return sum
    }

    // The sheet floats over the list, so it lives outside the column layout.
    ColumnLayout {
        id: column

        anchors.fill: parent
        spacing: 0

    // ---------------------------------------------------------------- header

    Item {
        id: header

        Layout.fillWidth: true
        Layout.preferredHeight: headerCol.implicitHeight + 2 * Theme.padding

        ColumnLayout {
            id: headerCol
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: Theme.padding
            anchors.rightMargin: Theme.padding
            spacing: 3

            RowLayout {
                Layout.fillWidth: true
                spacing: 7

                Icon {
                    Layout.preferredWidth: 17
                    Layout.preferredHeight: 17
                    name: "bell.fill"
                    color: Theme.accent
                }

                Text {
                    text: i18n("ntfy")
                    color: Theme.text
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontTitle + 1
                    font.weight: Font.DemiBold
                }

                Item { Layout.fillWidth: true }

                Text {
                    visible: root.countUnread() > 0
                    text: i18np("%1 unread", "%1 unread", root.countUnread())
                    color: Theme.accent
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontCaption
                    font.weight: Font.DemiBold
                }
            }

            Text {
                Layout.fillWidth: true
                text: root.subs.length === 0
                    ? i18n("No topics yet")
                    : i18np("%1 topic", "%1 topics", root.subs.length)
                color: Theme.textSecondary
                font.family: Theme.fontFamily
                font.pixelSize: Theme.fontCaption
                elide: Text.ElideRight
            }
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.separator
        }
    }

    // ------------------------------------------------------------------ list

    Flickable {
        id: list

        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 120
        Layout.topMargin: Theme.unit
        contentWidth: width
        contentHeight: listColumn.implicitHeight + Theme.unit
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickDeceleration: 4000
        flickableDirection: Flickable.VerticalFlick

        ScrollBar.vertical: ScrollBar {
            policy: list.contentHeight > list.height ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            width: 6
        }

        ColumnLayout {
            id: listColumn
            width: list.width
            spacing: 2

            // Empty state.
            ColumnLayout {
                visible: root.subs.length === 0
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumHeight: 150
                Layout.topMargin: 20
                Layout.bottomMargin: 20
                spacing: Theme.unit

                Icon {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.preferredWidth: 32
                    Layout.preferredHeight: 32
                    name: "bell"
                    color: Theme.textTertiary
                    opacity: 0.5
                }

                Text {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.fillWidth: true
                    Layout.leftMargin: 24
                    Layout.rightMargin: 24
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                    text: i18n("Nothing subscribed yet")
                    color: Theme.text
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontBody + 1
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.fillWidth: true
                    Layout.leftMargin: 30
                    Layout.rightMargin: 30
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                    text: i18n("Add a topic and every message it receives will pop up as a notification.")
                    color: Theme.textSecondary
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontBody
                }
            }

            Repeater {
                model: root.subs

                delegate: SubscriptionRow {
                    required property var modelData

                    width: listColumn.width
                    subscription: modelData
                }
            }
        }
    }

    // ---------------------------------------------------------------- footer

    Item {
        id: footer

        Layout.fillWidth: true
        Layout.preferredHeight: 52

        Rectangle {
            anchors.top: parent.top
            width: parent.width
            height: 1
            color: Theme.separator
        }

        Button {
            id: addButton

            anchors.left: parent.left
            anchors.leftMargin: Theme.padding
            anchors.verticalCenter: parent.verticalCenter

            implicitHeight: 30
            implicitWidth: addRow.implicitWidth + 26
            hoverEnabled: true

            background: Rectangle {
                radius: Theme.radiusControl
                color: addButton.down ? Qt.darker(Theme.accent, 1.14)
                                      : (addButton.hovered ? Qt.lighter(Theme.accent, 1.08) : Theme.accent)
                Behavior on color { ColorAnimation { duration: Theme.durationFast } }
            }

            contentItem: RowLayout {
                id: addRow
                spacing: 6

                Icon {
                    Layout.preferredWidth: 12
                    Layout.preferredHeight: 12
                    name: "bell.fill"
                    color: "#ffffff"
                }

                Text {
                    text: i18n("Add Subscription")
                    color: "#ffffff"
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontBody
                    font.weight: Font.DemiBold
                }
            }

            onClicked: root.adding = !root.adding
        }

        Button {
            id: markAllButton

            anchors.right: parent.right
            anchors.rightMargin: Theme.padding
            anchors.verticalCenter: parent.verticalCenter
            visible: root.subs.length > 0
            text: i18n("Mark All Read")
            hoverEnabled: true

            implicitHeight: 28
            implicitWidth: markLabel.implicitWidth + 18

            background: Rectangle {
                radius: Theme.radiusControl
                color: markAllButton.hovered ? Theme.surfaceHover : "transparent"
                Behavior on color { ColorAnimation { duration: Theme.durationFast } }
            }

            contentItem: Text {
                id: markLabel
                text: markAllButton.text
                color: Theme.textSecondary
                font.family: Theme.fontFamily
                font.pixelSize: Theme.fontBody
            }

            onClicked: Bridge.markAllRead()
        }
    }

    }

    // The add form sits above the footer like a sheet.
    AddSubscriptionSheet {
        id: sheet

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.bottomMargin: footer.height
        visible: opacity > 0
        opacity: root.adding ? 1 : 0
        enabled: opacity > 0.5

        onAccepted: root.adding = false

        Behavior on opacity {
            NumberAnimation { duration: Theme.durationNormal; easing.type: Easing.OutCubic }
        }
    }
}