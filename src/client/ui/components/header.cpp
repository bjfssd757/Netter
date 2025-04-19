#include "header.h"
#include <QHBoxLayout>
#include <QIcon>
#include <QPushButton>

Header::Header(QWidget *parent)
    : QWidget(parent)
{
    setObjectName("header");
    m_layout = new QHBoxLayout(this);
    m_layout->setContentsMargins(4, 4, 4, 4);
    m_layout->setSpacing(4);
    
    m_newButton = new QPushButton(tr("New"), this);
    m_openButton = new QPushButton(tr("Open"), this);
    m_saveButton = new QPushButton(tr("Save"), this);
    m_settingsButton = new QPushButton(tr("Settings"), this);
    
    m_startServerButton = new QPushButton(tr("Start Server"), this);
    m_stopServerButton = new QPushButton(tr("Stop Server"), this);
    m_restartServerButton = new QPushButton(tr("Restart Server"), this);
    
    m_layout->addWidget(m_newButton);
    m_layout->addWidget(m_openButton);
    m_layout->addWidget(m_saveButton);
    m_layout->addWidget(m_generateButton);
    m_layout->addStretch();
    m_layout->addWidget(m_startServerButton);
    m_layout->addWidget(m_stopServerButton);
    m_layout->addWidget(m_restartServerButton);
    m_layout->addStretch();
    m_layout->addWidget(m_settingsButton);
    
    connect(m_newButton, &QPushButton::clicked, this, &Header::newFileRequested);
    connect(m_openButton, &QPushButton::clicked, this, &Header::openFileRequested);
    connect(m_saveButton, &QPushButton::clicked, this, &Header::saveFileRequested);
    connect(m_generateButton, &QPushButton::clicked, this, &Header::generateRequested);
    connect(m_settingsButton, &QPushButton::clicked, this, &Header::settingsRequested);
    connect(m_startServerButton, &QPushButton::clicked, this, &Header::startServerRequested);
    connect(m_stopServerButton, &QPushButton::clicked, this, &Header::stopServerRequested);
    connect(m_restartServerButton, &QPushButton::clicked, this, &Header::restartServerRequested);
    
    setServerRunning(false);
}

void Header::setServerRunning(bool running)
{
    m_startServerButton->setVisible(!running);
    m_startServerButton->setEnabled(!running);
    
    m_stopServerButton->setVisible(running);
    m_stopServerButton->setEnabled(running);
    
    m_restartServerButton->setVisible(running);
    m_restartServerButton->setEnabled(running);
}

void Header::setupActions()
{
    m_newAction = m_toolbar->addAction(QIcon(":/assets/icons/new.png"), "New");
    m_openAction = m_toolbar->addAction(QIcon(":/assets/icons/open.png"), "Open");
    m_saveAction = m_toolbar->addAction(QIcon(":/assets/icons/save.png"), "Save");
    m_toolbar->addSeparator();
    m_generateAction = m_toolbar->addAction(QIcon(":/assets/icons/generate.png"), "Generate");
    m_toolbar->addSeparator();
    m_settingsAction = m_toolbar->addAction(QIcon(":/assets/icons/settings.png"), "Settings");
    
    connect(m_newAction, &QAction::triggered, this, &Header::newFileRequested);
    connect(m_openAction, &QAction::triggered, this, &Header::openFileRequested);
    connect(m_saveAction, &QAction::triggered, this, &Header::saveFileRequested);
    connect(m_generateAction, &QAction::triggered, this, &Header::generateRequested);
    connect(m_settingsAction, &QAction::triggered, this, &Header::settingsRequested);
}