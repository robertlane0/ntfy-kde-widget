/*
    SPDX-License-Identifier: MIT
*/
#include "bridge.h"
#include "ntfy.h"

#include <QCoreApplication>
#include <QDBusArgument>
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusMessage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMetaObject>
#include <QPointer>
#include <QProcess>
#include <QQmlEngine>
#include <QStringList>

#include <algorithm>
#include <vector>

namespace
{

struct Registry
{
    std::vector<QPointer<Bridge>> bridges;
    size_t sink = 0;
};

Registry &registry()
{
    static Registry r;
    return r;
}

/// Entry point called by the Rust core from its worker threads.
void onRustEvent(uint32_t kind, const char *json, size_t len)
{
    const QString payload = QString::fromUtf8(json, static_cast<qsizetype>(len));
    const auto snapshot = registry().bridges;
    for (const auto &bridge : snapshot) {
        if (!bridge) {
            continue;
        }
        // Hop to the GUI thread: QML and DBus are both single threaded.
        QMetaObject::invokeMethod(
            bridge,
            [bridge, kind, payload] {
                if (bridge) {
                    bridge->handleEvent(kind, payload);
                }
            },
            Qt::QueuedConnection);
    }
}

/// Keeps one notification alive so its action button can open the topic.
class NotificationAction : public QObject
{
    Q_OBJECT

public:
    NotificationAction(uint id, const QString &url, QObject *parent = nullptr)
        : QObject(parent)
        , m_id(id)
        , m_url(url)
        , m_iface(new QDBusInterface(QStringLiteral("org.freedesktop.Notifications"),
                                      QStringLiteral("/org/freedesktop/Notifications"),
                                      QStringLiteral("org.freedesktop.Notifications"),
                                      QDBusConnection::sessionBus()))
    {
        QDBusConnection::sessionBus().connect(
            QStringLiteral("org.freedesktop.Notifications"),
            QStringLiteral("/org/freedesktop/Notifications"),
            QStringLiteral("org.freedesktop.Notifications"),
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

/// Posts a desktop notification and wires its click action back to the topic.
void postNotification(const QString &title, const QString &body, const QString &url, int priority)
{
    auto *iface = new QDBusInterface(QStringLiteral("org.freedesktop.Notifications"),
                                     QStringLiteral("/org/freedesktop/Notifications"),
                                     QStringLiteral("org.freedesktop.Notifications"),
                                     QDBusConnection::sessionBus());
    iface->setTimeout(2000);

    // Actions are pairs: a label followed by its key. Both entries share the same
// label so the server renders a single button whichever one it picks.
const QStringList actions =
        url.isEmpty() ? QStringList() : QStringList{QStringLiteral("Open topic"), QStringLiteral("Open topic")};

    QVariantMap hints;
    hints.insert(QStringLiteral("urgency"), priority >= 5 ? 2 : (priority <= 2 ? 0 : 1));
    if (!title.isEmpty()) {
        hints.insert(QStringLiteral("x-kde-notification-title"), title);
    }

    // Urgent messages stay on screen longer; the rest use the server default.
    const int timeout = priority >= 5 ? 20000 : -1;

    const QDBusMessage reply = iface->call(
        QStringLiteral("Notify"),
        QStringLiteral("ntfy"),
        0u,
        QStringLiteral("mail-message-new"),
        title,
        body,
        actions,
        hints,
        timeout);

    if (reply.type() == QDBusMessage::ErrorMessage) {
        // Nothing is listening for desktop notifications; drop it quietly.
        iface->deleteLater();
        return;
    }

    if (url.isEmpty()) {
        iface->deleteLater();
        return;
    }

    const uint id = reply.arguments().isEmpty() ? 0u : reply.arguments().first().toUInt();
    // Owns itself until the action fires, then cleans up.
    new NotificationAction(id, url, QCoreApplication::instance());
    iface->deleteLater();
}

} // namespace

class Bridge::Private
{
public:
    QString state;
    QString recent;
    int unread = 0;
};

Bridge::Bridge(QObject *parent)
    : QObject(parent)
    , d(new Private)
{
    setObjectName(QStringLiteral("org.ntfy.widget.Bridge"));
    registry().bridges.push_back(this);
    if (registry().bridges.size() == 1) {
        registry().sink = ntfy_add_sink(&onRustEvent);
    }
    ntfy_start();
}

Bridge *Bridge::create(QQmlEngine *engine, QJSEngine *)
{
    // One engine, one set of streams: hand every QML client the same object.
    Q_UNUSED(engine)
    static Bridge *singleton = new Bridge;
    return singleton;
}

Bridge::~Bridge()
{
    auto &bridges = registry().bridges;
    bridges.erase(std::remove(bridges.begin(), bridges.end(), QPointer<Bridge>(this)), bridges.end());
    if (bridges.empty()) {
        ntfy_remove_sink(registry().sink);
        registry().sink = 0;
        ntfy_stop();
    }
    delete d;
}

QString Bridge::state() const
{
    return d->state;
}

QString Bridge::recent() const
{
    return d->recent;
}

int Bridge::unread() const
{
    return d->unread;
}

void Bridge::handleEvent(uint32_t kind, const QString &payload)
{
    const QJsonObject obj = QJsonDocument::fromJson(payload.toUtf8()).object();
    switch (kind) {
    case NtfyEventState:
        d->state = payload;
        d->recent = QString::fromUtf8(
            QJsonDocument(obj.value(QStringLiteral("recent")).toArray()).toJson(QJsonDocument::Compact));
        d->unread = obj.value(QStringLiteral("unread")).toInt();
        Q_EMIT stateChanged();
        break;
    case NtfyEventNotify:
        notify(obj);
        break;
    default:
        break;
    }
}

void Bridge::notify(const QJsonObject &payload)
{
    postNotification(
        payload.value(QStringLiteral("title")).toString(),
        payload.value(QStringLiteral("body")).toString(),
        payload.value(QStringLiteral("url")).toString(),
        payload.value(QStringLiteral("priority")).toInt());
}

namespace
{
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

QString Bridge::openUrl(const QString &url)
{
    return run(NtfyCmdOpen, url);
}

#include "bridge.moc"
