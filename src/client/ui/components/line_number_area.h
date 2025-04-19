#ifndef LINE_NUMBER_AREA_H
#define LINE_NUMBER_AREA_H

#include <QWidget>
#include <QSize>

class Editor;

class LineNumberArea : public QWidget
{
public:
    LineNumberArea(QWidget *parent = nullptr);
    
    void setEditor(Editor *editor);
    
    QSize sizeHint() const override;

protected:
    void paintEvent(QPaintEvent *event) override;

private:
    Editor *m_editor;
};

#endif // LINE_NUMBER_AREA_H