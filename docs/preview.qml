import QtQuick
import QtQuick.Controls

import "../packaging/contents/ui" as Ui
import org.ntfy.widget

ApplicationWindow {
    width: 660
    height: 520
    visible: true
    color: Ui.Theme.surface

    Column {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 16

        Row {
            spacing: 18
            Repeater {
                model: ["bell", "bell.fill", "bell.slash", "plus", "trash"]
                delegate: Column {
                    spacing: 4
                    Ui.Icon {
                        width: 40
                        height: 40
                        name: modelData
                        color: Ui.Theme.text
                    }
                    Text { text: modelData; font.pixelSize: 10; color: Ui.Theme.textTertiary }
                }
            }
        }

        Rectangle { height: 1; width: parent.width; color: Ui.Theme.separator }

        // The rows as they actually appear, over the popup surface.
        Ui.SubscriptionRow {
            width: parent.width
            subscription: ({ topic: "kde-widget-demo", server: "https://ntfy.sh", state: "live", unread: 3 })
        }
        Ui.SubscriptionRow {
            width: parent.width
            subscription: ({ topic: "release-watch", server: "https://ntfy.sh", state: "muted", unread: 0 })
        }

        Rectangle { height: 1; width: parent.width; color: Ui.Theme.separator }

        Row {
            spacing: 12
            Ui.PillButton { text: "Add Subscription"; iconName: "plus"; primary: true }
            Ui.PillButton { text: "Mark All Read" }
            Ui.PillButton { text: "Subscribe"; primary: true }
            Ui.PillButton { text: "Subscribe"; primary: true; enabled: false }
            Ui.PillButton { text: "Cancel" }
        }
    }
}