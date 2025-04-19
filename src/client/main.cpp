#include "headers/mainwindow.h"
#include <QApplication>
#include <QFile>
#include <QString>
#include <QLatin1String>

int main(int argc, char *argv[])
{
    QApplication a(argc, argv);
    MainWindow w;

    QFile file(":/src/client/style.qss");
    if (!file.open(QFile::ReadOnly)) {
        std::printf("Error: %s\n", file.errorString().toStdString().c_str());
    }
    const QString styleSheet = QLatin1String(file.readAll());
    file.close();

    a.setStyleSheet(styleSheet);

    w.resize(800, 600);
    w.show();
    return a.exec();
}
