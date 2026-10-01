/*
    SPDX-License-Identifier: MIT

    QObject wrapper around the Rust core. Lives on the GUI thread; Rust events
    arrive on worker threads and are delivered through a queued connection.
*/
#pragma once

#include <QObject>
#include <QString>

class QJsonObject;
class QQmlEngine;
class QJSEngine;

class Bridge : public QObject
{
    Q_OBJECT
    Q_PROPERTY(int unread READ unread NOTIFY stateChanged)

public:
    ~Bridge() override;

    /// Factory for the QML singleton; owns the single engine-wide instance.
    static Bridge *create(QQmlEngine *engine, QJSEngine *script);

    /// Full state snapshot as JSON, delivered on the GUI thread.
    Q_INVOKABLE QString state() const;
    Q_INVOKABLE QString recent() const;
    Q_INVOKABLE int unread() const;

    /// GUI thread entry point for core events; called by the shared sink.
    void handleEvent(uint32_t kind, const QString &payload);

public Q_SLOTS:
    /// Each returns an empty string on success, otherwise a message for the user.
    QString addSubscription(const QString &server, const QString &topic);
    QString removeSubscription(const QString &id);
    QString setEnabled(const QString &id, bool enabled);
    QString markRead(const QString &id);
    QString markAllRead();
    QString openUrl(const QString &url);

Q_SIGNALS:
    void stateChanged();

private:
    explicit Bridge(QObject *parent = nullptr);

    void notify(const QJsonObject &payload);

    class Private;
    Private *const d;
};