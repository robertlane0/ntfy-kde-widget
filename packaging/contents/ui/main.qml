/*
    SPDX-License-Identifier: MIT

    The applet root: the panel button (compactRepresentation) and the
    subscription manager (fullRepresentation, shown inside the shell's popup).
*/
import QtQuick

import org.kde.plasma.plasmoid

import org.ntfy.widget

PlasmoidItem {
    id: root

    // Snapshot of the whole engine state, refreshed on every core event.
    property var model: ({ subscriptions: [], unread: 0 })

    readonly property int totalUnread: {
        let sum = 0
        const subs = model.subscriptions || []
        for (let i = 0; i < subs.length; ++i) {
            sum += subs[i].unread || 0
        }
        return sum
    }

    function topicCount() {
        return (model.subscriptions || []).length
    }

    toolTipMainText: totalUnread > 0
        ? i18np("%1 message waiting", "%1 messages waiting", totalUnread)
        : i18np("%1 topic, all caught up", "%1 topics, all caught up", topicCount())
    toolTipSubText: i18n("ntfy")

    Connections {
        target: Bridge
        function onStateChanged() {
            root.model = JSON.parse(Bridge.state() || "{}")
        }
    }

    Component.onCompleted: root.model = JSON.parse(Bridge.state() || "{}")

    // A bare PlasmoidItem, unlike a CompactApplet, gets no popup toggle from
    // the shell, so the panel button has to ask for one. The toggle has to
    // happen on this root: the Plasmoid attached type does not resolve to the
    // applet from inside a separate component file.
    compactRepresentation: PanelIcon {
        unread: root.totalUnread
        onActivate: root.expanded = !root.expanded
    }

    fullRepresentation: SubscriptionManager {
        model: root.model
    }
}