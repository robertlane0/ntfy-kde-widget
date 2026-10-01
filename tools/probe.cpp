/*
    SPDX-License-Identifier: MIT

    Development helper: checks that Plasma can discover and load the applet
    package exactly the way plasmashell does. Run with the built package
    installed for the current user.
*/
#include <KPackage/Package>
#include <KPackage/PackageLoader>
#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QPluginLoader>
#include <QQmlComponent>
#include <QQmlEngine>

#include <cstdio>

namespace
{
void line(const char *label, const QString &value)
{
    fprintf(stderr, "%-18s %s\n", label, qPrintable(value));
}
} // namespace

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    const QString id = argc > 1 ? QString::fromLocal8Bit(argv[1]) : QStringLiteral("org.ntfy.widget");

    auto *loader = KPackage::PackageLoader::self();
    const auto found = loader->findPackages(QStringLiteral("Plasma/Applet"), QString());
    fprintf(stderr, "Plasma/Applet packages: %d\n", int(found.size()));
    for (const auto &meta : found) {
        fprintf(stderr, "  %s\n", qPrintable(meta.pluginId()));
    }

    KPackage::Package package = loader->loadPackage(QStringLiteral("Plasma/Applet"), id);
    line("valid:", package.isValid() ? QStringLiteral("yes") : QStringLiteral("no"));
    line("package root:", package.path());
    line("main script:", package.fileUrl(QByteArrayLiteral("mainscript")).toLocalFile());
    line("X-Plasma-API:", package.metadata().value(QStringLiteral("X-Plasma-API")));

    const QString codeDir = package.fileUrl(QByteArrayLiteral("code")).toLocalFile();
    line("code dir:", codeDir);
    const QDir dir(codeDir);
    for (const QString &entry : dir.entryList(QDir::Files)) {
        fprintf(stderr, "  %s\n", qPrintable(entry));
    }

    const QString library = QCoreApplication::applicationDirPath()
        + QStringLiteral("/module/org/ntfy/widget/libntfywidget.so");
    if (QFile::exists(library)) {
        QPluginLoader plugin(library);
        const bool ok = plugin.load();
        line("plugin load:", ok ? QStringLiteral("ok") : plugin.errorString());

        // The applet's main.qml does `import org.ntfy.widget`; make sure that
        // resolves against the package's own contents/code directory.
        QQmlEngine engine;
        // No explicit import path: this mirrors what plasmashell sees.
        for (const QString &path : engine.importPathList()) {
            fprintf(stderr, "  importPath %s\n", qPrintable(path));
        }
        QQmlComponent component(&engine);
        component.setData(QByteArrayLiteral("import QtQuick\nimport org.ntfy.widget\nItem {}\n"),
                          QUrl(QStringLiteral("probe.qml")));
        const QObject *obj = component.create();
        line("qml module:", obj ? QStringLiteral("imported") : component.errorString().trimmed());
        delete obj;
    } else {
        line("plugin missing:", library);
    }
    return 0;
}