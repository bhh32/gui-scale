use super::IsTab;
use cosmic::{
    widget::{button, Column, Row},
    Element
};

#[derive(Debug, Clone)]
pub enum TabBarMessage {
    TabSelected(String),
}

/// A generic TabBar that can hold multiple "tabs" and their content
pub struct TabBar<T: IsTab, Message: Clone + 'static> {
    pub tabs: Vec<(
        T,
        Box<dyn Fn() -> Element<'static, Message> + 'static>,
    )>,
}

impl<T: IsTab, Message: Clone + 'static> TabBar<T, Message> {
    pub fn new(
        default_tab: T,
        default_tab_content: impl Fn() -> Element<'static, Message> + 'static,
    ) -> Self {
        let mut tabs = Vec::new();
        let content_box: Box<dyn Fn() -> Element<'static, Message> + 'static> =
            Box::new(default_tab_content);
        tabs.push((default_tab, content_box));
        Self { tabs }
    }

    /// Push a new tab into the TabBar
    pub fn push(
        &mut self,
        tab: T,
        content: impl for<'a> Fn() -> Element<'static, Message> + 'static,
    ) {
        let content_box: Box<dyn Fn() -> Element<'static, Message> + 'static> =
            Box::new(content);
        self.tabs.push((tab, content_box));
    }

    /// View the TabBar as an Element
    pub fn view<F>(&self, on_select: F) -> Element<Message>
    where
        F: Fn(String) -> Message + 'static + Copy,
        Message: Clone,
    {
        let mut column = Column::new();

        // Add tab buttons
        let mut button_row = Row::new().spacing(10);
        for (tab, _) in &self.tabs {
            let label = tab.title();
            button_row = button_row.push(button::standard(label).on_press(on_select(tab.title())));
        }
        column = column.push(button_row);

        // Show content of active tab only
        for (tab, content) in &self.tabs {
            if tab.is_active() {
                column = column.push(content());
                break;
            }
        }

        column.into()
    }
}
