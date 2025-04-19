#ifndef EDITOR_H
#define EDITOR_H

#include <QPlainTextEdit>
#include <QSyntaxHighlighter>
#include <QRegularExpression>
#include <QColor>

class LineNumberArea;

class RDSyntaxHighlighter : public QSyntaxHighlighter
{
    Q_OBJECT

public:
    RDSyntaxHighlighter(QTextDocument *parent = nullptr);
    
    void setDefaultFormats();
    void setDarkThemeFormats();
    void setSolarizedLightFormats();
    void setSolarizedDarkFormats();

    void updateFormats(
        const QColor& keywordColor,
        const QColor& classColor,
        const QColor& functionColor,
        const QColor& quotationColor,
        const QColor& commentColor,
        const QColor& numberColor,
        const QColor& operatorColor
    );

protected:
    void highlightBlock(const QString &text) override;

private:
    struct HighlightingRule
    {
        QRegularExpression pattern;
        QTextCharFormat format;
    };
    QVector<HighlightingRule> m_highlightingRules;

    QTextCharFormat m_keywordFormat;
    QTextCharFormat m_stringFormat;
    QTextCharFormat m_commentFormat;
    QTextCharFormat m_functionFormat;
    QTextCharFormat m_numberFormat;
    QTextCharFormat m_operatorFormat;
    QTextCharFormat m_font;
    QTextFormat m_classFormat;
};

class Editor : public QPlainTextEdit
{
    Q_OBJECT

    Q_PROPERTY(QColor backgroundColor READ getBackgroundColor WRITE setBackgroundColor)
    Q_PROPERTY(QColor defaultTextColor READ getDefaultTextColor WRITE setDefaultTextColor)
    Q_PROPERTY(QColor keywordColor READ getKeywordColor WRITE setKeywordColor)
    Q_PROPERTY(QColor classColor READ getClassColor WRITE setClassColor)
    Q_PROPERTY(QColor functionColor READ getFunctionColor WRITE setFunctionColor)
    Q_PROPERTY(QColor quotationColor READ getQuotationColor WRITE setQuotationColor)
    Q_PROPERTY(QColor commentColor READ getCommentColor WRITE setCommentColor)
    Q_PROPERTY(QColor numberColor READ getNumberColor WRITE setNumberColor)
    Q_PROPERTY(QColor operatorColor READ getOperatorColor WRITE setOperatorColor)
    Q_PROPERTY(QColor currentLineColor READ getCurrentLineColor WRITE setCurrentLineColor)
    Q_PROPERTY(bool currentLineHighlightEnabled READ isCurrentLineHighlightEnabled WRITE setCurrentLineHighlightEnabled)
    
public:
    explicit Editor(QWidget *parent = nullptr);

    QColor getBackgroundColor() const { return m_backgroundColor; }
    void setBackgroundColor(const QColor& color);
    
    QColor getDefaultTextColor() const { return m_defaultTextColor; }
    void setDefaultTextColor(const QColor& color);
    
    QColor getKeywordColor() const { return m_keywordColor; }
    void setKeywordColor(const QColor& color);
    
    QColor getClassColor() const { return m_classColor; }
    void setClassColor(const QColor& color);
    
    QColor getFunctionColor() const { return m_functionColor; }
    void setFunctionColor(const QColor& color);
    
    QColor getQuotationColor() const { return m_quotationColor; }
    void setQuotationColor(const QColor& color);
    
    QColor getCommentColor() const { return m_commentColor; }
    void setCommentColor(const QColor& color);
    
    QColor getNumberColor() const { return m_numberColor; }
    void setNumberColor(const QColor& color);
    
    QColor getOperatorColor() const { return m_operatorColor; }
    void setOperatorColor(const QColor& color);
    
    QColor getCurrentLineColor() const { return m_currentLineColor; }
    void setCurrentLineColor(const QColor& color);
    
    bool isCurrentLineHighlightEnabled() const { return m_highlightCurrentLine; }
    void setCurrentLineHighlightEnabled(bool enabled);
    
    int lineNumberAreaWidth() const;
    void lineNumberAreaPaintEvent(QPaintEvent *event);
    
    void setTheme(const QString& themeName);
    void applySettings();

protected:
    void resizeEvent(QResizeEvent *event) override;
    void keyPressEvent(QKeyEvent *e) override;

private slots:
    void highlightCurrentLine();
    void highlightMatchingBraces();
    void updateLineNumberAreaWidth(int newBlockCount);
    void updateLineNumberArea(const QRect &rect, int dy);

private:
    RDSyntaxHighlighter *m_highlighter;
    LineNumberArea *m_lineNumberArea;

    QColor m_backgroundColor;
    QColor m_defaultTextColor;
    QColor m_keywordColor;
    QColor m_classColor;
    QColor m_functionColor;
    QColor m_quotationColor;
    QColor m_commentColor;
    QColor m_numberColor;
    QColor m_operatorColor;
    QColor m_currentLineColor;
    bool m_highlightCurrentLine;

    void updateSyntaxHighlighter();
};

#endif // EDITOR_H