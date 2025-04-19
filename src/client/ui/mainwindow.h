#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QMainWindow>
#include <QVBoxLayout>
#include <QSplitter>
#include "components/header.h"
#include "components/footer.h"
#include "components/sidebar.h"
#include "components/editor.h"
#include "components/template_panel.h"
#include "../utils/error_handler.h"
#include "../core/cli_interface.h"

class MainWindow : public QMainWindow
{
    Q_OBJECT
    
public:
    MainWindow(QWidget *parent = nullptr);
    ~MainWindow();
    
private slots:
    void onNewFile();
    void onOpenFile();
    void onSaveFile();
    void onShowSettings();
    void onTemplateSelected(const QString& templateName);
    void onStartServer();
    void onStopServer();
    void onRestartServer();
    
private:
    void setupUI();
    void createActions();
    void createMenus();
    void loadProject(const QString& filePath);
    void applyTheme(const QString& themeName);
    void onThemeChanged(const QString& newTheme);
    void applyEditorSettings();
    void applyUISettings();
    void closeEvent(QCloseEvent* event) override;
    
    Header* m_header;
    Footer* m_footer;
    Sidebar* m_sidebar;
    Editor* m_editor;
    TemplatePanel* m_templatePanel;
    QWidget* m_centralWidget;
    QVBoxLayout* m_mainLayout;
    QSplitter* m_horizontalSplitter;
    QSplitter* m_verticalSplitter;
    ErrorHandler m_errorHandler;
    QString m_currentFilePath;
    CliInterface* m_cliInterface;
};

#endif // MAINWINDOW_H