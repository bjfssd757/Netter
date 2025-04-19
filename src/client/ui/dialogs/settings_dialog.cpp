#include "settings_dialog.h"
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QFormLayout>
#include <QLabel>
#include <QPushButton>
#include <QFileDialog>
#include <QSettings>
#include <QGroupBox>
#include "../../core/settings_manager.h"

SettingsDialog::SettingsDialog(QWidget *parent)
    : QDialog(parent)
{
    setWindowTitle("Settings");
    setMinimumWidth(400);
    
    QVBoxLayout *mainLayout = new QVBoxLayout(this);
    
    QGroupBox *pathsGroup = new QGroupBox("Paths", this);
    QFormLayout *pathsLayout = new QFormLayout(pathsGroup);
    
    m_execPathEdit = new QLineEdit(this);
    QPushButton *browseCLIButton = new QPushButton("...", this);
    browseCLIButton->setFixedWidth(30);
    
    QHBoxLayout *execPathLayout = new QHBoxLayout();
    execPathLayout->addWidget(m_execPathEdit);
    execPathLayout->addWidget(browseCLIButton);
    
    m_templatesPathEdit = new QLineEdit(this);
    QPushButton *browseTemplatesButton = new QPushButton("...", this);
    browseTemplatesButton->setFixedWidth(30);
    
    QHBoxLayout *templatesPathLayout = new QHBoxLayout();
    templatesPathLayout->addWidget(m_templatesPathEdit);
    templatesPathLayout->addWidget(browseTemplatesButton);
    
    pathsLayout->addRow("CLI executable:", execPathLayout);
    pathsLayout->addRow("Templates folder:", templatesPathLayout);
    
    QGroupBox *uiGroup = new QGroupBox("Interface", this);
    QFormLayout *uiLayout = new QFormLayout(uiGroup);
    
    m_themeComboBox = new QComboBox(this);
    m_themeComboBox->addItems(QStringList() << "Light" << "Dark" << "System");
    
    m_autoSaveCheckBox = new QCheckBox("Enable auto-save", this);
    
    uiLayout->addRow("Theme:", m_themeComboBox);
    uiLayout->addRow("", m_autoSaveCheckBox);
    
    QHBoxLayout *buttonsLayout = new QHBoxLayout();
    m_saveButton = new QPushButton("Save", this);
    m_cancelButton = new QPushButton("Cancel", this);
    
    buttonsLayout->addStretch(1);
    buttonsLayout->addWidget(m_saveButton);
    buttonsLayout->addWidget(m_cancelButton);
    
    mainLayout->addWidget(pathsGroup);
    mainLayout->addWidget(uiGroup);
    mainLayout->addStretch(1);
    mainLayout->addLayout(buttonsLayout);
    
    connect(m_saveButton, &QPushButton::clicked, this, &SettingsDialog::onSaveSettings);
    connect(m_cancelButton, &QPushButton::clicked, this, &QDialog::reject);
    connect(browseCLIButton, &QPushButton::clicked, [this]() {
        QString path = QFileDialog::getOpenFileName(this, "Select CLI Executable");
        if (!path.isEmpty()) {
            m_execPathEdit->setText(path);
        }
    });
    connect(browseTemplatesButton, &QPushButton::clicked, [this]() {
        QString path = QFileDialog::getExistingDirectory(this, "Select Templates Directory");
        if (!path.isEmpty()) {
            m_templatesPathEdit->setText(path);
        }
    });

    QGroupBox *editorGroup = new QGroupBox("Code Editor", this);
    QFormLayout *editorLayout = new QFormLayout(editorGroup);

    m_editorThemeComboBox = new QComboBox(this);
    m_editorThemeComboBox->addItems(QStringList() 
        << "Default" 
        << "Dark Theme" 
        << "Solarized Light" 
        << "Solarized Dark"
        << "Monokai"
        << "GitHub"
    );

    m_fontSizeSpinner = new QSpinBox(this);
    m_fontSizeSpinner->setRange(8, 24);
    m_fontSizeSpinner->setValue(11);

    m_lineNumbersCheckBox = new QCheckBox("Show line numbers", this);
    m_highlightLineCheckBox = new QCheckBox("Highlight current line", this);

    m_tabSizeSpinner = new QSpinBox(this);
    m_tabSizeSpinner->setRange(2, 8);
    m_tabSizeSpinner->setValue(4);
    
    editorLayout->addRow("Theme:", m_editorThemeComboBox);
    editorLayout->addRow("Font size:", m_fontSizeSpinner);
    editorLayout->addRow("", m_lineNumbersCheckBox);
    editorLayout->addRow("", m_highlightLineCheckBox);
    editorLayout->addRow("Tab size:", m_tabSizeSpinner);

    mainLayout->addWidget(editorGroup);
    
    loadSettings();
}

void SettingsDialog::loadSettings()
{
    JsonSettings& settings = JsonSettings::instance();
    
    m_themeComboBox->setCurrentText(settings.getValue("ui/theme", "Default").toString());
    m_fontSizeSpinner->setValue(settings.getValue("editor/font_size", 11).toInt());
    m_lineNumbersCheckBox->setChecked(settings.getValue("editor/show_line_numbers", true).toBool());
    
    QJsonObject editorConfig = settings.getGroup("editor");
    m_autoSaveCheckBox->setChecked(editorConfig.value("auto_save").toBool(true));
    m_tabSizeSpinner->setValue(editorConfig.value("tab_size").toInt(4));
}

void SettingsDialog::onSaveSettings()
{
    JsonSettings& settings = JsonSettings::instance();
    
    qDebug() << "Сохранение настроек из диалога...";
    
    settings.setValue("ui/theme", m_themeComboBox->currentText());
    
    settings.setValue("editor/font_size", m_fontSizeSpinner->value());
    settings.setValue("editor/show_line_numbers", m_lineNumbersCheckBox->isChecked());
    
    QJsonObject editorConfig;
    editorConfig["auto_save"] = m_autoSaveCheckBox->isChecked();
    editorConfig["tab_size"] = m_tabSizeSpinner->value();
    settings.setGroup("editor", editorConfig);
    
    bool saveResult = settings.save();
    qDebug() << "Результат сохранения настроек:" << (saveResult ? "успешно" : "с ошибкой");
    
    emit settingsChanged();
    
    accept();
}