/*
    SPDX-License-Identifier: MIT

    Floating card for creating a subscription. Fields, inline validation and a
    primary action, in the same visual language as the rest of the panel.
*/
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import org.ntfy.widget

Rectangle {
    id: root

    signal accepted()

    property string error: ""

    implicitHeight: content.implicitHeight + 2 * Theme.padding
    radius: Theme.radiusCard
    color: Theme.surfaceRaised
    border.width: 1
    border.color: error.length > 0 ? Qt.rgba(Theme.error.r, Theme.error.g, Theme.error.b, 0.5) : Theme.border

    // Soft shadow so the sheet reads as floating above the list.
    Rectangle {
        anchors.fill: parent
        anchors.margins: -1
        radius: parent.radius + 1
        color: "transparent"
        border.width: 1
        border.color: Theme.dark ? Qt.rgba(0, 0, 0, 0.35) : Qt.rgba(0, 0, 0, 0.10)
        z: -1
    }

    ColumnLayout {
        id: content

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Theme.padding
        spacing: Theme.unit

        Text {
            Layout.fillWidth: true
            text: i18n("New subscription")
            color: Theme.text
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontTitle
            font.weight: Font.DemiBold
        }

        Field {
            id: serverField

            Layout.fillWidth: true
            label: i18n("Server")
            placeholder: "ntfy.sh"
            text: "ntfy.sh"
            inputMethodHints: Qt.ImhUrlCharactersOnly
        }

        Field {
            id: topicField

            Layout.fillWidth: true
            label: i18n("Topic")
            placeholder: "my-alerts"
            onAccepted: submit.clicked()

            // Topics are restricted by the protocol, so catch it before the core does.
            onTextChanged: {
                const trimmed = text.trim()
                if (trimmed.length > 0 && !/^[A-Za-z0-9._-]+$/.test(trimmed)) {
                    root.error = i18n("Topics use letters, digits, dots, dashes and underscores")
                } else if (root.error.length > 0) {
                    root.error = ""
                }
            }
        }

        Text {
            visible: root.error.length > 0
            Layout.fillWidth: true
            Layout.topMargin: -2
            text: root.error
            color: Theme.error
            wrapMode: Text.WordWrap
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontCaption
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: 2
            spacing: Theme.unit

            Item { Layout.fillWidth: true }

            Button {
                id: cancelButton

                text: i18n("Cancel")
                hoverEnabled: true
                implicitHeight: 30

                background: Rectangle {
                    radius: Theme.radiusControl
                    color: cancelButton.down ? Theme.surfaceActive
                                             : (cancelButton.hovered ? Theme.surfaceHover : "transparent")
                    Behavior on color { ColorAnimation { duration: Theme.durationFast } }
                }

                contentItem: Text {
                    text: cancelButton.text
                    color: Theme.textSecondary
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontBody
                    verticalAlignment: Text.AlignVCenter
                }

                onClicked: {
                    topicField.clear()
                    serverField.text = "ntfy.sh"
                    root.error = ""
                    root.accepted()
                }
            }

            Button {
                id: submit

                objectName: "addSubscriptionConfirm"
                text: i18n("Subscribe")
                hoverEnabled: true
                enabled: topicField.text.trim().length > 0
                implicitHeight: 30
                implicitWidth: submitLabel.implicitWidth + 26

                background: Rectangle {
                    radius: Theme.radiusControl
                    color: !submit.enabled
                        ? Qt.rgba(Theme.textTertiary.r, Theme.textTertiary.g, Theme.textTertiary.b, 0.35)
                        : (submit.down ? Qt.darker(Theme.accent, 1.14)
                                       : (submit.hovered ? Qt.lighter(Theme.accent, 1.08) : Theme.accent))
                    Behavior on color { ColorAnimation { duration: Theme.durationFast } }
                }

                contentItem: Text {
                    id: submitLabel
                    text: submit.text
                    color: "#ffffff"
                    font.family: Theme.fontFamily
                    font.pixelSize: Theme.fontBody
                    font.weight: Font.DemiBold
                    verticalAlignment: Text.AlignVCenter
                }

                onClicked: {
                    const topic = topicField.text.trim()
                    if (topic.length === 0) {
                        return
                    }
                    const problem = Bridge.addSubscription(serverField.text.trim(), topic)
                    if (problem.length > 0) {
                        root.error = problem
                        return
                    }
                    topicField.clear()
                    root.error = ""
                    root.accepted()
                }
            }
        }
    }

    // Start with the topic field focused so the common case is one keystroke away.
    Component.onCompleted: topicField.forceActiveFocus()
}