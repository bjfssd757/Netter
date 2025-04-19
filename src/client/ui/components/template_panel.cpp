#include "template_panel.h"
#include <QVBoxLayout>
#include <QDir>

TemplatePanel::TemplatePanel(QWidget *parent)
    : QWidget(parent)
{
    QVBoxLayout *layout = new QVBoxLayout(this);
    
    m_titleLabel = new QLabel("Templates", this);
    m_titleLabel->setStyleSheet("font-weight: bold; font-size: 14px;");
    
    m_templateList = new QListWidget(this);
    m_templateList->setIconSize(QSize(32, 32));
    
    layout->addWidget(m_titleLabel);
    layout->addWidget(m_templateList);
    
    loadTemplates();
    
    connect(m_templateList, &QListWidget::itemClicked, this, &TemplatePanel::onTemplateClicked);
    
    setObjectName("templatePanel");
}

void TemplatePanel::loadTemplates()
{    
    QDir templatesDir(":src/client/assets/templates");
    foreach (const QString &fileName, templatesDir.entryList(QDir::Files)) {
        if (fileName.endsWith(".rd")) {
            m_templateList->addItem(fileName.left(fileName.lastIndexOf(".")));
        }
    }
}

void TemplatePanel::onTemplateClicked(QListWidgetItem *item)
{
    emit templateSelected(item->text());
}