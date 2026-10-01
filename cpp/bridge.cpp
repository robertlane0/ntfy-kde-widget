/*
    SPDX-License-Identifier: MIT
*/
#include "bridge.h"
#include "ntfy.h"

#include <QCoreApplication>
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusMessage>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLoggingCategory>
#include <QMetaObject>
#include <QProcess>
#include <QStringList>

namespace
{

constexpr auto notificationService = "org.freedesktop.Notifications";
constexpr auto notificationPath = "/org/freedesktop/Notifications";

/// The one bridge in this process, the sink feeding it and how many applet
/// views are using it. The Rust core runs only while that count is above zero.
struct Registry
{
    Bridge *bridge = nullptr;
    size_t sink = 0;
    int views = 0;
};

Registry &registry()
{
    static Registry r;
    return r;
}

/// Called by the Rust core from its worker threads.
void onRustEvent(uint32_t kind, const char *json, size_t len)
{
    Bridge *bridge = registry().bridge;
    if (!bridge) {
        return;
    }
    const QString payload = QString::fromUtf8(json, static_cast<qsizetype>(len));
    // Hop to the GUI thread: QML and DBus are both single threaded.
    QMetaObject::invokeMethod(
        bridge,
        [bridge, kind, payload] { bridge->handleEvent(kind, payload); },
        Qt::QueuedConnection);
}

/// Keeps one notification alive so its action button can open the topic.
class NotificationAction : public QObject
{
    Q_OBJECT

public:
    NotificationAction(uint id, QString url, QObject *parent = nullptr)
        : QObject(parent)
        , m_id(id)
        , m_url(std::move(url))
        , m_iface(new QDBusInterface(QString::fromLatin1(notificationService),
                                      QString::fromLatin1(notificationPath),
                                      QString::fromLatin1(notificationService),
                                      QDBusConnection::sessionBus()))
    {
        QDBusConnection::sessionBus().connect(QString::fromLatin1(notificationService),
                                              QString::fromLatin1(notificationPath),
                                              QString::fromLatin1(notificationService),
                                              QStringLiteral("ActionInvoked"),
                                              this,
                                              SLOT(onActionInvoked(quint32, QString)));
    }

private Q_SLOTS:
    void onActionInvoked(quint32 id, const QString &key)
    {
        if (id != m_id) {
            return;
        }
        if (!m_url.isEmpty()) {
            QProcess::startDetached(QStringLiteral("xdg-open"), QStringList{m_url});
        }
        m_iface->call(QStringLiteral("CloseNotification"), m_id);
        deleteLater();
    }

private:
    const uint m_id;
    const QString m_url;
    QDBusInterface *const m_iface;
};

/// Posts a desktop notification and wires its action back to the topic.
void postNotification(const QString &title, const QString &body, const QString &url, int priority)
{
    auto *iface = new QDBusInterface(QString::fromLatin1(notificationService),
                                     QString::fromLatin1(notificationPath),
                                     QString::fromLatin1(notificationService),
                                     QDBusConnection::sessionBus());
    iface->setTimeout(2000);

    // Actions are pairs: a label followed by its key. Both entries share one
    // label so the server renders a single button whichever one it picks.
    const QStringList actions =
        url.isEmpty() ? QStringList()
                      : QStringList{QStringLiteral("Open topic"), QStringLiteral("Open topic")};

    QVariantMap hints;
    hints.insert(QStringLiteral("urgency"), priority >= 5 ? 2 : (priority <= 2 ? 0 : 1));
    if (!title.isEmpty()) {
        hints.insert(QStringLiteral("x-kde-notification-title"), title);
    }

    // Urgent messages stay on screen longer; the rest clear themselves so a
    // quiet topic cannot leave a column of popups behind.
    const int timeout = priority >= 5 ? 30000 : 10000;

    const QDBusMessage reply = iface->call(QStringLiteral("Notify"),
                                           QStringLiteral("ntfy"),
                                           0u,
                                           QStringLiteral("mail-message-new"),
                                           title,
                                           body,
                                           actions,
                                           hints,
                                           timeout);

    const bool delivered = reply.type() != QDBusMessage::ErrorMessage && !url.isEmpty();
    if (delivered) {
        const uint id = reply.arguments().isEmpty() ? 0u : reply.arguments().first().toUInt();
        // Owns itself until the action fires, then cleans up.
        new NotificationAction(id, url, QCoreApplication::instance());
    }
    iface->deleteLater();
}

/// Runs a command and turns the JSON result into an empty string or an error.
QString run(uint32_t op, const QString &a = {}, const QString &b = {}, const QString &c = {})
{
    char *raw = ntfy_command(op, a.toUtf8().constData(), b.toUtf8().constData(), c.toUtf8().constData());
    if (!raw) {
        return QStringLiteral("The ntfy core did not respond");
    }
    const QByteArray text(raw);
    ntfy_string_free(raw);

    const QJsonObject obj = QJsonDocument::fromJson(text).object();
    if (obj.value(QStringLiteral("ok")).toBool()) {
        return {};
    }
    return obj.value(QStringLiteral("error")).toString(QStringLiteral("Unknown error"));
}

} // namespace

class Bridge::Private
{
public:
    QString state;
};

Bridge::Bridge(QObject *parent)
    : QObject(parent)
    , d(new Private)
{
    setObjectName(QStringLiteral("org.ntfy.widget.Bridge"));
}

Bridge *Bridge::create(QQmlEngine *, QJSEngine *)
{
    // One engine, one set of streams: hand every QML client the same object.
    Registry &r = registry();
    if (!r.bridge) {
        r.bridge = new Bridge;
    }
    return r.bridge;
}

Bridge::~Bridge()
{
    Registry &r = registry();
    if (r.bridge == this) {
        r.bridge = nullptr;
        r.views = 0;
        if (r.sink) {
            ntfy_remove_sink(r.sink);
            r.sink = 0;
        }
        ntfy_stop();
    }
    delete d;
}

void Bridge::activate()
{
    Registry &r = registry();
    if (r.views++ == 0) {
        if (!r.sink) {
            r.sink = ntfy_add_sink(&onRustEvent);
        }
        ntfy_start();
    }
}

void Bridge::deactivate()
{
    Registry &r = registry();
    // The applet is gone, so nothing should stream or pop up notifications for
    // the rest of the session.
    if (r.views > 0 && --r.views == 0) {
        ntfy_stop();
    }
}

QString Bridge::state() const
{
    return d->state;
}

void Bridge::handleEvent(uint32_t kind, const QString &payload)
{
    switch (kind) {
    case NtfyEventState:
        d->state = payload;
        Q_EMIT stateChanged();
        break;
    case NtfyEventNotify: {
        const QJsonObject msg = QJsonDocument::fromJson(payload.toUtf8()).object();
        postNotification(msg.value(QStringLiteral("title")).toString(),
                         msg.value(QStringLiteral("body")).toString(),
                         msg.value(QStringLiteral("url")).toString(),
                         msg.value(QStringLiteral("priority")).toInt());
        break;
    }
    case NtfyEventLog: {
        // The core has no logger of its own, so diagnostics ride on this event
        // and end up wherever the shell's output goes.
        const QJsonObject msg = QJsonDocument::fromJson(payload.toUtf8()).object();
        const bool warn = msg.value(QStringLiteral("level")).toString() != QLatin1String("info");
        const QString text = QStringLiteral("ntfy: %1").arg(msg.value(QStringLiteral("text")).toString());
        if (warn) {
            qWarning().noquote() << text;
        } else {
            qInfo().noquote() << text;
        }
        break;
    }
    default:
        break;
    }
}

QString Bridge::addSubscription(const QString &server, const QString &topic)
{
    return run(NtfyCmdAdd, server, topic);
}

QString Bridge::removeSubscription(const QString &id)
{
    return run(NtfyCmdRemove, id);
}

QString Bridge::setEnabled(const QString &id, bool enabled)
{
    return run(NtfyCmdSetEnabled, id, enabled ? QStringLiteral("1") : QStringLiteral("0"));
}

QString Bridge::markRead(const QString &id)
{
    return run(NtfyCmdMarkRead, id);
}

QString Bridge::markAllRead()
{
    return run(NtfyCmdMarkAllRead);
}

#include "bridge.moc"