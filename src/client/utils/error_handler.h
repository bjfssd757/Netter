#ifndef ERROR_HANDLER_H
#define ERROR_HANDLER_H

#include <QString>
#include <QWidget>
#include "../ui/dialogs/error_dialog.h"

class ErrorHandler
{
public:
    void showError(const QString& title, const QString& message, QWidget* parent = nullptr);
    void logError(const QString& message);
};

#endif // ERROR_HANDLER_H