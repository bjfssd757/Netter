#ifndef FILE_UTILS_H
#define FILE_UTILS_H

#include <QString>
#include <stdexcept>

class FileUtils
{
public:
    static QString loadFromFile(const QString& path);
    static void saveToFile(const QString& path, const QString& content);
};

#endif // FILE_UTILS_H