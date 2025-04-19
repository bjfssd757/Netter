#ifndef HEADER_H
#define HEADER_H

#include <QWidget>
#include <QToolBar>
#include <QAction>
#include <QPushButton>
#include <QHBoxLayout>

class Header : public QWidget
{
    Q_OBJECT
    
public:
    explicit Header(QWidget *parent = nullptr);

    void setServerRunning(bool running);    
    
signals:
    void newFileRequested();
    void openFileRequested();
    void saveFileRequested();
    void generateRequested();
    void settingsRequested();
    void startServerRequested();
    void stopServerRequested();
    void restartServerRequested();
    
private:
    void setupActions();
    
    QToolBar *m_toolbar;
    QAction *m_newAction;
    QAction *m_openAction;
    QAction *m_saveAction;
    QAction *m_generateAction;
    QAction *m_settingsAction;

    QHBoxLayout *m_layout;
    QPushButton *m_newButton;
    QPushButton *m_openButton;
    QPushButton *m_saveButton;
    QPushButton *m_generateButton;
    QPushButton *m_settingsButton;
    QPushButton *m_startServerButton;
    QPushButton *m_stopServerButton;
    QPushButton *m_restartServerButton;
};

#endif // HEADER_H