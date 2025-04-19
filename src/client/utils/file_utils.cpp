#include "file_utils.h"
#include <QFile>
#include <QTextStream>
#include <QFileInfo>
#include <QDir>

QString FileUtils::loadFromFile(const QString& path)
{
    QFile file(path);
    if (!file.exists()) {
        throw std::runtime_error("File does not exist: " + path.toStdString());
    }
    
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        throw std::runtime_error("Could not open file for reading: " + path.toStdString());
    }
    
    QTextStream in(&file);
    QString content = in.readAll();
    file.close();
    
    return content;
}

void FileUtils::saveToFile(const QString& path, const QString& content)
{
    // Создаем директорию, если она не существует
    QFileInfo fileInfo(path);
    QDir dir = fileInfo.dir();
    if (!dir.exists()) {
        if (!dir.mkpath(".")) {
            throw std::runtime_error("Could not create directory for file: " + path.toStdString());
        }
    }
    
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        throw std::runtime_error("Could not open file for writing: " + path.toStdString());
    }
    
    QTextStream out(&file);
    out << content;
    file.close();
}