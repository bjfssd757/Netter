#include "headers/cli.h"
#include <QDir>
#include <QProcess>

Netter::Netter() : _serverPid(0) {

}

void Netter::addParameter(const QString &param) {
    _parameters.append(param);
}

void Netter::addParameters(const QStringList &params) {
    _parameters.append(params);
}

bool Netter::startServer(QString *errorMessage) {
    if (_serverPath.isEmpty()) {
        if (errorMessage) {
            *errorMessage = "Server path is not set";
        }
        return false;
    }

    const bool success = QProcess::startDetached(
            _serverPath,
            _parameters,
            QDir::currentPath(),
            &_serverPid
        );

    if (!success && errorMessage) {
        *errorMessage = "Failed to start server";
    }

    return success;
}

qint64 Netter::getPid() const {
    return _serverPid;
}
