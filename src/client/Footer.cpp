#include "headers/footer.h"
#include <QIcon>
#include <QDesktopServices>
#include <QHBoxLayout>
#include <QUrl>

Footer::Footer(QWidget *parent)
    :QWidget(parent)
{
    setAttribute(Qt::WA_StyledBackground, true);

    github = new QPushButton("Github", this);
    discord = new QPushButton("discord", this);

    auto *layout = new QHBoxLayout(this);

    github->setIcon(QIcon(":/images/github_icon.png"));
    github->setIconSize(QSize(32, 32));
    github->setFlat(true);
    github->setCursor(Qt::PointingHandCursor);

    discord->setIcon(QIcon(":/images/discord_icon.png"));
    discord->setIconSize(QSize(32, 32));
    discord->setFlat(true);
    discord->setCursor(Qt::PointingHandCursor);

    layout->addWidget(github, 1);
    layout->addWidget(discord, 2);

    connect(github, &QPushButton::clicked, this, &Footer::githubOpenLink);
}

void Footer::githubOpenLink() {
    QDesktopServices::openUrl(QUrl("https://github.com/bjfssd757/Netter"));
}