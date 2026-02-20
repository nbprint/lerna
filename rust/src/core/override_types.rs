// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Override types for the configuration override system.

use std::collections::HashSet;

/// Quote style for quoted strings
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Quote {
    Single = 0,
    Double = 1,
}

impl Quote {
    /// Get the quote character
    pub fn char(&self) -> char {
        match self {
            Quote::Single => '\'',
            Quote::Double => '"',
        }
    }
}

/// A quoted string with its quote style preserved
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuotedString {
    pub text: String,
    pub quote: Quote,
}

impl QuotedString {
    /// Create a new QuotedString
    pub fn new(text: String, quote: Quote) -> Self {
        Self { text, quote }
    }

    /// Create a single-quoted string
    pub fn single(text: String) -> Self {
        Self::new(text, Quote::Single)
    }

    /// Create a double-quoted string
    pub fn double(text: String) -> Self {
        Self::new(text, Quote::Double)
    }

    /// Return the string with quotes
    pub fn with_quotes(&self) -> String {
        let qc = self.quote.char();
        let esc_qc = format!("\\{}", qc);
        let escaped = self.text.replace(qc, &esc_qc);
        format!("{}{}{}", qc, escaped, qc)
    }
}

/// Type of override operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OverrideType {
    /// Change existing value (key=value)
    Change = 1,
    /// Add new value (+key=value)
    Add = 2,
    /// Force add value (++key=value)
    ForceAdd = 3,
    /// Delete value (~key)
    Del = 4,
    /// Extend list (+key+=[...])
    ExtendList = 5,
}

impl std::fmt::Display for OverrideType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverrideType::Change => write!(f, "CHANGE"),
            OverrideType::Add => write!(f, "ADD"),
            OverrideType::ForceAdd => write!(f, "FORCE_ADD"),
            OverrideType::Del => write!(f, "DEL"),
            OverrideType::ExtendList => write!(f, "EXTEND_LIST"),
        }
    }
}

/// Type of value in an override
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ValueType {
    Element = 1,
    ChoiceSweep = 2,
    GlobChoiceSweep = 3,
    SimpleChoiceSweep = 4,
    RangeSweep = 5,
    IntervalSweep = 6,
    ListExtension = 7,
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueType::Element => write!(f, "ELEMENT"),
            ValueType::ChoiceSweep => write!(f, "CHOICE_SWEEP"),
            ValueType::GlobChoiceSweep => write!(f, "GLOB_CHOICE_SWEEP"),
            ValueType::SimpleChoiceSweep => write!(f, "SIMPLE_CHOICE_SWEEP"),
            ValueType::RangeSweep => write!(f, "RANGE_SWEEP"),
            ValueType::IntervalSweep => write!(f, "INTERVAL_SWEEP"),
            ValueType::ListExtension => write!(f, "LIST_EXTENSION"),
        }
    }
}

/// A key in the configuration (e.g., "db.driver" or "db[name]")
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    /// The key name (e.g., "db")
    pub key_or_group: String,
    /// Package prefix (e.g., "pkg" in "@pkg:db")
    pub package: Option<String>,
}

impl Key {
    /// Create a new key
    pub fn new(key_or_group: String) -> Self {
        Self {
            key_or_group,
            package: None,
        }
    }

    /// Create a new key with package
    pub fn with_package(key_or_group: String, package: String) -> Self {
        Self {
            key_or_group,
            package: Some(package),
        }
    }

    /// Check if the key has a package
    pub fn has_package(&self) -> bool {
        self.package.is_some()
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(pkg) = &self.package {
            write!(f, "@{}:{}", pkg, self.key_or_group)
        } else {
            write!(f, "{}", self.key_or_group)
        }
    }
}

/// Base trait for sweep types
pub trait Sweep {
    fn tags(&self) -> &HashSet<String>;
    fn tags_mut(&mut self) -> &mut HashSet<String>;
}

/// A choice sweep (e.g., "db=mysql,postgresql")
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceSweep {
    pub tags: HashSet<String>,
    pub list: Vec<ParsedElement>,
    pub simple_form: bool,
    pub shuffle: bool,
}

impl Default for ChoiceSweep {
    fn default() -> Self {
        Self {
            tags: HashSet::new(),
            list: Vec::new(),
            simple_form: false,
            shuffle: false,
        }
    }
}

impl Sweep for ChoiceSweep {
    fn tags(&self) -> &HashSet<String> {
        &self.tags
    }

    fn tags_mut(&mut self) -> &mut HashSet<String> {
        &mut self.tags
    }
}

/// A range sweep (e.g., "x=range(1,10)")
#[derive(Clone, Debug, PartialEq)]
pub struct RangeSweep {
    pub tags: HashSet<String>,
    pub start: Option<f64>,
    pub stop: Option<f64>,
    pub step: f64,
    pub shuffle: bool,
    /// Whether all values should be treated as integers (from int() cast or integer input)
    pub is_int: bool,
}

impl Default for RangeSweep {
    fn default() -> Self {
        Self {
            tags: HashSet::new(),
            start: None,
            stop: None,
            step: 1.0,
            shuffle: false,
            is_int: false,
        }
    }
}

impl Sweep for RangeSweep {
    fn tags(&self) -> &HashSet<String> {
        &self.tags
    }

    fn tags_mut(&mut self) -> &mut HashSet<String> {
        &mut self.tags
    }
}

/// An interval sweep (e.g., "x=interval(0.0, 1.0)")
#[derive(Clone, Debug, PartialEq)]
pub struct IntervalSweep {
    pub tags: HashSet<String>,
    pub start: Option<f64>,
    pub end: Option<f64>,
    /// Whether int() cast was applied (affects Python type conversion)
    pub is_int: bool,
}

impl Default for IntervalSweep {
    fn default() -> Self {
        Self {
            tags: HashSet::new(),
            start: None,
            end: None,
            is_int: false,
        }
    }
}

impl Sweep for IntervalSweep {
    fn tags(&self) -> &HashSet<String> {
        &self.tags
    }

    fn tags_mut(&mut self) -> &mut HashSet<String> {
        &mut self.tags
    }
}

/// A list extension value (for extend_list function)
#[derive(Clone, Debug, PartialEq)]
pub struct ListExtension {
    pub values: Vec<ParsedElement>,
}

/// A glob choice sweep (pattern-based selection)
#[derive(Clone, Debug, PartialEq)]
pub struct GlobChoiceSweep {
    pub tags: HashSet<String>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl Default for GlobChoiceSweep {
    fn default() -> Self {
        Self {
            tags: HashSet::new(),
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }
}

impl Sweep for GlobChoiceSweep {
    fn tags(&self) -> &HashSet<String> {
        &self.tags
    }

    fn tags_mut(&mut self) -> &mut HashSet<String> {
        &mut self.tags
    }
}

/// A parsed element value
#[derive(Clone, Debug, PartialEq)]
pub enum ParsedElement {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    QuotedString(QuotedString),
    List(Vec<ParsedElement>),
    Dict(Vec<(String, ParsedElement)>),
}

impl ParsedElement {
    /// Check if the element is null
    pub fn is_null(&self) -> bool {
        matches!(self, ParsedElement::Null)
    }

    /// Try to get as string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ParsedElement::String(s) => Some(s),
            ParsedElement::QuotedString(qs) => Some(&qs.text),
            _ => None,
        }
    }

    /// Try to get as i64
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ParsedElement::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as f64
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ParsedElement::Float(f) => Some(*f),
            ParsedElement::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Try to get as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParsedElement::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// The value part of an override
#[derive(Clone, Debug, PartialEq)]
pub enum OverrideValue {
    Element(ParsedElement),
    ChoiceSweep(ChoiceSweep),
    RangeSweep(RangeSweep),
    IntervalSweep(IntervalSweep),
    GlobChoiceSweep(GlobChoiceSweep),
    ListExtension(ListExtension),
}

impl OverrideValue {
    /// Get the value type
    pub fn value_type(&self) -> ValueType {
        match self {
            OverrideValue::Element(_) => ValueType::Element,
            OverrideValue::ChoiceSweep(cs) if cs.simple_form => ValueType::SimpleChoiceSweep,
            OverrideValue::ChoiceSweep(_) => ValueType::ChoiceSweep,
            OverrideValue::RangeSweep(_) => ValueType::RangeSweep,
            OverrideValue::IntervalSweep(_) => ValueType::IntervalSweep,
            OverrideValue::GlobChoiceSweep(_) => ValueType::GlobChoiceSweep,
            OverrideValue::ListExtension(_) => ValueType::ListExtension,
        }
    }

    /// Check if this is a sweep
    pub fn is_sweep(&self) -> bool {
        !matches!(self, OverrideValue::Element(_))
    }
}

/// A complete override (e.g., "+db=mysql" or "db.port=3306")
#[derive(Clone, Debug, PartialEq)]
pub struct Override {
    /// The type of override operation
    pub override_type: OverrideType,
    /// The key being overridden
    pub key: Key,
    /// The value (None for deletions)
    pub value: Option<OverrideValue>,
    /// The original input line
    pub input_line: Option<String>,
}

impl Override {
    /// Create a new change override
    pub fn change(key: Key, value: OverrideValue) -> Self {
        Self {
            override_type: OverrideType::Change,
            key,
            value: Some(value),
            input_line: None,
        }
    }

    /// Create a new add override
    pub fn add(key: Key, value: OverrideValue) -> Self {
        Self {
            override_type: OverrideType::Add,
            key,
            value: Some(value),
            input_line: None,
        }
    }

    /// Create a new delete override
    pub fn delete(key: Key) -> Self {
        Self {
            override_type: OverrideType::Del,
            key,
            value: None,
            input_line: None,
        }
    }

    /// Check if this is a sweep override
    pub fn is_sweep(&self) -> bool {
        self.value.as_ref().map_or(false, |v| v.is_sweep())
    }

    /// Get the value type if there is a value
    pub fn value_type(&self) -> Option<ValueType> {
        self.value.as_ref().map(|v| v.value_type())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_char() {
        assert_eq!(Quote::Single.char(), '\'');
        assert_eq!(Quote::Double.char(), '"');
    }

    #[test]
    fn test_quoted_string_with_quotes() {
        let qs = QuotedString::single("hello".to_string());
        assert_eq!(qs.with_quotes(), "'hello'");

        let qs = QuotedString::double("hello".to_string());
        assert_eq!(qs.with_quotes(), "\"hello\"");

        // Test escaping
        let qs = QuotedString::single("it's".to_string());
        assert_eq!(qs.with_quotes(), "'it\\'s'");
    }

    #[test]
    fn test_override_type_display() {
        assert_eq!(format!("{}", OverrideType::Change), "CHANGE");
        assert_eq!(format!("{}", OverrideType::Add), "ADD");
        assert_eq!(format!("{}", OverrideType::Del), "DEL");
    }

    #[test]
    fn test_key_display() {
        let key = Key::new("db.driver".to_string());
        assert_eq!(format!("{}", key), "db.driver");

        let key = Key::with_package("db".to_string(), "pkg".to_string());
        assert_eq!(format!("{}", key), "@pkg:db");
    }

    #[test]
    fn test_parsed_element() {
        let elem = ParsedElement::String("hello".to_string());
        assert_eq!(elem.as_str(), Some("hello"));
        assert_eq!(elem.as_int(), None);

        let elem = ParsedElement::Int(42);
        assert_eq!(elem.as_int(), Some(42));
        assert_eq!(elem.as_float(), Some(42.0));
    }

    #[test]
    fn test_override_change() {
        let key = Key::new("db.port".to_string());
        let value = OverrideValue::Element(ParsedElement::Int(3306));
        let ovr = Override::change(key, value);

        assert_eq!(ovr.override_type, OverrideType::Change);
        assert!(!ovr.is_sweep());
        assert_eq!(ovr.value_type(), Some(ValueType::Element));
    }

    #[test]
    fn test_override_delete() {
        let key = Key::new("db".to_string());
        let ovr = Override::delete(key);

        assert_eq!(ovr.override_type, OverrideType::Del);
        assert!(ovr.value.is_none());
        assert!(!ovr.is_sweep());
    }
}
