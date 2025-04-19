#ifndef SIDEBAR_H
#define SIDEBAR_H

#include <QWidget>
#include <QListWidget>
#include <QPushButton>
#include <QTreeView>
#include <QStandardItemModel>

class Sidebar : public QWidget
{
    Q_OBJECT
    
public:
    explicit Sidebar(QWidget *parent = nullptr);

    void applySettings();
    
public slots:
    void updateRoutesList(const QStringList& routes);
    
signals:
    void routeSelected(const QString& routePath);
    
private slots:
    void onRouteSelected(const QModelIndex& index);
    
private:
    QTreeView *m_routesTreeView;
    QStandardItemModel *m_routesModel;
    QPushButton *m_addRouteButton;
};

#endif // SIDEBAR_H