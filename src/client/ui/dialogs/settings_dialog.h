#ifndef SETTINGS_DIALOG_H
#define SETTINGS_DIALOG_H

#include <QDialog>
#include <QLineEdit>
#include <QComboBox>
#include <QCheckBox>
#include <QPushButton>
#include <QSpinBox>

class SettingsDialog : public QDialog
{
    Q_OBJECT
    
public:
    explicit SettingsDialog(QWidget *parent = nullptr);

signals:
    void settingsChanged();
    
private slots:
    void onSaveSettings();
    
private:
    void loadSettings();
    
    QLineEdit *m_execPathEdit;
    QLineEdit *m_templatesPathEdit;
    QComboBox *m_themeComboBox;
    QCheckBox *m_autoSaveCheckBox;
    QPushButton *m_saveButton;
    QPushButton *m_cancelButton;
    QComboBox *m_editorThemeComboBox;
    QSpinBox *m_fontSizeSpinner;
    QCheckBox *m_lineNumbersCheckBox;
    QCheckBox *m_highlightLineCheckBox;
    QSpinBox *m_tabSizeSpinner;
};

#endif // SETTINGS_DIALOG_H