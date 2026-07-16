use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiDescriptor {
    pub assets_dir: String,
    pub dev_url: Option<String>,
    pub width: u32,
    pub height: u32,
    pub min_width: u32,
    pub min_height: u32,
    pub resizable: bool,
}

impl UiDescriptor {
    pub fn web_assets(assets_dir: impl Into<String>) -> Self {
        Self {
            assets_dir: assets_dir.into(),
            dev_url: None,
            width: 900,
            height: 560,
            min_width: 640,
            min_height: 420,
            resizable: true,
        }
    }

    pub fn with_dev_url(mut self, dev_url: impl Into<String>) -> Self {
        self.dev_url = Some(dev_url.into());
        self
    }

    #[must_use]
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    #[must_use]
    pub fn with_min_size(mut self, min_width: u32, min_height: u32) -> Self {
        self.min_width = min_width;
        self.min_height = min_height;
        self
    }

    #[must_use]
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
}
