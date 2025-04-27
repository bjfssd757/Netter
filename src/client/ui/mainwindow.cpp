#include "mainwindow.h"
#include "dialogs/settings_dialog.h"
#include "../utils/file_utils.h"
#include <QMessageBox>
#include <QFileDialog>
#include <QMenuBar>
#include <QStatusBar>
#include "../core/settings_manager.h"
#include "../core/cli_interface.h"
#include <QApplication>
#include <QCloseEvent>

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , m_currentFilePath("")
{
    setupUI();
    createActions();
    createMenus();
    
    // Подключаем сигналы изменения настроек к соответствующим слотам
    JsonSettings& settings = JsonSettings::instance();
    
    connect(&settings, &JsonSettings::themeChanged,
            this, &MainWindow::onThemeChanged);
            
    connect(&settings, &JsonSettings::editorSettingsChanged,
            this, &MainWindow::applyEditorSettings);
            
    connect(&settings, &JsonSettings::uiSettingsChanged,
            this, &MainWindow::applyUISettings);
    
    settings.applySettings(this);
    
    setWindowTitle("NetterUI");
    setMinimumSize(1024, 768);

    m_cliInterface = new CliInterface(this);

    if (!m_cliInterface->isNetterAvailable()) {
        qDebug() << "Netter CLI not found or not executable.";
        QMessageBox::critical(
            this,
            tr("Netter CLI Not Found"),
            tr("Could not find or execute the Netter CLI tool. "
            "Please ensure it is installed and available in your PATH.")
        );
    }

    connect(m_cliInterface, &CliInterface::processStarted, [this](const QString& command) {
        m_footer->showMessage(tr("Running: %1").arg(command));
    });
    
    connect(m_cliInterface, &CliInterface::processError, [this](const QString& errorMessage) {
        m_errorHandler.showError(tr("CLI Error"), errorMessage);
    });
    
    connect(m_cliInterface, &CliInterface::serverStarted, [this](const QString& hostPort) {
        m_footer->showMessage(tr("Server started at %1").arg(hostPort));
        m_header->setServerRunning(true);
    });
    
    connect(m_cliInterface, &CliInterface::serverStopped, [this]() {
        m_footer->showMessage(tr("Server stopped"));
        m_header->setServerRunning(false);
    });
    
    connect(m_cliInterface, &CliInterface::serverError, [this](const QString& errorMessage) {
        m_errorHandler.showError(tr("Server Error"), errorMessage);
        m_header->setServerRunning(false);
    });
    
    connect(m_cliInterface, &CliInterface::outputReceived, [this](const QString& output) {
        // There will be support for output in terminal
        qDebug() << "Server output:" << output;
    });
}

MainWindow::~MainWindow()
{
    if (m_cliInterface && m_cliInterface->isServerRunning()) {
        m_cliInterface->stopServer();
    }
}

void MainWindow::setupUI()
{
    m_centralWidget = new QWidget(this);
    setCentralWidget(m_centralWidget);
    
    m_mainLayout = new QVBoxLayout(m_centralWidget);
    m_mainLayout->setSpacing(0);
    m_mainLayout->setContentsMargins(0, 0, 0, 0);
    
    m_header = new Header(this);
    m_footer = new Footer(this);
    m_sidebar = new Sidebar(this);
    m_editor = new Editor(this);
    m_templatePanel = new TemplatePanel(this);
    
    m_horizontalSplitter = new QSplitter(Qt::Horizontal);
    m_horizontalSplitter->addWidget(m_sidebar);
    
    m_verticalSplitter = new QSplitter(Qt::Vertical);
    m_verticalSplitter->addWidget(m_editor);
    m_verticalSplitter->addWidget(m_templatePanel);
    m_verticalSplitter->setStretchFactor(0, 3);
    m_verticalSplitter->setStretchFactor(1, 1);
    
    m_horizontalSplitter->addWidget(m_verticalSplitter);
    m_horizontalSplitter->setStretchFactor(0, 1);
    m_horizontalSplitter->setStretchFactor(1, 3);
    
    m_mainLayout->addWidget(m_header);
    m_mainLayout->addWidget(m_horizontalSplitter, 1);
    m_mainLayout->addWidget(m_footer);

    m_header->setObjectName("header");
    m_footer->setObjectName("footer");
    m_sidebar->setObjectName("sidebar");
    m_editor->setObjectName("codeEditor");
    
    connect(m_header, &Header::newFileRequested, this, &MainWindow::onNewFile);
    connect(m_header, &Header::openFileRequested, this, &MainWindow::onOpenFile);
    connect(m_header, &Header::saveFileRequested, this, &MainWindow::onSaveFile);
    connect(m_header, &Header::settingsRequested, this, &MainWindow::onShowSettings);
    connect(m_templatePanel, &TemplatePanel::templateSelected, this, &MainWindow::onTemplateSelected);
    
    statusBar()->addWidget(m_footer);

    connect(m_header, &Header::startServerRequested, this, &MainWindow::onStartServer);
    connect(m_header, &Header::stopServerRequested, this, &MainWindow::onStopServer);
    connect(m_header, &Header::restartServerRequested, this, &MainWindow::onRestartServer);
}

void MainWindow::onStartServer()
{
    if (m_currentFilePath.isEmpty()) {
        QMessageBox::warning(
            this, 
            tr("Save Required"), 
            tr("You need to save the file before starting the server.")
        );
        onSaveFile();
        
        if (m_currentFilePath.isEmpty()) {
            return;
        }
    }
    
    m_cliInterface->startServer(m_currentFilePath);
}

void MainWindow::onStopServer()
{
    m_cliInterface->stopServer();
}

void MainWindow::onRestartServer()
{
    m_cliInterface->restartServer();
}

void MainWindow::createActions()
{
    // There will be actions for menu
}

void MainWindow::createMenus()
{
    // There will be menus for file, edit, etc.
}

void MainWindow::onNewFile()
{
    if (!m_editor->document()->isEmpty()) {
        QMessageBox::StandardButton reply = QMessageBox::question(
            this, "New File", "Do you want to save changes to the current file?",
            QMessageBox::Yes | QMessageBox::No | QMessageBox::Cancel);
            
        if (reply == QMessageBox::Yes) {
            onSaveFile();
        } else if (reply == QMessageBox::Cancel) {
            return;
        }
    }
    
    m_editor->clear();
    m_currentFilePath = "";
    setWindowTitle("NetterUI - New File");
    m_footer->showMessage("New file created");
}

void MainWindow::onOpenFile()
{
    QString filePath = QFileDialog::getOpenFileName(this, "Open RD File", "", "RD Files (*.rd);;All Files (*)");
    if (filePath.isEmpty()) {
        return;
    }
    
    loadProject(filePath);
}

void MainWindow::onSaveFile()
{
    if (m_currentFilePath.isEmpty()) {
        QString filePath = QFileDialog::getSaveFileName(this, "Save RD File", "", "RD Files (*.rd);;All Files (*)");
        if (filePath.isEmpty()) {
            return;
        }
        m_currentFilePath = filePath;
    }
    
    try {
        FileUtils::saveToFile(m_currentFilePath, m_editor->toPlainText());
        setWindowTitle("RD Generator - " + m_currentFilePath);
        m_footer->showMessage("File saved: " + m_currentFilePath);
    } catch (const std::exception& e) {
        m_errorHandler.showError("Save Error", e.what());
    }
}

void MainWindow::onTemplateSelected(const QString& templateName)
{
    try {
        QString templateContent = FileUtils::loadFromFile(":src/client/assets/templates/" + templateName + ".rd");
        m_editor->setPlainText(templateContent);
        m_footer->showMessage("Template loaded: " + templateName);
    } catch (const std::exception& e) {
        m_errorHandler.showError("Template Error", e.what());
    }
}

void MainWindow::loadProject(const QString& filePath)
{
    try {
        QString content = FileUtils::loadFromFile(filePath);
        m_editor->setPlainText(content);
        m_currentFilePath = filePath;
        setWindowTitle("NetterUI - " + filePath);
        m_footer->showMessage("File opened: " + filePath);
    } catch (const std::exception& e) {
        m_errorHandler.showError("Open Error", e.what());
    }
}

void MainWindow::applyTheme(const QString& themeName)
{
    QString styleFilePath;
    
    if (themeName == "Dark") {
        styleFilePath = ":src/client/assets/styles/dark.qss";
    } else if (themeName == "Light") {
        styleFilePath = ":src/client/assets/styles/light.qss";
    } else {
        styleFilePath = ":src/client/assets/styles/main.qss";
    }
    
    QFile file(styleFilePath);
    if (file.open(QFile::ReadOnly | QFile::Text)) {
        QTextStream stream(&file);
        QString styleSheet = stream.readAll();
        
        setStyleSheet(styleSheet);
        
        file.close();
    }
    
    if (m_editor) {
        m_editor->setTheme(themeName);
    }
}

void MainWindow::onShowSettings()
{
    SettingsDialog dialog(this);
    
    connect(&dialog, &SettingsDialog::settingsChanged, [this]() {
        qDebug() << "Получен сигнал settingsChanged, применяем настройки...";
        JsonSettings& settings = JsonSettings::instance();
        settings.applySettings(this);
        
        m_footer->showMessage("Settings updated");
    });
    
    int result = dialog.exec();
    qDebug() << "Диалог настроек закрыт с результатом:" << result;
}

void MainWindow::onThemeChanged(const QString& newTheme)
{
    applyTheme(newTheme);
    
    if (m_editor) {
        m_editor->setTheme(newTheme);
    }
}

void MainWindow::applyEditorSettings()
{
    if (m_editor) {
        m_editor->applySettings();
    }
}

void MainWindow::applyUISettings()
{
    if (m_sidebar) {
        m_sidebar->applySettings();
    }
    
    if (m_footer) {
        m_footer->applySettings();
    }
}

void MainWindow::closeEvent(QCloseEvent* event)
{
    if (!m_editor->document()->isEmpty() && !m_editor->document()->isModified()) {
        QMessageBox::StandardButton reply = QMessageBox::question(
            this, "Close Application", "Do you want to save changes before closing?",
            QMessageBox::Yes | QMessageBox::No | QMessageBox::Cancel);
            
        if (reply == QMessageBox::Yes) {
            onSaveFile();
            
            if (m_editor->document()->isModified()) {
                event->ignore();
                return;
            }
        } else if (reply == QMessageBox::Cancel) {
            event->ignore();
            return;
        }
    }
    
    JsonSettings& settings = JsonSettings::instance();
    QJsonObject windowConfig = settings.getGroup("ui").value("window").toObject();
    
    if (!isMaximized()) {
        windowConfig["width"] = width();
        windowConfig["height"] = height();
    }
    
    windowConfig["maximized"] = isMaximized();
    
    QJsonObject uiConfig = settings.getGroup("ui");
    uiConfig["window"] = windowConfig;
    settings.setGroup("ui", uiConfig);
    
    settings.save();
    
    QMainWindow::closeEvent(event);
}