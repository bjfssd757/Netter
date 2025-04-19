#include "sidebar.h"
#include <QVBoxLayout>
#include <QLabel>
#include <QIcon>
#include "../../core/settings_manager.h"

Sidebar::Sidebar(QWidget *parent)
    : QWidget(parent)
{
    QVBoxLayout *layout = new QVBoxLayout(this);
    
    QLabel *routesLabel = new QLabel("Routes", this);
    routesLabel->setStyleSheet("font-weight: bold; font-size: 14px;");
    
    m_routesModel = new QStandardItemModel(this);
    m_routesModel->setHorizontalHeaderLabels(QStringList() << "Route");
    
    m_routesTreeView = new QTreeView(this);
    m_routesTreeView->setModel(m_routesModel);
    m_routesTreeView->setHeaderHidden(true);
    
    m_addRouteButton = new QPushButton("Add Route", this);
    
    layout->addWidget(routesLabel);
    layout->addWidget(m_routesTreeView);
    layout->addWidget(m_addRouteButton);
    
    connect(m_routesTreeView, &QTreeView::clicked, this, &Sidebar::onRouteSelected);
    
    setFixedWidth(200);
    setObjectName("sidebarWidget");
    
    QStandardItem *apiItem = new QStandardItem("API");
    m_routesModel->appendRow(apiItem);
    apiItem->appendRow(new QStandardItem("/users"));
    apiItem->appendRow(new QStandardItem("/user/{id}"));
    apiItem->appendRow(new QStandardItem("/admin/{action}"));
    apiItem->appendRow(new QStandardItem("/complex"));
}

void Sidebar::updateRoutesList(const QStringList& routes)
{
    m_routesModel->clear();
    QStandardItem *apiItem = new QStandardItem("API");
    m_routesModel->appendRow(apiItem);
    
    foreach (const QString &route, routes) {
        apiItem->appendRow(new QStandardItem(route));
    }
    
    m_routesTreeView->expandAll();
}

void Sidebar::onRouteSelected(const QModelIndex& index)
{
    QStandardItem *item = m_routesModel->itemFromIndex(index);
    if (item && item->parent()) {
        emit routeSelected(item->text());
    }
}

void Sidebar::applySettings()
{
    JsonSettings& settings = JsonSettings::instance();
    QJsonObject sidebarConfig = settings.getGroup("ui").value("sidebar").toObject();
    
    if (sidebarConfig.contains("width")) {
        setFixedWidth(sidebarConfig["width"].toInt());
    }
    
    // There will be add other settings for sidebar
}