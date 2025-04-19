#ifndef FOOTER_H
#define FOOTER_H

#include <QWidget>
#include <QLabel>
#include <QProgressBar>

class Footer : public QWidget
{
    Q_OBJECT
    
public:
    explicit Footer(QWidget *parent = nullptr);
    
    void showMessage(const QString& message);
    void showProgress(int value);
    void applySettings();
    
private:
    QLabel *m_statusLabel;
    QProgressBar *m_progressBar;
};

#endif // FOOTER_H