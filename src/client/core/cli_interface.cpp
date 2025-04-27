#include "cli_interface.h"
#include <QFileInfo>
#include <QDebug>
#include <QRegularExpression>
#include <QThread>

QString CliInterface::m_path = QString();

CliInterface::CliInterface(QObject *parent)
    : QObject(parent)
    , m_process(nullptr)
    , m_serverRunning(false)
    , m_serverHostPort("")
{
    m_process = new QProcess(this);

    connect(m_process, &QProcess::readyReadStandardOutput, this, &CliInterface::handleProcessOutput);
    connect(m_process, &QProcess::readyReadStandardError, this, &CliInterface::handleProcessOutput);
    connect(m_process, &QProcess::errorOccurred, this, &CliInterface::handleProcessError);
    connect(m_process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), 
            this, &CliInterface::handleProcessFinished);
}

CliInterface::~CliInterface()
{
    if (m_process) {
        if (m_process->state() != QProcess::NotRunning) {
            stopServer();
            m_process->waitForFinished(3000);
        }
        delete m_process;
    }
}

bool CliInterface::startServer(const QString& filePath)
{
    if (m_serverRunning) {
        qWarning() << "Server is already running";
        return false;
    }

    if (!QFileInfo::exists(filePath)) {
        QString error = "Input file does not exist: " + filePath;
        qWarning() << error;
        emit processError(error);
        return false;
    }
    
    if (m_process->state() != QProcess::NotRunning) {
        m_process->terminate();
        m_process->waitForFinished(3000);
        if (m_process->state() != QProcess::NotRunning) {
            m_process->kill();
            m_process->waitForFinished();
        }
    }
    
    QString absoluteFilePath = QFileInfo(filePath).absoluteFilePath();
    
    QStringList arguments;
    arguments << "parse" << "--path" << absoluteFilePath;
    
    QString command = "netter " + arguments.join(" ");
    qDebug() << "Starting server:" << command;
    emit processStarted(command);
    
    m_process->start("netter", arguments);
    
    if (!m_process->waitForStarted(5000)) {
        QString error = "Failed to start Netter server: " + m_process->errorString();
        qWarning() << error;
        emit processError(error);
        return false;
    }
    
    qDebug() << "Process started with PID:" << m_process->processId();
    return true;
}

bool CliInterface::restartServer()
{
    if (!m_serverRunning) {
        qWarning() << "Server is not running, cannot restart";
        return false;
    }
    
    QString filePath = m_process->arguments().last();
    stopServer();
    
    QThread::msleep(500);
    
    return startServer(filePath);
}

bool CliInterface::stopServer()
{
    if (!m_serverRunning) {
        qDebug() << "Server is not running";
        return true;
    }
    
    if (m_process->state() != QProcess::NotRunning) {
        qDebug() << "Stopping server...";
        m_process->terminate();
        
        if (!m_process->waitForFinished(3000)) {
            qDebug() << "Server did not terminate gracefully, killing process";
            m_process->kill();
        }
    }
    
    m_serverRunning = false;
    emit serverStopped();
    return true;
}

bool CliInterface::isServerRunning() const
{
    return m_serverRunning;
}

QString CliInterface::parsePath(const QString& filePath, bool* success)
{
    if (!QFileInfo::exists(filePath)) {
        qWarning() << "Input file does not exist:" << filePath;
        if (success) *success = false;
        emit processError("Input file does not exist: " + filePath);
        return QString();
    }
    
    if (m_process->state() != QProcess::NotRunning) {
        m_process->terminate();
        m_process->waitForFinished(3000);
        if (m_process->state() != QProcess::NotRunning) {
            m_process->kill();
            m_process->waitForFinished();
        }
    }
    
    QString absoluteFilePath = QFileInfo(filePath).absoluteFilePath();
    
    QStringList arguments;
    arguments << "parse" << "--path" << absoluteFilePath;
    
    QString command = "netter " + arguments.join(" ");
    qDebug() << "Executing command:" << command;
    emit processStarted(command);
    
    m_process->start("netter", arguments);
    
    if (!m_process->waitForStarted(5000)) {
        QString error = "Failed to start Netter CLI: " + m_process->errorString();
        qWarning() << error;
        if (success) *success = false;
        emit processError(error);
        return QString();
    }
    
    qDebug() << "Process started with PID:" << m_process->processId();
    
    if (!m_process->waitForFinished(30000)) {
        QString error = "Netter CLI process timed out";
        qWarning() << error;
        m_process->terminate();
        if (success) *success = false;
        emit processError(error);
        return QString();
    }
    
    QString output = QString::fromUtf8(m_process->readAllStandardOutput());
    QString errorOutput = QString::fromUtf8(m_process->readAllStandardError());
    
    qDebug() << "Process exit code:" << m_process->exitCode();
    qDebug() << "Process stdout:" << output;
    qDebug() << "Process stderr:" << errorOutput;
    
    int exitCode = m_process->exitCode();
    if (exitCode != 0) {
        qWarning() << "Netter CLI returned error code:" << exitCode;
        qWarning() << "Error output:" << errorOutput;
        if (success) *success = false;
        emit processError("Failed to parse file: " + errorOutput);
        return QString();
    }
    
    if (success) *success = true;
    return output;
}

void CliInterface::handleProcessOutput()
{
    QString output = QString::fromUtf8(m_process->readAllStandardOutput());
    QString errorOutput = QString::fromUtf8(m_process->readAllStandardError());
    
    if (!output.isEmpty()) {
        emit outputReceived(output);

        QRegularExpression serverStartedRegex("server starting at ([^\\s]+)");
        QRegularExpressionMatch match = serverStartedRegex.match(output);
        
        if (match.hasMatch()) {
            m_serverHostPort = match.captured(1);
            m_serverRunning = true;
            qDebug() << "Server started at" << m_serverHostPort;
            emit serverStarted(m_serverHostPort);
        }
    }
    
    if (!errorOutput.isEmpty()) {
        emit outputReceived(errorOutput);
        
        if (!m_serverRunning) {
            emit serverError(errorOutput);
        }
    }
}

void CliInterface::handleProcessError(QProcess::ProcessError error)
{
    QString errorMsg;
    
    switch (error) {
        case QProcess::FailedToStart:
            errorMsg = "Failed to start Netter process: executable not found or insufficient permissions";
            break;
        case QProcess::Crashed:
            errorMsg = "Netter process crashed";
            break;
        case QProcess::Timedout:
            errorMsg = "Netter process timed out";
            break;
        case QProcess::WriteError:
            errorMsg = "Error writing to Netter process";
            break;
        case QProcess::ReadError:
            errorMsg = "Error reading from Netter process";
            break;
        case QProcess::UnknownError:
            errorMsg = "Unknown error with Netter process";
            break;
    }
    
    qWarning() << "Process error:" << errorMsg;
    emit processError(errorMsg);
    
    m_serverRunning = false;
    emit serverStopped();
}

void CliInterface::handleProcessFinished(int exitCode, QProcess::ExitStatus exitStatus)
{
    if (m_serverRunning) {
        QString message;
        
        if (exitStatus == QProcess::NormalExit) {
            message = "Server stopped with exit code " + QString::number(exitCode);
        } else {
            message = "Server crashed";
        }
        
        qDebug() << message;
        m_serverRunning = false;
        emit serverStopped();
    }
}

bool CliInterface::isNetterAvailable()
{
    QProcess checkProcess;
    checkProcess.start("netter", QStringList() << "--version");
    
    bool started = checkProcess.waitForStarted(3000);
    if (!started) {
        qWarning() << "Failed to start netter process:" << checkProcess.errorString();
        return false;
    }
    
    bool finished = checkProcess.waitForFinished(5000);
    if (!finished) {
        qWarning() << "Netter process did not respond in time";
        checkProcess.terminate();
        return false;
    }
    
    QString output = QString::fromUtf8(checkProcess.readAllStandardOutput());
    QString error = QString::fromUtf8(checkProcess.readAllStandardError());
    
    qDebug() << "Netter version check output:" << output;
    if (!error.isEmpty()) {
        qDebug() << "Netter version check error:" << error;
    }
    
    return checkProcess.exitCode() == 0;
}

// void CliInterface::setNetterPath()
// {
//     CliInterface::m_path = QString(NETTER_PATH);

//     if (m_path.isEmpty()) {
//         qWarning() << "Netter path is empty";
//         return;
//     } else {
//         QProcess checkProcess;
//         QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
//         QString currentPath = env.value("PATH");

//         #ifdef Q_OS_WIN
//         env.insert("PATH", currentPath + ";" + m_path);
//         #else
//         env.insert("PATH", currentPath + ":" + m_path);
//         #endif

//         checkProcess.setProcessEnvironment(env);

//         checkProcess.start("netter", QStringList() << "--version");
//         if (!checkProcess.waitForStarted()) {
//             qWarning() << "Failed to start netter process:" << checkProcess.errorString();
//             return;
//         }
//     }
// }