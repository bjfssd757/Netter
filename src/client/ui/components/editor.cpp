#include "editor.h"
#include <QFont>
#include <QFontMetrics>
#include <QPainter>
#include <QTextBlock>
#include <QKeyEvent>
#include <QDebug>
#include <QStyle>
#include "line_number_area.h"
#include "../../core/settings_manager.h"

Editor::Editor(QWidget *parent)
    : QPlainTextEdit(parent)
    , m_backgroundColor(Qt::white)
    , m_defaultTextColor(Qt::black)
    , m_keywordColor(Qt::blue)
    , m_classColor(QColor("#267f99"))
    , m_functionColor(QColor("#795e26"))
    , m_quotationColor(QColor("#a31515"))
    , m_commentColor(Qt::darkGreen)
    , m_numberColor(QColor("#098658"))
    , m_operatorColor(Qt::black)
    , m_currentLineColor(QColor("#f8f8f8"))
    , m_highlightCurrentLine(true)
{
    setObjectName("codeEditor");

    QFont font("Consolas", 11);
    font.setFixedPitch(true);
    setFont(font);
    
    setLineWrapMode(QPlainTextEdit::NoWrap);
    
    setTabStopDistance(4 * fontMetrics().horizontalAdvance(' '));
    
    m_highlighter = new RDSyntaxHighlighter(document());
    
    m_lineNumberArea = new LineNumberArea(this);
    m_lineNumberArea->setEditor(this);
    
    connect(this, &QPlainTextEdit::blockCountChanged, this, &Editor::updateLineNumberAreaWidth);
    connect(this, &QPlainTextEdit::updateRequest, this, &Editor::updateLineNumberArea);
    connect(this, &QPlainTextEdit::cursorPositionChanged, this, &Editor::highlightCurrentLine);
    connect(this, &QPlainTextEdit::cursorPositionChanged, this, &Editor::highlightMatchingBraces);
    
    updateLineNumberAreaWidth(0);
    highlightCurrentLine();
    
    setObjectName("codeEditor");
}

void Editor::setBackgroundColor(const QColor& color)
{
    if (m_backgroundColor != color) {
        m_backgroundColor = color;
        
        QPalette pal = palette();
        pal.setColor(QPalette::Base, m_backgroundColor);
        setPalette(pal);
    }
}

void Editor::setDefaultTextColor(const QColor& color)
{
    if (m_defaultTextColor != color) {
        m_defaultTextColor = color;
        
        QPalette pal = palette();
        pal.setColor(QPalette::Text, m_defaultTextColor);
        setPalette(pal);
        
        updateSyntaxHighlighter();
    }
}

void Editor::setKeywordColor(const QColor& color) 
{ 
    if (m_keywordColor != color) {
        m_keywordColor = color;
        updateSyntaxHighlighter();
    }
}

void Editor::setClassColor(const QColor& color) 
{ 
    if (m_classColor != color) {
        m_classColor = color;
        updateSyntaxHighlighter();
    }
}

void Editor::setFunctionColor(const QColor& color)
{ 
    if (m_functionColor != color) {
        m_functionColor = color;
        updateSyntaxHighlighter();
    }
}

void Editor::setQuotationColor(const QColor& color) 
{ 
    if (m_quotationColor != color) {
        m_quotationColor = color;
        updateSyntaxHighlighter();
    }
}

void Editor::setCommentColor(const QColor& color) 
{ 
    if (m_commentColor != color) {
        m_commentColor = color;
        updateSyntaxHighlighter();
    }
}

void Editor::setNumberColor(const QColor& color) 
{ 
    if (m_numberColor != color) {
        m_numberColor = color;
        updateSyntaxHighlighter();
    }
}

void Editor::setOperatorColor(const QColor& color) 
{ 
    if (m_operatorColor != color) {
        m_operatorColor = color;
        updateSyntaxHighlighter();
    }
}

void Editor::setCurrentLineColor(const QColor& color)
{
    if (m_currentLineColor != color) {
        m_currentLineColor = color;
        highlightCurrentLine();
    }
}

void Editor::setCurrentLineHighlightEnabled(bool enabled)
{
    if (m_highlightCurrentLine != enabled) {
        m_highlightCurrentLine = enabled;
        
        setProperty("currentLineHighlightEnabled", enabled);
        
        if (enabled) {
            connect(this, &QPlainTextEdit::cursorPositionChanged, 
                    this, &Editor::highlightCurrentLine);
            highlightCurrentLine();
        } else {
            disconnect(this, &QPlainTextEdit::cursorPositionChanged, 
                      this, &Editor::highlightCurrentLine);
            QList<QTextEdit::ExtraSelection> extraSelections;
            setExtraSelections(extraSelections);
        }
    }
}

int Editor::lineNumberAreaWidth() const
{
    int digits = 1;
    int max = qMax(1, blockCount());
    while (max >= 10) {
        max /= 10;
        ++digits;
    }
    
    int space = 3 + fontMetrics().horizontalAdvance(QLatin1Char('9')) * digits;
    
    return space;
}

void Editor::updateLineNumberAreaWidth(int /* newBlockCount */)
{
    setViewportMargins(lineNumberAreaWidth(), 0, 0, 0);
}

void Editor::updateLineNumberArea(const QRect &rect, int dy)
{
    if (dy)
        m_lineNumberArea->scroll(0, dy);
    else
        m_lineNumberArea->update(0, rect.y(), m_lineNumberArea->width(), rect.height());
    
    if (rect.contains(viewport()->rect()))
        updateLineNumberAreaWidth(0);
}

void Editor::resizeEvent(QResizeEvent *e)
{
    QPlainTextEdit::resizeEvent(e);
    
    QRect cr = contentsRect();
    m_lineNumberArea->setGeometry(QRect(cr.left(), cr.top(), lineNumberAreaWidth(), cr.height()));
}

void Editor::lineNumberAreaPaintEvent(QPaintEvent *event)
{
    QPainter painter(m_lineNumberArea);
    QColor color = QColor(Qt::blue);
    color.setAlpha(10);
    painter.fillRect(event->rect(), color);
    
    QTextBlock block = firstVisibleBlock();
    int blockNumber = block.blockNumber();
    int top = qRound(blockBoundingGeometry(block).translated(contentOffset()).top());
    int bottom = top + qRound(blockBoundingRect(block).height());
    
    while (block.isValid() && top <= event->rect().bottom()) {
        if (block.isVisible() && bottom >= event->rect().top()) {
            QString number = QString::number(blockNumber + 1);
            painter.setPen(QColor(120, 120, 120));
            painter.drawText(0, top, m_lineNumberArea->width() - 2, fontMetrics().height(),
                             Qt::AlignRight, number);
        }
        
        block = block.next();
        top = bottom;
        bottom = top + qRound(blockBoundingRect(block).height());
        ++blockNumber;
    }
}

void Editor::highlightCurrentLine()
{
    if (!m_highlightCurrentLine)
        return;
    
    QList<QTextEdit::ExtraSelection> extraSelections;
    
    if (!isReadOnly()) {
        QTextEdit::ExtraSelection selection;
        
        selection.format.setBackground(m_currentLineColor);
        selection.format.setProperty(QTextFormat::FullWidthSelection, true);
        selection.cursor = textCursor();
        selection.cursor.clearSelection();
        extraSelections.append(selection);
    }
    
    setExtraSelections(extraSelections);
}

void Editor::updateSyntaxHighlighter()
{
    if (m_highlighter) {
        m_highlighter->updateFormats(
            m_keywordColor,
            m_classColor,
            m_functionColor,
            m_quotationColor,
            m_commentColor,
            m_numberColor,
            m_operatorColor
        );
    }
}

void Editor::highlightMatchingBraces()
{
    QList<QTextEdit::ExtraSelection> selections = extraSelections();
    
    selections.erase(
        std::remove_if(
            selections.begin(), 
            selections.end(),
            [](const QTextEdit::ExtraSelection& sel) {
                return sel.format.background() == QColor(Qt::lightGray).lighter(130);
            }
        ),
        selections.end()
    );

    QTextCursor cursor = textCursor();
    QTextDocument *document = this->document();
    
    QHash<QChar, QChar> matchingPairs;
    matchingPairs['{'] = '}';
    matchingPairs['['] = ']';
    matchingPairs['('] = ')';
    matchingPairs['}'] = '{';
    matchingPairs[']'] = '[';
    matchingPairs[')'] = '(';
    
    int pos = cursor.position();
    QChar currentChar, previousChar;
    
    if (pos < document->characterCount()) {
        currentChar = document->characterAt(pos);
    }
    if (pos > 0) {
        previousChar = document->characterAt(pos - 1);
    }
    
    auto highlightBracket = [&](int pos, const QChar& bracket) {
        QTextEdit::ExtraSelection selection;
        QTextCursor bracketCursor(document);
        bracketCursor.setPosition(pos);
        bracketCursor.movePosition(QTextCursor::NextCharacter, QTextCursor::KeepAnchor);
        selection.cursor = bracketCursor;
        selection.format.setBackground(QColor(Qt::lightGray));
        selections.append(selection);
    };
    
    auto findMatchingBracket = [&](int startPos, QChar openBracket, QChar closeBracket, int direction) -> int {
        int count = 1;
        int pos = startPos + direction;
        
        while (pos >= 0 && pos < document->characterCount()) {
            QChar ch = document->characterAt(pos);
            
            if (ch == openBracket) count++;
            else if (ch == closeBracket) count--;
            
            if (count == 0) return pos;
            pos += direction;
        }
        
        return -1;
    };
    
    if (matchingPairs.contains(currentChar)) {
        QChar matchingBracket = matchingPairs[currentChar];
        bool isOpenBracket = (currentChar == '{' || currentChar == '[' || currentChar == '(');
        
        int matchPos = isOpenBracket ? 
                      findMatchingBracket(pos, currentChar, matchingBracket, 1) : 
                      findMatchingBracket(pos, matchingBracket, currentChar, -1);
        
        if (matchPos != -1) {
            highlightBracket(pos, currentChar);
            highlightBracket(matchPos, document->characterAt(matchPos));
        }
    }
    
    if (matchingPairs.contains(previousChar)) {
        QChar matchingBracket = matchingPairs[previousChar];
        bool isOpenBracket = (previousChar == '{' || previousChar == '[' || previousChar == '(');
        
        int matchPos = isOpenBracket ? 
                      findMatchingBracket(pos-1, previousChar, matchingBracket, 1) : 
                      findMatchingBracket(pos-1, matchingBracket, previousChar, -1);
        
        if (matchPos != -1) {
            highlightBracket(pos-1, previousChar);
            highlightBracket(matchPos, document->characterAt(matchPos));
        }
    }
    
    setExtraSelections(selections);
}

void Editor::keyPressEvent(QKeyEvent *e)
{
    if (e->key() == Qt::Key_Return || e->key() == Qt::Key_Enter) {
        QTextCursor cursor = textCursor();
        int pos = cursor.position();
        int blockPos = cursor.block().position();
        QString text = cursor.block().text();
        int leftPos = pos - blockPos;
        
        int indent = 0;
        for (int i = 0; i < text.length() && text[i].isSpace(); ++i) {
            indent++;
        }
        
        QString indentStr = text.left(indent);
        bool addBrace = false;

        if (leftPos > 0 && text[leftPos - 1] == '{') {
            indentStr += "    ";
            addBrace = true;
        }
        
        QPlainTextEdit::keyPressEvent(e);

        cursor = textCursor();
        cursor.insertText(indentStr);

        if (addBrace) {
            int cursorPos = cursor.position();
            cursor.insertBlock();
            cursor.insertText(text.left(indent) + "}");
            cursor.setPosition(cursorPos);
            setTextCursor(cursor);
        }
    } else {
        QPlainTextEdit::keyPressEvent(e);
    }
}

void Editor::setTheme(const QString& themeName)
{
    style()->unpolish(this);
    style()->polish(this);
}

RDSyntaxHighlighter::RDSyntaxHighlighter(QTextDocument *parent)
    : QSyntaxHighlighter(parent)
{
    setDefaultFormats();
}

void RDSyntaxHighlighter::setDefaultFormats()
{
    m_highlightingRules.clear();
    
    // Keywords
    m_keywordFormat.setForeground(QColor("#6D6DDF").lighter(100));
    m_keywordFormat.setFontWeight(QFont::Bold);
    QStringList keywordPatterns;
    keywordPatterns << "\\broute\\b" << "\\bGET\\b" << "\\bPOST\\b" << "\\bPUT\\b"
                    << "\\bDELETE\\b" << "\\bif\\b" << "\\belse\\b" << "\\bval\\b"
                    << "\\bvar\\b" << "\\breturn\\b" << "\\bresponse\\b" << "\\brequest\\b";

    foreach (const QString &pattern, keywordPatterns) {
        HighlightingRule rule;
        rule.pattern = QRegularExpression(pattern);
        rule.format = m_keywordFormat;
        m_highlightingRules.append(rule);
    }

    // Strings
    m_stringFormat.setForeground(QColor("#E69917").lighter(100));
    HighlightingRule rule;
    rule.pattern = QRegularExpression("\".*?\"");
    rule.pattern.setPatternOptions(QRegularExpression::InvertedGreedinessOption);
    rule.format = m_stringFormat;
    m_highlightingRules.append(rule);

    // Comments
    m_commentFormat.setForeground(QColor("#5CE75C").lighter(150));
    rule.pattern = QRegularExpression("//[^\n]*");
    rule.format = m_commentFormat;
    m_highlightingRules.append(rule);

    rule.pattern = QRegularExpression("/\\*.*\\*/");
    rule.pattern.setPatternOptions(QRegularExpression::InvertedGreedinessOption);
    rule.format = m_commentFormat;
    m_highlightingRules.append(rule);

    // Functions
    m_functionFormat.setForeground(QColor("#DCE417"));
    rule.pattern = QRegularExpression("\\b[A-Za-z0-9_]+(?=\\()");
    rule.format = m_functionFormat;
    m_highlightingRules.append(rule);

    // Numbers
    m_numberFormat.setForeground(QColor("#8fbc8f"));
    QRegularExpression numberRegex("\\b[0-9]+\\b");
    HighlightingRule numberRule;
    numberRule.pattern = numberRegex;
    numberRule.format = m_numberFormat;
    m_highlightingRules.append(numberRule);

    // Operators
    m_operatorFormat.setForeground(QColor("#ace1af"));
    QRegularExpression operatorRegex("[\\+\\-\\*\\/\\=\\<\\>\\!\\&\\|\\^\\~\\%]");
    HighlightingRule operatorRule;
    operatorRule.pattern = operatorRegex;
    operatorRule.format = m_operatorFormat;
    m_highlightingRules.append(operatorRule);
}

void RDSyntaxHighlighter::setDarkThemeFormats()
{
    // Dark Theme
}

void RDSyntaxHighlighter::setSolarizedLightFormats()
{
    // Solarized Light темы
}

void RDSyntaxHighlighter::setSolarizedDarkFormats()
{
    // Solarized Dark темы
}

void RDSyntaxHighlighter::highlightBlock(const QString &text)
{
    foreach (const HighlightingRule &rule, m_highlightingRules) {
        QRegularExpression expression(rule.pattern);
        QRegularExpressionMatch match = expression.match(text);
        int index = match.capturedStart();
        while (index >= 0) {
            int length = match.capturedLength();
            setFormat(index, length, rule.format);
            match = expression.match(text, index + length);
            index = match.capturedStart();
        }
    }
}

void Editor::applySettings()
{
    JsonSettings& settings = JsonSettings::instance();
    QJsonObject editorConfig = settings.getGroup("editor");
    
    QString fontFamily = editorConfig.value("font_family").toString("Consolas");
    int fontSize = editorConfig.value("font_size").toInt(11);
    QFont font(fontFamily, fontSize);
    font.setFixedPitch(true);
    setFont(font);
    
    int tabSize = editorConfig.value("tab_size").toInt(4);
    setTabStopDistance(tabSize * fontMetrics().horizontalAdvance(' '));
    
    bool showLineNumbers = editorConfig.value("show_line_numbers").toBool(true);
    if (m_lineNumberArea) {
        m_lineNumberArea->setVisible(showLineNumbers);
        updateLineNumberAreaWidth(0);
    }
    
    bool highlightCurrentLine = editorConfig.value("highlight_current_line").toBool(true);
    setCurrentLineHighlightEnabled(highlightCurrentLine);
    
    bool readOnly = editorConfig.value("read_only").toBool(false);
    setReadOnly(readOnly);
    
    style()->unpolish(this);
    style()->polish(this);
}

void RDSyntaxHighlighter::updateFormats(
    const QColor& keywordColor,
    const QColor& classColor,
    const QColor& functionColor,
    const QColor& quotationColor,
    const QColor& commentColor,
    const QColor& numberColor,
    const QColor& operatorColor
)
{
    m_highlightingRules.clear();
    
    m_keywordFormat.setForeground(keywordColor);
    m_keywordFormat.setFontWeight(QFont::Bold);
    
    m_classFormat.setForeground(classColor);
    
    m_functionFormat.setForeground(functionColor);
    
    m_stringFormat.setForeground(quotationColor);
    
    m_commentFormat.setForeground(commentColor);
    m_commentFormat.setFontItalic(true);
    
    m_numberFormat.setForeground(numberColor);
    
    m_operatorFormat.setForeground(operatorColor);
    
    const QString keywordPatterns[] = {
        "\\bclass\\b", "\\bdef\\b", "\\breturn\\b", "\\bif\\b",
        "\\belse\\b", "\\bwhile\\b", "\\bfor\\b", "\\bval\\b",
        "\\broute\\b", "\\bGET\\b", "\\bPOST\\b", "\\bPUT\\b",
        "\\bDELETE\\b", "\\bresponse\\b", "\\brequest\\b",
        "\\bPATCH\\b", "\\bOPTIONS\\b", "\\bHEAD\\b"
    };
    
    for (const QString &pattern : keywordPatterns) {
        QRegularExpression expression(pattern);
        m_highlightingRules.append({expression, m_keywordFormat});
    }
    
    m_highlightingRules.append({QRegularExpression("\".*\""), m_stringFormat});
    m_highlightingRules.append({QRegularExpression("'.*'"), m_stringFormat});
    
    m_highlightingRules.append({QRegularExpression("//[^\n]*"), m_commentFormat});
    
    m_highlightingRules.append({QRegularExpression("\\b[A-Za-z0-9_]+(?=\\()"), m_functionFormat});
    
    m_highlightingRules.append({QRegularExpression("\\b\\d+\\b"), m_numberFormat});
    
    m_highlightingRules.append({QRegularExpression("[\\+\\-\\*\\/\\=\\!\\<\\>]"), m_operatorFormat});
    
    rehighlight();
}