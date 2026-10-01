/*
    SPDX-License-Identifier: MIT

    QML extension plugin. Exports the Bridge singleton, which is the only
    thing the applet's QML needs from C++. Shipping it as an ordinary QML
    module keeps the applet itself a plain QML KPackage, so there is no plugin
    loading to arrange on Plasma's side.
*/
#include "bridge.h"

#include <QQmlEngine>
#include <QQmlExtensionPlugin>
#include <QtQml/qqml.h>

class NtfyWidgetPlugin : public QQmlExtensionPlugin
{
    Q_OBJECT
    Q_PLUGIN_METADATA(IID "org.qt-project.Qt.QQmlExtensionInterface")

public:
    void registerTypes(const char *uri) override
    {
        Q_ASSERT(QLatin1String(uri) == QLatin1String("org.ntfy.widget"));
        qmlRegisterSingletonType<Bridge>("org.ntfy.widget", 1, 0, "Bridge", Bridge::create);
    }
};

#include "plugin.moc"