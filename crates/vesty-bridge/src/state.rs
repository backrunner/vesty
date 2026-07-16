use serde_json::Value;
use vesty_ipc::PluginSnapshot;

#[derive(Clone, Debug)]
pub struct BridgeStateStore {
    snapshot: PluginSnapshot,
}

impl BridgeStateStore {
    pub fn new(snapshot: PluginSnapshot) -> Self {
        Self { snapshot }
    }

    pub fn snapshot(&self) -> &PluginSnapshot {
        &self.snapshot
    }

    pub fn replace_snapshot(&mut self, snapshot: PluginSnapshot) {
        self.snapshot = snapshot;
    }

    pub fn set_config_value(&mut self, key: String, value: Value) {
        let mut config = self
            .snapshot
            .config
            .as_object()
            .cloned()
            .unwrap_or_default();
        config.insert(key, value);
        self.snapshot.config = Value::Object(config);
        self.snapshot.revision += 1;
        self.snapshot.config_revision += 1;
    }

    pub fn set_ui_state(&mut self, value: Value) {
        self.snapshot.ui_state = value;
        self.snapshot.revision += 1;
        self.snapshot.ui_revision += 1;
    }

    pub fn advance_params_revision(&mut self) {
        self.snapshot.revision += 1;
        self.snapshot.params_revision += 1;
    }

    pub fn config_entry_count(&self) -> usize {
        self.snapshot
            .config
            .as_object()
            .map_or(0, |config| config.len())
    }

    pub fn has_config_key(&self, key: &str) -> bool {
        self.snapshot
            .config
            .as_object()
            .is_some_and(|config| config.contains_key(key))
    }
}
