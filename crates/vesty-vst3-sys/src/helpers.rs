use sha2::{Digest, Sha256};
use std::path::Path;

use crate::*;

pub(crate) fn generated_bindings_module_path_check(path: &Path) -> GeneratedBindingsPlanCheck {
    let extension_ok = path.extension().and_then(|extension| extension.to_str()) == Some("rs");
    let file_name_ok = path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .is_some();
    if extension_ok && file_name_ok {
        GeneratedBindingsPlanCheck {
            name: "bindings module path".to_string(),
            status: "ok".to_string(),
            value: path.display().to_string(),
            hint: None,
        }
    } else {
        GeneratedBindingsPlanCheck {
            name: "bindings module path".to_string(),
            status: "failed".to_string(),
            value: format!(
                "bindings module path must name a Rust `.rs` file: {}",
                path.display()
            ),
            hint: Some("use a path such as target/vst3-sdk/generated.rs".to_string()),
        }
    }
}

pub(crate) fn interface_methods(interface: &str) -> Vec<&'static BindingInterfaceMethodSpec> {
    GENERATED_BINDINGS_INTERFACE_METHODS
        .iter()
        .filter(|method| method.interface == interface)
        .collect()
}

pub(crate) fn interface_methods_const_name(interface: &str) -> String {
    format!("{}_METHODS", rust_identifier_constant_fragment(interface))
}

pub(crate) fn interface_vtable_slots_const_name(interface: &str) -> String {
    format!(
        "{}_VTABLE_SLOTS",
        rust_identifier_constant_fragment(interface)
    )
}

pub(crate) fn interface_callback_types_const_name(interface: &str) -> String {
    format!(
        "{}_CALLBACK_TYPES",
        rust_identifier_constant_fragment(interface)
    )
}

pub(crate) fn interface_vtable_field_offsets_const_name(interface: &str) -> String {
    format!(
        "{}_VTABLE_FIELD_OFFSETS",
        rust_identifier_constant_fragment(interface)
    )
}

pub(crate) fn interface_iid_const_name(interface: &str) -> String {
    format!("{}_IID", rust_identifier_constant_fragment(interface))
}

pub(crate) fn com_object_interfaces_const_name(object: &str) -> String {
    format!("{}_INTERFACES", rust_identifier_constant_fragment(object))
}

pub(crate) fn com_object_identity_spec(
    object: &str,
) -> Option<&'static BindingComObjectIdentitySpec> {
    GENERATED_BINDINGS_COM_OBJECT_IDENTITIES
        .iter()
        .find(|spec| spec.object == object)
}

pub(crate) fn com_object_identity_plan_const_name(object: &str) -> String {
    format!(
        "{}_IDENTITY_PLAN",
        rust_identifier_constant_fragment(object)
    )
}

pub(crate) fn com_object_query_interface_dispatch_const_name(object: &str) -> String {
    format!(
        "{}_QUERY_INTERFACE_DISPATCH",
        rust_identifier_constant_fragment(object)
    )
}

pub(crate) fn factory_class_plan_const_name(class_object: &str) -> String {
    format!(
        "{}_FACTORY_CLASS_PLAN",
        rust_identifier_constant_fragment(class_object)
    )
}

pub(crate) fn interface_method_count_const_name(interface: &str) -> String {
    format!(
        "{}_METHOD_COUNT",
        rust_identifier_constant_fragment(interface)
    )
}

pub(crate) fn rust_identifier_constant_fragment(value: &str) -> String {
    let mut fragment = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            fragment.push(character.to_ascii_uppercase());
        } else {
            fragment.push('_');
        }
    }
    fragment
}

pub(crate) fn rust_string_literal(value: &str) -> String {
    let mut literal = String::from("\"");
    for character in value.chars() {
        literal.extend(character.escape_default());
    }
    literal.push('"');
    literal
}

pub(crate) fn contains_identifier_token(text: &str, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    text.match_indices(token).any(|(start, _)| {
        let before = start
            .checked_sub(1)
            .and_then(|index| text.as_bytes().get(index))
            .copied();
        let after = text.as_bytes().get(start + token.len()).copied();
        !before.is_some_and(is_identifier_byte) && !after.is_some_and(is_identifier_byte)
    })
}

pub(crate) fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn sdk_version_hint(root: &Path) -> Option<String> {
    for relative in SDK_VERSION_HINT_FILES {
        let path = root.join(relative);
        if !path_is_regular_file_no_symlink(&path) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        if text.contains(STEINBERG_VST3_SDK_BASELINE) {
            return Some(format!("{relative} mentions {STEINBERG_VST3_SDK_BASELINE}"));
        }
        if text.contains("3.8.0") || text.contains("3.8") {
            return Some(format!("{relative} mentions VST3 SDK 3.8"));
        }
    }
    None
}
