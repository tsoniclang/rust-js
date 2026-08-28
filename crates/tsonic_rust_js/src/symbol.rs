use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::hash::{Hash, Hasher};

use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};

thread_local! {
    static NEXT_SYMBOL_ID: Cell<u64> = const { Cell::new(1) };
    static SYMBOL_REGISTRY: RefCell<HashMap<String, JsSymbol>> = RefCell::new(HashMap::new());
}

#[derive(Clone)]
pub struct JsSymbol {
    token: Rc<SymbolToken>,
}

struct SymbolToken {
    id: u64,
    description: Option<String>,
    registry_key: Option<String>,
}

impl JsSymbol {
    pub fn create() -> Self {
        Self::with_description(None)
    }

    pub fn create_string(description: &str) -> Self {
        Self::with_description(Some(description.to_owned()))
    }

    pub fn create_number(description: f64) -> Self {
        Self::with_description(Some(ryu_js::Buffer::new().format(description).to_owned()))
    }

    pub fn for_key(key: &str) -> Self {
        SYMBOL_REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            registry
                .entry(key.to_owned())
                .or_insert_with(|| Self::with_registry_key(key.to_owned()))
                .clone()
        })
    }

    pub fn key_for(symbol: &Self) -> Option<String> {
        symbol.token.registry_key.clone()
    }

    pub fn description(&self) -> Option<String> {
        self.token.description.clone()
    }

    pub fn identity_key(&self) -> u64 {
        self.token.id
    }

    fn with_description(description: Option<String>) -> Self {
        let id = NEXT_SYMBOL_ID.with(|next| {
            let current = next.get();
            next.set(current.checked_add(1).expect("JavaScript symbol identity exhausted"));
            current
        });
        Self {
            token: Rc::new(SymbolToken {
                id,
                description,
                registry_key: None,
            }),
        }
    }

    fn with_registry_key(key: String) -> Self {
        let symbol = Self::with_description(Some(key.clone()));
        Self {
            token: Rc::new(SymbolToken {
                id: symbol.token.id,
                description: symbol.token.description.clone(),
                registry_key: Some(key),
            }),
        }
    }
}

impl std::fmt::Debug for JsSymbol {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.token.description {
            Some(description) => write!(formatter, "Symbol({description})"),
            None => formatter.write_str("Symbol()"),
        }
    }
}

impl PartialEq for JsSymbol {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.token, &other.token)
    }
}

impl Eq for JsSymbol {}

impl Hash for JsSymbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.token.id.hash(state);
    }
}

impl JsSameValueZero for JsSymbol {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsHash for JsSymbol {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.token) as usize)
    }
}

impl JsStrictEqual for JsSymbol {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}
