#ifndef ERROR_DIALOG_H
#define ERROR_DIALOG_H

#include <QDialog>
#include <QLabel>
#include <QTextEdit>
#include <QPushButton>

class ErrorDialog : public QDialog
{
    Q_OBJECT
    
public:
    explicit ErrorDialog(const QString& title, const QString& message, QWidget *parent = nullptr);
    
private:
    QLabel *m_titleLabel;
    QTextEdit *m_messageEdit;
    QPushButton *m_okButton;
};

#endif // ERROR_DIALOG_H