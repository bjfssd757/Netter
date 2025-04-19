#ifndef FOOTER_H
#define FOOTER_H

#include <QPushButton>
#include <QWidget>

class Footer : public QWidget {

    Q_OBJECT

public:
    explicit Footer(QWidget *parent = nullptr);

private slots:
    static void githubOpenLink();

private:
    QPushButton *github;
    QPushButton *discord;
    std::string *githubLink;
    std::string *discordLink;
    std::string *githubIcon;
    std::string *discordIcon;

};

#endif //FOOTER_H
