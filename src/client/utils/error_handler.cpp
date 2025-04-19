#include "error_handler.h"
#include <QDebug>

void ErrorHandler::showError(const QString& title, const QString& message, QWidget* parent)
{
    ErrorDialog dialog(title, message, parent);
    dialog.exec();
    
    logError(title + ": " + message);
}

void ErrorHandler::logError(const QString& message)
{
    qCritical() << "ERROR:" << message;
    // Здесь можно добавить логирование в файл
}