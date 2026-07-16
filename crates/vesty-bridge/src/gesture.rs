#[derive(Clone, Debug, PartialEq)]
pub enum ParamGesturePhase {
    Begin,
    Perform,
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParamGesture {
    pub phase: ParamGesturePhase,
    pub id: String,
    pub normalized: Option<f64>,
    pub gesture_id: Option<String>,
    pub request_ids: Vec<String>,
}
