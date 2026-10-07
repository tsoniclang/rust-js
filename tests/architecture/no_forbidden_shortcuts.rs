use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_PATTERNS: &[&str] = &[
    "quickjs",
    "rquickjs",
    "v8",
    "boa_engine",
    "std::process::Command::new(\"node\")",
    "std::process::Command::new(\"npm\")",
    "std::process::Command::new(\"npx\")",
    "std::process::Command::new(\"tsx\")",
    "Command::new(\"node\")",
    "Command::new(\"npm\")",
    "Command::new(\"npx\")",
    "Command::new(\"tsx\")",
    "Any",
    "TypeId",
    "downcast",
];

#[test]
fn no_forbidden_shortcuts_present_in_product_sources() {
    let workspace_root = locate_workspace_root()
        .unwrap_or_else(|| panic!("unable to locate workspace root from test manifest directory"));
    let mut violations = Vec::new();
    let mut rust_files = Vec::new();
    collect_workspace_product_sources(&workspace_root, &mut rust_files);

    for file in rust_files {
        let source = read_source_file(&file);
        let relative = file
            .strip_prefix(&workspace_root)
            .expect("product source path");
        for forbidden in product_source_violations(relative, &source) {
            violations.push(format!("{}: contains `{}`", file.display(), forbidden));
        }
    }

    if !violations.is_empty() {
        let mut message = String::from("forbidden shortcuts found:\n");
        for violation in violations {
            message.push_str(" - ");
            message.push_str(&violation);
            message.push('\n');
        }
        panic!("{}", message);
    }
}

#[test]
fn native_array_erasure_is_one_sealed_checked_storage_owner() {
    let workspace_root = locate_workspace_root().expect("runtime workspace");
    let source = read_source_file(
        &workspace_root.join("crates/tsonic_rust_js/src/array/js_array/erasure.rs"),
    );
    assert!(source
        .contains("trait NativeArrayStorage: Any + tsonic_rust_runtime::ObjectIdentityCarrier"));
    assert!(!source.contains("pub trait NativeArrayStorage"));
    assert!(source.contains("downcast::<JsArrayOwner<Value>>()"));
    assert!(source.contains("downcast_ref::<JsArrayOwner<Value>>()"));
    assert_eq!(source.matches(".downcast").count(), 2);
    assert!(!source.contains("unsafe"));
    assert!(!source.contains("TypeId"));
    assert!(!source.contains("Box<"));
    assert!(!source.contains("Rc::new"));
}

#[test]
fn native_payload_reuses_runtime_without_a_local_erasure_contract() {
    let root = locate_workspace_root().expect("runtime workspace");
    let native = read_source_file(&root.join("crates/tsonic_rust_js/src/value/native.rs"));
    let values = read_source_file(&root.join("crates/tsonic_rust_js/src/value/mod.rs"));
    assert!(native.contains("NativePayload::from_closed(value)"));
    assert!(native.contains("value.native_value()"));
    assert!(values.contains("Native(NativePayload)"));
    assert!(!native.contains("struct NativeValue"));
    assert!(!values.contains("fn native_value"));
    for (relative, source) in [
        ("crates/tsonic_rust_js/src/value/native.rs", native),
        ("crates/tsonic_rust_js/src/value/mod.rs", values),
        (
            "crates/tsonic_rust_js/src/array/js_array/erasure.rs",
            read_source_file(&root.join("crates/tsonic_rust_js/src/array/js_array/erasure.rs")),
        ),
    ] {
        let path = Path::new(relative);
        assert!(product_source_violations(path, &source).is_empty());
        for injected in [
            "use core::any::Any;",
            "use std::{any::{Any as Native}, fmt};",
            "value.downcast_ref::<String>();",
        ] {
            let mutated = format!("{source}\n{injected}");
            assert!(!product_source_violations(path, &mutated).is_empty());
        }
    }
}

#[test]
fn any_import_spellings_and_mutated_array_recovery_are_rejected() {
    for source in [
        "use std::any::Any;",
        "use core::any::Any;",
        "use core::{any::Any, fmt};",
        "use std::{any::{Any as Native}, fmt};",
        "use core::any::{type_name, Any as Native};",
        "use core::any::*;",
        "use core::any as native;",
        "use core::{any::{*}, fmt};",
        "use std::{any as native, fmt};",
        "fn reflect(value: &dyn Any) {}",
        "value.downcast_ref::<u64>()",
    ] {
        assert!(!find_forbidden_patterns(source).is_empty());
        assert!(!product_source_violations(Path::new("unowned.rs"), source).is_empty());
    }
    assert!(find_forbidden_patterns("enum Property { Any, Assigned }").is_empty());
    assert!(find_forbidden_patterns("Property::Any").is_empty());
    assert!(find_forbidden_patterns("fn valid(value: &dyn Anything) {}").is_empty());
    assert!(find_forbidden_patterns("fn valid<Value: AnySuffix>() {}").is_empty());
    assert!(find_forbidden_patterns("company::Any").is_empty());
    assert!(find_forbidden_patterns("/// Any ByteSet may match a single char.").is_empty());
    assert!(find_forbidden_patterns("core::any::type_name::<u64>()").is_empty());
    let root = locate_workspace_root().expect("runtime workspace");
    let path = Path::new("crates/tsonic_rust_js/src/array/js_array/erasure.rs");
    let source = read_source_file(&root.join(path));
    for mutated in [
        source.replace("trait NativeArrayStorage:", "pub trait NativeArrayStorage:"),
        source.replace("downcast::<JsArrayOwner<Value>>()", "downcast::<String>()"),
        source.replace(
            "downcast_ref::<JsArrayOwner<Value>>()",
            "downcast_ref::<String>()",
        ),
    ] {
        assert!(!product_source_violations(path, &mutated).is_empty());
    }
}

#[test]
fn no_forbidden_shortcuts_in_fixture_text() {
    let source = r#"
        let code = std::process::Command::new("node").arg("--version").spawn();
    "#;
    let hits = find_forbidden_patterns(source);
    assert!(!hits.is_empty());
}

#[test]
fn allowlisted_name_occurrences_are_not_flagged_by_scanner() {
    let source = r#"
        use tsonic_rust_node::error::NodeError;
        let kind = "node";
        let module = "tsonic_rust_node";
        let node_error = NodeError::new("E001", "node sample");
        let class = "NodeError";
        assert!(!kind.is_empty() && !module.is_empty() && !class.is_empty());
        assert!(!node_error.code().is_empty());
    "#;
    let hits = find_forbidden_patterns(source);
    assert!(hits.is_empty());
}

fn find_forbidden_patterns(source: &str) -> Vec<&'static str> {
    FORBIDDEN_PATTERNS
        .iter()
        .copied()
        .filter(|pattern| {
            if *pattern == "Any" {
                contains_native_any(source)
            } else {
                source.contains(pattern)
            }
        })
        .collect()
}

fn contains_native_any(source: &str) -> bool {
    let compact: String = source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    compact.contains("any::*")
        || compact.contains("::anyas")
        || compact.contains("{anyas")
        || compact.contains(",anyas")
        || compact.split("any::{").skip(1).any(|group| {
            group
                .split('}')
                .next()
                .unwrap_or_default()
                .split(',')
                .any(|item| item == "*")
        })
        || source.match_indices("Any").any(|(index, _)| {
            let before = &source[..index];
            let after = &source[index + "Any".len()..];
            let identifier = |character: char| character.is_alphanumeric() || character == '_';
            if before.chars().next_back().is_some_and(identifier)
                || after.chars().next().is_some_and(identifier)
            {
                return false;
            }
            let prefix: String = before
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect();
            let namespace = prefix
                .strip_suffix("any::")
                .is_some_and(|owner| !owner.chars().next_back().is_some_and(identifier));
            namespace
                || prefix.ends_with("dyn")
                || prefix.ends_with('+')
                || (prefix.ends_with(':') && !prefix.ends_with("::"))
                || prefix
                    .rsplit_once("any::{")
                    .is_some_and(|(_, group)| !group.contains('}'))
        })
}

fn product_source_violations(path: &Path, source: &str) -> Vec<&'static str> {
    let mut remaining = source.to_owned();
    if path == Path::new("crates/tsonic_rust_js/src/array/js_array/erasure.rs") {
        if source.contains("pub trait NativeArrayStorage") {
            return vec!["sealed native array owner contract"];
        }
        for fragment in [
            "use std::any::Any;",
            "trait NativeArrayStorage: Any + tsonic_rust_runtime::ObjectIdentityCarrier {",
            r#"pub fn restore<Value: 'static>(&self) -> Option<JsArray<Value>> {
        let owner: Rc<dyn Any> = self.owner.clone();
        owner
            .downcast::<JsArrayOwner<Value>>()
            .ok()
            .map(|state| JsArray { state })
    }"#,
            r#"fn checked_native_owner<Value: 'static>(
        &self,
    ) -> crate::errors::JsResult<&JsArrayOwner<Value>> {
        let owner: &dyn Any = self.owner.as_ref();
        owner.downcast_ref::<JsArrayOwner<Value>>().ok_or_else(|| {
            crate::errors::type_error(
                "An array operation requires the exact native element backing.",
            )
        })
    }"#,
        ] {
            if remaining.matches(fragment).count() != 1 {
                return vec!["exact native array recovery contract"];
            }
            remaining = remaining.replace(fragment, "");
        }
    }
    find_forbidden_patterns(&remaining)
}

fn locate_workspace_root() -> Option<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut current = manifest_dir.to_path_buf();
    loop {
        if current.join("Cargo.toml").exists() && current.join("crates").is_dir() {
            return Some(current);
        }

        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
            continue;
        }
        return None;
    }
}

fn collect_workspace_product_sources(root: &Path, out: &mut Vec<PathBuf>) {
    let crates_root = root.join("crates");
    let Ok(crate_entries) = fs::read_dir(&crates_root) else {
        return;
    };

    for entry in crate_entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }

        let src_root = entry_path.join("src");
        if !src_root.is_dir() {
            continue;
        }
        collect_rs_under_dir(&src_root, out);
    }
}

fn collect_rs_under_dir(root: &Path, out: &mut Vec<PathBuf>) {
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
                continue;
            }

            if entry_path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            out.push(entry_path);
        }
    }
}

fn read_source_file(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| {
        panic!(
            "failed to read Rust source file {}: {}",
            path.display(),
            err
        )
    })
}
