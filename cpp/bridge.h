/*
    SPDX-License-Identifier: MIT

    QObject wrapper around the Rust core. Lives on the GUI thread; Rust events
    arrive on worker threads and are delivered through a queued connection.
*/
#pragma once

#include <QObject>
#include <QString>

class QQmlEngine;
class QJSEngine;

class Bridge : public QObject
{
    Q_OBJECT

public:
    ~Bridge() override;

    /// Factory for the QML singleton; owns the single engine-wide instance.
    static Bridge *create(QQmlEngine *engine, QJSEngine *script);

    /// Full state snapshot as JSON, delivered on the GUI thread.
    Q_INVOKABLE QString state() const;

    /// GUI thread entry point for core events; called by the shared sink.
    void handleEvent(uint32_t kind, const QString &payload);

public Q_SLOTS:
    /// Each returns an empty string on success, otherwise a message for the user.
    QString addSubscription(const QString &server, const QString &topic);
    QString removeSubscription(const QString &id);
    QString setEnabled(const QString &id, bool enabled);
    QString markRead(const QString &id);
    QString markAllRead();

Q_SIGNALS:
    void stateChanged();

private:
    explicit Bridge(QObject *parent = nullptr);

    class Private;
    Private *const d;
};