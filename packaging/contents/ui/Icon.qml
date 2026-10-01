/*
    SPDX-License-Identifier: MIT

    Inline icon set drawn from a 16x16 design grid, so the widget does not
    depend on a particular icon theme being installed.

    Names: bell, bell.fill, bell.slash, plus, trash.
    `bell.slash` is an outline bell with the slash knocked out of it, drawn with a
    heavier stroke than the plain outline so the cut does not erase it.
*/
import QtQuick
import QtQuick.Shapes as Shapes

Item {
    id: root

    property color color: Theme.text
    property real weight: 1.6
    property string name: "bell"

    /// Colours stacked behind the slash so it knocks out whatever is
    /// underneath: the first entry is the surface, the rest are overlays.
    property var slashLayers: [Theme.surface]

    implicitWidth: 16
    implicitHeight: 16

    readonly property real s: width / 16
    readonly property bool isBell: name.startsWith("bell")
    readonly property bool isTrash: name.startsWith("trash")
    readonly property bool isPlus: name === "plus"
    readonly property bool slashed: isBell && name.endsWith(".slash")
    readonly property bool filled: name.endsWith("fill")

    /// Stroke width on the 16 unit grid. A filled bell keeps only a hairline,
    /// otherwise the body turns into a blob at panel sizes.
    readonly property real strokeWidth: filled ? 0.9 : weight

    // ---------------------------------------------------------------- bell

    Shapes.Shape {
        visible: root.isBell && !root.slashed
        anchors.fill: parent
        preferredRendererType: Shapes.Shape.GeometryRenderer
        layer.enabled: true
        layer.samples: 4

        Shapes.ShapePath {
            strokeColor: root.color
            strokeWidth: root.strokeWidth * root.s
            fillColor: root.filled ? root.color : "transparent"
            capStyle: Shapes.ShapePath.RoundCap
            joinStyle: Shapes.ShapePath.RoundJoin

            PathSvg {
                // Dome with a flared skirt.
                path: "M 4.4 6.6 A 3.6 3.6 0 0 1 11.6 6.6 V 9.2 L 12.8 11.1 H 3.2 L 4.4 9.2 Z"
            }
        }

        Shapes.ShapePath {
            strokeColor: root.color
            strokeWidth: root.strokeWidth * root.s
            fillColor: "transparent"
            capStyle: Shapes.ShapePath.RoundCap

            PathSvg {
                // Clapper, then the stub on top.
                path: "M 6.5 13.1 A 1.5 1.5 0 0 0 9.5 13.1 M 8 2 V 2.5"
            }
        }
    }

    Shapes.Shape {
        visible: root.slashed
        anchors.fill: parent
        preferredRendererType: Shapes.Shape.GeometryRenderer
        layer.enabled: true
        layer.samples: 4

        // A sturdy outline: the slash has to cut through it without erasing it.
        Shapes.ShapePath {
            strokeColor: root.color
            strokeWidth: 2.0 * root.s
            fillColor: "transparent"
            capStyle: Shapes.ShapePath.RoundCap
            joinStyle: Shapes.ShapePath.RoundJoin

            PathSvg {
                path: "M 4.4 6.6 A 3.6 3.6 0 0 1 11.6 6.6 V 9.2 L 12.8 11.1 H 3.2 L 4.4 9.2 Z"
            }
        }

        // The slash, stroked in the row's own colours so it reads as a cut.
        // It has to live inside the Shape: a layered Shape is composited after
        // its plain siblings, so a sibling rectangle would end up underneath.
        Shapes.ShapePath {
            strokeColor: root.slashLayers[0] ?? "transparent"
            strokeWidth: 1.5 * root.s
            fillColor: "transparent"
            capStyle: Shapes.ShapePath.FlatCap

            PathSvg {
                path: "M 2.9 13.1 L 13.1 2.9"
            }
        }

        Shapes.ShapePath {
            // A second pass for the optional overlay colour; it comes out
            // transparent when the row has only one layer.
            strokeColor: root.slashLayers[1] ?? "transparent"
            strokeWidth: 1.5 * root.s
            fillColor: "transparent"
            capStyle: Shapes.ShapePath.FlatCap

            PathSvg {
                path: "M 2.9 13.1 L 13.1 2.9"
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
                path: "M 8 3.8 V 12.2 M 3.8 8 H 12.2"
            }
        }
    }
}