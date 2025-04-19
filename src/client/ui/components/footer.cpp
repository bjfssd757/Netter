#include "footer.h"
#include <QHBoxLayout>
#include "../../core/settings_manager.h"

Footer::Footer(QWidget *parent)
    : QWidget(parent)
{
    QHBoxLayout *layout = new QHBoxLayout(this);
    layout->setContentsMargins(10, 2, 10, 2);
    
    m_statusLabel = new QLabel("Ready", this);
    m_progressBar = new QProgressBar(this);
    m_progressBar->setRange(0, 100);
    m_progressBar->setValue(0);
    m_progressBar->setFixedWidth(150);
    m_progressBar->setVisible(false);
    
    layout->addWidget(m_statusLabel, 1);
    layout->addWidget(m_progressBar);
    
    setFixedHeight(25);
    setObjectName("footerWidget");
}

void Footer::showMessage(const QString& message)
{
    m_statusLabel->setText(message);
}

void Footer::showProgress(int value)
{
    if (value == 0) {
        m_progressBar->setVisible(false);
    } else {
        m_progressBar->setVisible(true);
        m_progressBar->setValue(value);
    }
}

void Footer::applySettings()
{
    JsonSettings& settings = JsonSettings::instance();
    QJsonObject footerConfig = settings.getGroup("ui").value("footer").toObject();
    
    // There will be settings for footer
}