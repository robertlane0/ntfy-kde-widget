/*
    SPDX-License-Identifier: MIT

    Development helper: shows a QML file in a window so UI changes can be
    checked without restarting the shell.
*/
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlEngine>
#include <QQmlError>
#include <QTimer>

#include <cstdio>

namespace
{
void report(const QList<QQmlError> &errors)
{
    for (const auto &e : errors) {
        fprintf(stderr, "  %s\n", qPrintable(e.toString()));
    }
}
} // namespace

int main(int argc, char **argv)
{
    qputenv("QT_QUICK_CONTROLS_STYLE", "Basic");
    QGuiApplication app(argc, argv);

    const QStringList args = app.arguments();
    if (args.size() < 2) {
        fprintf(stderr, "usage: %s <file.qml>\n", argv[0]);
        return 2;
    }

    QQmlApplicationEngine engine;
    QObject::connect(&engine, &QQmlEngine::warnings, &app, &report);
    engine.load(QUrl::fromLocalFile(args.at(1)));
    if (engine.rootObjects().isEmpty()) {
        fprintf(stderr, "failed to load %s\n", qPrintable(args.at(1)));
        return 1;
    }
    // Close on its own so the tool leaves nothing behind.
    QTimer::singleShot(120000, &app, &QGuiApplication::quit);
    return app.exec();
}