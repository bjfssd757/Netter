#include "headers/mainwindow.h"
#include <QGridLayout>
#include <QWidget>
#include "headers/Header.h"
#include "headers/cli.h"
#include <QLineEdit>
#include <QMessageBox>

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
{

    auto *centralWidget = new QWidget(this);
    setCentralWidget(centralWidget);

    auto *layout = new QGridLayout(this);
    auto *header = new Header(this);


    connect(header, &Header::homeClicked, [&]() {
        qDebug() << "Home clicked!";
    });

    connect(header, &Header::settingsClicked, [&]() {
        qDebug() << "Settings clicked!";
    });

    setLayout(layout);

    layout->addWidget(header);

    centralWidget->setLayout(layout);

    // Netter launcher;

    // launcher.setServerPath(":/src");

    // launcher.addParameter("--" + )
}

// class Button : public QWidget {
//     Q_OBJECT

// public:
//     explicit Button(QWidget *parent = nullptr) :
//         QWidget(parent) {
//         auto *lineEdit = new QLineEdit(this);
//         lineEdit->setPlaceholderText("Enter text");

//         auto *line = new QLineEdit(this);
//         auto *acceptLine = new QPushButton("Enter", this);

//         line->setPlaceholderText("Enter type server");
//         line->setPlaceholderText("Enter host");
//         line->setPlaceholderText("Enter port");
//         line->setPlaceholderText("Enter path to config file");

//         connect(acceptLine, &QPushButton::clicked,
//             this, Button::onButtonClicked);

//         connect(lineEdit, &QLineEdit::returnPressed,
//                 this, &Button::onButtonClicked);

//     }

// private slots:
//     void onButtonClicked() {
//         QString text = lineEdit->text();

//         if (!text.isEmpty()) {
//             QMessageBox::information(
//                 this, "Text input",
//                 "You entered: " + text);

//             processText(text);

//             printf(text.toStdString().c_str());

//             lineEdit->clear();
//         } else {
//             QMessageBox::warning(this,
//                 "Warning", "Text is empty");
//         }
//     }

//     static void processText(const QString &text) {
//         qDebug() << text;
//     }

// private:
//     QLineEdit *lineEdit;
// };