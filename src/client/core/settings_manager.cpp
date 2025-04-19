#include "settings_manager.h"
#include <QFile>
#include <QDir>
#include <QJsonDocument>
#include <QStandardPaths>
#include <QDebug>
#include <QSettings>
#include <QWidget>
#include <QScreen>

JsonSettings::JsonSettings(QObject *parent)
    : QObject(parent)
{
    QString appDataPath = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    QDir dir(appDataPath);
    if (!dir.exists()) {
        dir.mkpath(".");
    }
    
    m_settingsFilePath = dir.filePath("settings.json");
    qDebug() << "Путь к файлу настроек:" << m_settingsFilePath;
}

JsonSettings& JsonSettings::instance()
{
    static JsonSettings instance;
    return instance;
}

bool JsonSettings::load()
{
    QFile file(m_settingsFilePath);
    if (!file.exists()) {
        qDebug() << "Файл настроек не существует, будут использоваться значения по умолчанию.";
        return false;
    }
    
    if (!file.open(QIODevice::ReadOnly)) {
        qWarning() << "Не удалось открыть файл настроек для чтения:" << file.errorString();
        return false;
    }
    
    QByteArray data = file.readAll();
    file.close();
    
    QJsonParseError error;
    QJsonDocument doc = QJsonDocument::fromJson(data, &error);
    
    if (error.error != QJsonParseError::NoError) {
        qWarning() << "Ошибка при разборе JSON:" << error.errorString();
        return false;
    }
    
    if (!doc.isObject()) {
        qWarning() << "Некорректный формат файла настроек: корневой элемент не является объектом JSON";
        return false;
    }
    
    m_settings = doc.object();
    qDebug() << "Настройки успешно загружены из:" << m_settingsFilePath;
    
    return true;
}

bool JsonSettings::save() const
{
    QDir dir(QFileInfo(m_settingsFilePath).dir());
    if (!dir.exists()) {
        dir.mkpath(".");
    }

    QFile file(m_settingsFilePath);
    if (!file.open(QIODevice::WriteOnly)) {
        qWarning() << "Не удалось открыть файл настроек для записи:" << file.errorString();
        return false;
    }
    
    QJsonDocument doc(m_settings);
    QByteArray jsonData = doc.toJson(QJsonDocument::Indented);
    
    qint64 bytesWritten = file.write(jsonData);
    file.close();
    
    if (bytesWritten != jsonData.size()) {
        qWarning() << "Не удалось записать все данные настроек";
        return false;
    }
    
    qDebug() << "Настройки успешно сохранены в:" << m_settingsFilePath;
    return true;
}

QJsonObject JsonSettings::getSettings() const
{
    return m_settings;
}

void JsonSettings::setSettings(const QJsonObject& settings)
{
    m_settings = settings;
}

QVariant JsonSettings::getValue(const QString& key, const QVariant& defaultValue) const
{
    if (!m_settings.contains(key)) {
        return defaultValue;
    }
    
    return m_settings.value(key).toVariant();
}

void JsonSettings::setValue(const QString& key, const QVariant& value)
{
    m_settings.insert(key, QJsonValue::fromVariant(value));
}

QJsonObject JsonSettings::getGroup(const QString& groupName) const
{
    if (!m_settings.contains(groupName) || !m_settings.value(groupName).isObject()) {
        return QJsonObject();
    }
    
    return m_settings.value(groupName).toObject();
}

void JsonSettings::setGroup(const QString& groupName, const QJsonObject& groupData)
{
    m_settings.insert(groupName, groupData);
}


void JsonSettings::applyEditorSettings()
{
    QJsonObject editorConfig = getGroup("editor");
    
    QString fontFamily = editorConfig.value("font_family").toString("Consolas");
    int fontSize = editorConfig.value("font_size").toInt(11);
    int tabSize = editorConfig.value("tab_size").toInt(4);
    bool showLineNumbers = editorConfig.value("show_line_numbers").toBool(true);
    bool highlightCurrentLine = editorConfig.value("highlight_current_line").toBool(true);
    
    emit editorSettingsChanged();
}

void JsonSettings::applyThemeSettings(QWidget* mainWindow)
{
    QString themeName = getValue("ui/theme", "Default").toString();
    
    QString stylesheetPath;
    if (themeName == "Dark") {
        stylesheetPath = ":src/client/assets/styles/dark.qss";
    } else if (themeName == "Light") {
        stylesheetPath = ":src/client/assets/styles/light.qss";
    } else if (themeName == "System") {
        bool isDarkMode = false;
        #ifdef Q_OS_WIN
            QSettings winRegistry("HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize", 
                                QSettings::NativeFormat);
            isDarkMode = !winRegistry.value("AppsUseLightTheme", 1).toBool();
        #endif
        
        stylesheetPath = isDarkMode ? ":src/client/assets/styles/dark.qss" : ":src/client/assets/styles/light.qss";
    } else {
        stylesheetPath = ":src/client/assets/styles/default.qss";
    }

    QFile file(stylesheetPath);
    if (file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QString styleSheet = QLatin1String(file.readAll());
        mainWindow->setStyleSheet(styleSheet);
        file.close();
    } else {
        qWarning() << "Failed to open stylesheet:" << stylesheetPath;
    }
    
    emit themeChanged(themeName);
}

void JsonSettings::applyWindowSettings(QWidget* mainWindow)
{
    QJsonObject windowConfig = getGroup("ui").value("window").toObject();
    
    int width = windowConfig.value("width").toInt(1024);
    int height = windowConfig.value("height").toInt(768);
    mainWindow->resize(width, height);
    
    bool maximized = windowConfig.value("maximized").toBool(false);
    if (maximized) {
        mainWindow->showMaximized();
    }
    
    bool centerWindow = windowConfig.value("center_on_screen").toBool(true);
    if (centerWindow && !maximized) {
        QRect screenGeometry = QGuiApplication::primaryScreen()->availableGeometry();
        mainWindow->move(
            (screenGeometry.width() - mainWindow->width()) / 2,
            (screenGeometry.height() - mainWindow->height()) / 2
        );
    }
}

void JsonSettings::applySettings(QWidget* mainWindow)
{
    applyThemeSettings(mainWindow);

    emit uiSettingsChanged();
    emit editorSettingsChanged();
}

void JsonSettings::debugSettings() const
{
    qDebug() << "Текущие настройки:";
    
    QJsonDocument doc(m_settings);
    qDebug().noquote() << doc.toJson(QJsonDocument::Indented);
}