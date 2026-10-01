/*
    SPDX-License-Identifier: MIT

    Inline icon set drawn from a 16x16 design grid, so the widget does not
    depend on a particular icon theme being installed.
*/
import QtQuick
import QtQuick.Shapes as Shapes

Item {
    id: root

    property color color: Theme.text
    property real weight: 1.6
    property string name: "bell"

    implicitWidth: 16
    implicitHeight: 16

    readonly property real s: width / 16
    readonly property bool slashed: name.endsWith(".slash") || name.endsWith(".slash.fill")
    readonly property bool filled: name.endsWith("fill")
    readonly property bool isBell: name.startsWith("bell")
    readonly property bool isTrash: name.startsWith("trash")
    readonly property bool isPlus: name === "plus"

    // ---------------------------------------------------------------- bell

    Shapes.Shape {
        visible: root.isBell
        anchors.fill: parent
        preferredRendererType: Shapes.Shape.GeometryRenderer
        layer.enabled: true
        layer.samples: 4

        Shapes.ShapePath {
            strokeColor: root.color
            // A filled bell reads as a blob at panel sizes, so the stroke
            // shrinks to a hairline whenever the body is filled.
            strokeWidth: (root.filled ? 0.9 : root.weight) * root.s
            fillColor: root.filled ? root.color : "transparent"
            capStyle: Shapes.ShapePath.RoundCap
            joinStyle: Shapes.ShapePath.RoundJoin

            // Dome with a flared skirt.
            PathSvg {
                path: "M 4.4 6.6 A 3.6 3.6 0 0 1 11.6 6.6 V 9.2 L 12.8 11.1 H 3.2 L 4.4 9.2 Z"
            }
        }

        Shapes.ShapePath {
            strokeColor: root.color
            strokeWidth: root.weight * root.s
            fillColor: "transparent"
            capStyle: Shapes.ShapePath.RoundCap

            // Clapper, then the stub on top.
            PathSvg {
                path: "M 6.5 13.1 A 1.5 1.5 0 0 0 9.5 13.1 M 8 2 V 2.5"
            }
        }
    }

    // --------------------------------------------------------------- trash

    Shapes.Shape {
        visible: root.isTrash
        anchors.fill: parent
        preferredRendererType: Shapes.Shape.GeometryRenderer
        layer.enabled: true
        layer.samples: 4

        Shapes.ShapePath {
            strokeColor: root.color
            strokeWidth: root.weight * root.s
            fillColor: "transparent"
            capStyle: Shapes.ShapePath.RoundCap
            joinStyle: Shapes.ShapePath.RoundJoin

            PathSvg {
                path: "M 3.2 4.6 H 12.8 M 6.4 4.6 V 3.3 H 9.6 V 4.6 M 4.4 4.6 L 5 13 H 11 L 11.6 4.6"
            }
        }
    }

    // ---------------------------------------------------------------- plus

    Shapes.Shape {
        visible: root.isPlus
        anchors.fill: parent
        preferredRendererType: Shapes.Shape.GeometryRenderer
        layer.enabled: true
        layer.samples: 4

        Shapes.ShapePath {
            strokeColor: root.color
            strokeWidth: root.weight * root.s
            capStyle: Shapes.ShapePath.RoundCap

            PathSvg {
                path: "M 8 3.4 V 12.6 M 3.4 8 H 12.6"
            }
        }
    }

    // --------------------------------------------------------------- slash

    Shapes.Shape {
        visible: root.slashed
        anchors.fill: parent
        preferredRendererType: Shapes.Shape.GeometryRenderer
        layer.enabled: true
        layer.samples: 4

        // Masked with the row colour so it knocks out the bell underneath.
        Rectangle {
            anchors.centerIn: parent
            width: 15.5 * root.s
            height: root.weight * root.s + 1.5
            rotation: -45
            color: root.name === "bell.slash" ? Theme.surface : Theme.surfaceHover
            radius: height / 2
        }

        Shapes.ShapePath {
            strokeColor: root.color
            strokeWidth: root.weight * root.s
            capStyle: Shapes.ShapePath.RoundCap

            PathSvg {
                path: "M 2.7 13.3 L 13.3 2.7"
            }
        }
    }
}