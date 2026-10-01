/*
    SPDX-License-Identifier: MIT

    Labelled text input with a macOS style inset field.
*/
import QtQuick
import QtQuick.Layouts

import org.ntfy.widget

ColumnLayout {
    id: root

    property alias text: input.text
    property string label: ""
    property string placeholder: ""
    property alias input: input
    property int inputMethodHints: Qt.ImhNone

    signal accepted()

    spacing: 4

    Text {
        visible: root.label.length > 0
        text: root.label
        color: Theme.textSecondary
        font.family: Theme.fontFamily
        font.pixelSize: Theme.fontCaption
        font.weight: Font.Medium
    }

    Rectangle {
        Layout.fillWidth: true
        implicitHeight: 32
        radius: Theme.radiusControl
        color: input.activeFocus ? Theme.surface : Theme.surfaceHover
        border.width: 1
        border.color: input.activeFocus ? Theme.accent : Theme.border

        Behavior on color {
            ColorAnimation { duration: Theme.durationFast }
        }

        Behavior on border.color {
            ColorAnimation { duration: Theme.durationFast }
        }

        TextInput {
            id: input

            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 10
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.text
            font.family: Theme.fontFamily
            font.pixelSize: Theme.fontBody
            selectByMouse: true
            selectionColor: Theme.accent
            selectedTextColor: "#ffffff"
            clip: true
            inputMethodHints: root.inputMethodHints

            onAccepted: root.accepted()

            Text {
                anchors.fill: parent
                verticalAlignment: Text.AlignVCenter
                visible: input.text.length === 0
                text: root.placeholder
                color: Theme.textTertiary
                font: input.font
            }
        }
    }
}