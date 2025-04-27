#ifndef CLI_INTERFACE_H
#define CLI_INTERFACE_H

#include <QObject>
#include <QString>
#include <QProcess>
#include <QCommandLineParser>

class CliInterface : public QObject
{
    Q_OBJECT

public:
    explicit CliInterface(QObject *parent = nullptr);
    ~CliInterface();

    bool startServer(const QString& filePath);
    bool restartServer();
    bool stopServer();
    bool isServerRunning() const;
    bool isNetterAvailable();
    // static void setNetterPath();
    
    QString parsePath(const QString& filePath, bool* success = nullptr);

signals:
    void processStarted(const QString& command);
    void processError(const QString& errorMessage);
    void serverStarted(const QString& hostPort);
    void serverStopped();
    void serverError(const QString& errorMessage);
    void outputReceived(const QString& output);

private slots:
    void handleProcessOutput();
    void handleProcessError(QProcess::ProcessError error);
    void handleProcessFinished(int exitCode, QProcess::ExitStatus exitStatus);

private:
    QProcess* m_process;
    bool m_serverRunning;
    QString m_serverHostPort;
    static QString m_path;
};

#endif // CLI_INTERFACE_H