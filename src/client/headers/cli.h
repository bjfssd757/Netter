//
// Created by user on 14.04.2025.
//

#ifndef CLI_H
#define CLI_H

#include <QList>
#include <QString>

class Netter {
public:
    Netter();

    void addParameter(const QString &param);
    void addParameters(const QStringList &params);

    void setServerPath(const QString &path);

    bool startServer(QString *errorMessage = nullptr);

    qint64 getPid() const;

private:
    QString _serverPath;
    QStringList _parameters;
    qint64 _serverPid;
};

#endif //CLI_H
