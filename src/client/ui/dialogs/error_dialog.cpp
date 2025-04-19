#include "error_dialog.h"
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QIcon>

ErrorDialog::ErrorDialog(const QString& title, const QString& message, QWidget *parent)
    : QDialog(parent)
{
    setWindowTitle("Error");
    setFixedSize(400, 300);
    
    QVBoxLayout *mainLayout = new QVBoxLayout(this);
    mainLayout->setSpacing(15);
    
    QHBoxLayout *headerLayout = new QHBoxLayout();
    QLabel *iconLabel = new QLabel(this);
    iconLabel->setPixmap(QIcon::fromTheme("dialog-error").pixmap(32, 32));
    
    m_titleLabel = new QLabel(title, this);
    QFont titleFont = m_titleLabel->font();
    titleFont.setBold(true);
    titleFont.setPointSize(12);
    m_titleLabel->setFont(titleFont);
    
    headerLayout->addWidget(iconLabel);
    headerLayout->addWidget(m_titleLabel, 1);
    
    m_messageEdit = new QTextEdit(this);
    m_messageEdit->setReadOnly(true);
    m_messageEdit->setText(message);
    
    m_okButton = new QPushButton("OK", this);
    connect(m_okButton, &QPushButton::clicked, this, &QDialog::accept);
    
    QHBoxLayout *buttonLayout = new QHBoxLayout();
    buttonLayout->addStretch(1);
    buttonLayout->addWidget(m_okButton);
    
    mainLayout->addLayout(headerLayout);
    mainLayout->addWidget(m_messageEdit);
    mainLayout->addLayout(buttonLayout);
    
    setObjectName("errorDialog");
}