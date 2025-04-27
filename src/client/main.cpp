#include "ui/mainwindow.h"
#include "core/settings_manager.h"
#include <QApplication>
#include <QFile>
#include <QTextStream>
#include <QCommandLineParser>
#include "core/cli_interface.h"

static QFile logFile("debug_output.log");

void customMessageHandler(QtMsgType type, const QMessageLogContext &context, const QString &msg) {
    if (!logFile.isOpen()) {
        return;
    }

    QTextStream out(&logFile);
    switch (type) {
    case QtDebugMsg:
        out << "[DEBUG] ";
        break;
    case QtWarningMsg:
        out << "[WARNING] ";
        break;
    case QtCriticalMsg:
        out << "[CRITICAL] ";
        break;
    case QtFatalMsg:
        out << "[FATAL] ";
        break;
    }
    out << msg << Qt::endl;
    out.flush();
}

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
    app.setApplicationName("NetterUI");
    app.setOrganizationName("Netter");

    if (logFile.open(QIODevice::WriteOnly | QIODevice::Append)) {
        qDebug() << "Logging to debug_output.log started.";
        qInstallMessageHandler(customMessageHandler);
    } else {
        qWarning() << "Failed to open log file.";
    }

    JsonSettings::instance().load();
    JsonSettings::instance().debugSettings();
    
    QFile styleFile(":src/client/assets/styles/main.qss");
    if (styleFile.open(QFile::ReadOnly | QFile::Text)) {
        QTextStream stream(&styleFile);
        app.setStyleSheet(stream.readAll());
        styleFile.close();
    }
    
    MainWindow mainWindow;
    mainWindow.show();

    QObject::connect(&app, &QApplication::aboutToQuit, []() {
        qDebug() << "Приложение завершается, сохранение настроек...";
        JsonSettings::instance().save();
    });
    
    return app.exec();
}