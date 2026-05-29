#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusPane {
    CollectionTree,
    Details,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiFrame {
    pub title: String,
    pub focus: FocusPane,
}

impl Default for UiFrame {
    fn default() -> Self {
        Self {
            title: "Brutui".to_string(),
            focus: FocusPane::CollectionTree,
        }
    }
}
