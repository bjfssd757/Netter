#ifndef TEMPLATE_PANEL_H
#define TEMPLATE_PANEL_H

#include <QWidget>
#include <QListWidget>
#include <QLabel>

class TemplatePanel : public QWidget
{
    Q_OBJECT
    
public:
    explicit TemplatePanel(QWidget *parent = nullptr);
    
signals:
    void templateSelected(const QString& templateName);
    
private slots:
    void onTemplateClicked(QListWidgetItem *item);
    
private:
    void loadTemplates();
    
    QLabel *m_titleLabel;
    QListWidget *m_templateList;
};

#endif // TEMPLATE_PANEL_H