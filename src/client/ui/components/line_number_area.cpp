#include "line_number_area.h"
#include "editor.h"

LineNumberArea::LineNumberArea(QWidget *parent)
    : QWidget(parent), m_editor(nullptr)
{
}

void LineNumberArea::setEditor(Editor *editor)
{
    m_editor = editor;
    setParent(editor);
}

QSize LineNumberArea::sizeHint() const
{
    if (m_editor)
        return QSize(m_editor->lineNumberAreaWidth(), 0);
    return QSize(0, 0);
}

void LineNumberArea::paintEvent(QPaintEvent *event)
{
    if (m_editor)
        m_editor->lineNumberAreaPaintEvent(event);
}