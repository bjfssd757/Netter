#ifndef HEADER_H
#define HEADER_H

#include <QPushButton>
#include <QLabel>
#include <QWidget>
#include <QObject>
#include <QVBoxLayout>

class Header final : public QWidget
{
    Q_OBJECT

public:
    explicit Header(QWidget *parent = nullptr);

signals:
    void homeClicked();
    void settingsClicked();

private:
    QPushButton *home;
    QPushButton *settings;

};

#endif // HEADER_H
