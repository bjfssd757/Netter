#include "headers/Header.h"

Header::Header(QWidget *parent)
    :QWidget(parent)
{
    setAttribute(Qt::WA_StyledBackground, true);

    home = new QPushButton("Home", this);
    settings = new QPushButton("Settings", this);

    auto *layout = new QVBoxLayout(this);
    layout -> addWidget(home);
    layout -> addWidget(settings);

    connect(home, &QPushButton::clicked, this, &Header::homeClicked);
    connect(settings, &QPushButton::clicked, this, &Header::settingsClicked);
}
