pub mod tab_bar;

use cosmic::Element;

pub(crate) trait IsTab {
    fn new(title: impl Into<String>) -> Self;
    fn title(&self) -> String;
    fn is_active(&self) -> bool;
    fn active(&mut self, active: bool);
}

#[derive(Debug, Clone)]
pub struct Tab {
    pub title: String,
    pub active: bool,
}

impl Tab {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            active: false,
        }
    }
}

impl IsTab for Tab {
    fn new(title: impl Into<String>) -> Self {
        Tab::new(title)
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn active(&mut self, active: bool) {
        self.active = active;
    }
}

impl Tab {
    pub fn content<Message>(
        &self,
        content: Element<'static, Message>,
    ) -> Element<'static, Message> {
        content.into()
    }
}
