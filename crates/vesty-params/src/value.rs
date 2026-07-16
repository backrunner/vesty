use crate::{ParamKind, ParamSpec};

pub fn normalized_to_plain(spec: &ParamSpec, normalized: f64) -> f64 {
    match spec.kind {
        ParamKind::Float { min, max } => min + normalized.clamp(0.0, 1.0) * (max - min),
        ParamKind::Bool => {
            if normalized >= 0.5 {
                1.0
            } else {
                0.0
            }
        }
        ParamKind::Choice { ref values } => {
            if values.len() <= 1 {
                0.0
            } else {
                choice_index_from_normalized(values.len(), normalized) as f64
            }
        }
    }
}

pub fn plain_to_normalized(spec: &ParamSpec, plain: f64) -> f64 {
    match spec.kind {
        ParamKind::Float { min, max } => {
            if (max - min).abs() <= f64::EPSILON {
                0.0
            } else {
                ((plain - min) / (max - min)).clamp(0.0, 1.0)
            }
        }
        ParamKind::Bool => {
            if plain >= 0.5 {
                1.0
            } else {
                0.0
            }
        }
        ParamKind::Choice { ref values } => {
            if values.len() <= 1 {
                0.0
            } else {
                (plain.round() / (values.len() as f64 - 1.0)).clamp(0.0, 1.0)
            }
        }
    }
}

pub fn format_normalized_value(spec: &ParamSpec, normalized: f64) -> String {
    if let ParamKind::Choice { values } = &spec.kind {
        let index = choice_index_from_normalized(values.len(), normalized);
        return values
            .get(index)
            .cloned()
            .unwrap_or_else(|| index.to_string());
    }

    let plain = normalized_to_plain(spec, normalized);
    match &spec.unit {
        Some(unit) if !unit.is_empty() => format!("{plain:.3} {unit}"),
        _ => format!("{plain:.3}"),
    }
}

pub fn parse_normalized_value(spec: &ParamSpec, text: &str) -> Option<f64> {
    let text = text.trim();
    if let ParamKind::Choice { values } = &spec.kind
        && let Some((index, _)) = values
            .iter()
            .enumerate()
            .find(|(_, value)| value.eq_ignore_ascii_case(text))
    {
        return Some(normalized_for_choice_index(values.len(), index));
    }

    let text = spec
        .unit
        .as_deref()
        .and_then(|unit| text.strip_suffix(unit))
        .unwrap_or(text)
        .trim();
    let plain = text.parse::<f64>().ok()?;
    Some(plain_to_normalized(spec, plain))
}

pub(crate) fn choice_index_from_normalized(values_len: usize, normalized: f64) -> usize {
    if values_len <= 1 {
        0
    } else {
        (normalized.clamp(0.0, 1.0) * (values_len as f64 - 1.0)).round() as usize
    }
}

pub(crate) fn normalized_for_choice_index(values_len: usize, index: usize) -> f64 {
    if values_len <= 1 {
        0.0
    } else {
        index.min(values_len - 1) as f64 / (values_len as f64 - 1.0)
    }
}
