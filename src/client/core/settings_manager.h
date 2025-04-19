#ifndef JSON_SETTINGS_H
#define JSON_SETTINGS_H

#include <QObject>
#include <QJsonObject>
#include <QVariantMap>

class JsonSettings : public QObject
{
    Q_OBJECT

public:
    static JsonSettings& instance();
    
    bool load();
    bool save() const;
    
    QJsonObject getSettings() const;
    void setSettings(const QJsonObject& settings);
    
    QVariant getValue(const QString& key, const QVariant& defaultValue = QVariant()) const;
    void setValue(const QString& key, const QVariant& value);
    
    QJsonObject getGroup(const QString& groupName) const;
    void setGroup(const QString& groupName, const QJsonObject& groupData);

    void applySettings(QWidget* mainwindow);
    void debugSettings() const;

signals:
    void themeChanged(const QString& newTheme);
    void editorSettingsChanged();
    void uiSettingsChanged();
    
private:
    explicit JsonSettings(QObject *parent = nullptr);

    void applyThemeSettings(QWidget* mainWindow);
    void applyEditorSettings();
    void applyWindowSettings(QWidget* mainWindow);
    
    QString m_settingsFilePath;
    QJsonObject m_settings;
};

#endif // JSON_SETTINGS_H