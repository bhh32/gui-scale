use super::IsTab;
use iced::widget::{Button, Column, Row, Text};
use iced::{Element, Theme};

#[derive(Debug, Clone)]
pub enum TabBarMessage {
    TabSelected(String),
}

pub struct TabBar<T: IsTab, Message: Clone + 'static> {
    pub tabs: Vec<(
        T,
        Box<dyn Fn() -> Element<'static, Message, Theme> + 'static>,
    )>,
}

impl<T: IsTab, Message: Clone + 'static> TabBar<T, Message> {
    pub fn new(
        default_tab: T,
        default_tab_content: impl Fn() -> Element<'static, Message, Theme> + 'static,
    ) -> Self {
        let mut tabs = Vec::new();
        let content_box: Box<dyn Fn() -> Element<'static, Message, Theme> + 'static> =
            Box::new(default_tab_content);
        tabs.push((default_tab, content_box));
        Self { tabs }
    }

    pub fn push(
        &mut self,
        tab: T,
        content: impl for<'a> Fn() -> Element<'static, Message> + 'static,
    ) {
        self.tabs.push((tab, Box::new(content)));
    }

    pub fn view<F>(&self, on_select: F) -> Element<Message>
    where
        F: Fn(String) -> Message + 'static + Copy,
        Message: Clone,
    {
        let mut column = Column::new();

        // Add tab buttons
        let tab_buttons = self
            .tabs
            .iter()
            .fold(Row::new().spacing(10), |row, (tab, _)| {
                let label = if tab.is_active() {
                    format!("{}", tab.title())
                } else {
                    tab.title()
                };

                row.push(Button::new(Text::new(label)).on_press(on_select(tab.title())))
            });

        column = column.push(tab_buttons);

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
