// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyO3 bindings for override parser

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3::types::{PyDict, PyList, PySet};
use std::sync::Arc;

use lerna::{
    OverrideParser as RustOverrideParser,
    Override as RustOverride,
    ParsedElement as RustParsedElement,
    OverrideValue as RustOverrideValue,
    ValueType as RustValueType,
    ChoiceSweep as RustChoiceSweep,
    RangeSweep as RustRangeSweep,
    IntervalSweep as RustIntervalSweep,
    FunctionCallback,
};

use crate::override_types::{PyKey, PyOverrideType, PyValueType, PyQuotedString};


/// Wrapper around Python Functions object to call user-defined functions from Rust.
///
/// This allows the Rust parser to delegate unknown function calls to Python.
/// The pure Rust parser works without this - it's only used when PyO3 is involved.
pub struct PyFunctionCallback {
    /// The Python Functions object
    functions: Py<PyAny>,
}

impl PyFunctionCallback {
    /// Create a new callback wrapper around a Python Functions object
    pub fn new(functions: Py<PyAny>) -> Self {
        Self { functions }
    }
}

// Send + Sync are required by FunctionCallback trait
// This is safe because we only access the Python object while holding the GIL
unsafe impl Send for PyFunctionCallback {}
unsafe impl Sync for PyFunctionCallback {}

/// Functions that the Rust parser handles natively.
/// has_function returns False for these unless Python explicitly marks them as shadowed.
const RUST_NATIVE_FUNCTIONS: &[&str] = &[
    "choice", "range", "interval", "shuffle", "sort", "tag", "glob",
    "int", "float", "str", "bool", "json_str", "extend_list",
];

impl FunctionCallback for PyFunctionCallback {
    fn has_function(&self, name: &str) -> bool {
        // For Rust-native functions, check if Python has a user override
        if RUST_NATIVE_FUNCTIONS.contains(&name) {
            Python::attach(|py| {
                let func_obj = self.functions.bind(py);
                // Check if Functions has a 'user_overrides' set containing this name
                if let Ok(user_overrides) = func_obj.getattr("user_overrides") {
                    if let Ok(contains) = user_overrides.call_method1("__contains__", (name,)) {
                        return contains.extract::<bool>().unwrap_or(false);
                    }
                }
                false
            })
        } else {
            // For non-native functions, just check if it exists
            Python::attach(|py| {
                let func_obj = self.functions.bind(py);
                if let Ok(definitions) = func_obj.getattr("definitions") {
                    if let Ok(contains) = definitions.call_method1("__contains__", (name,)) {
                        return contains.extract::<bool>().unwrap_or(false);
                    }
                }
                false
            })
        }
    }

    fn call(
        &self,
        name: &str,
        args: Vec<RustParsedElement>,
        kwargs: Vec<(String, RustParsedElement)>,
    ) -> Result<RustParsedElement, String> {
        Python::attach(|py| {
            let func_obj = self.functions.bind(py);

            // Build a FunctionCall-like object to pass to Functions.eval
            // We need to create a dict with name, args, kwargs
            let py_args = PyList::empty(py);
            for arg in &args {
                match parsed_element_to_py(py, arg) {
                    Ok(py_arg) => { py_args.append(py_arg).map_err(|e| e.to_string())?; }
                    Err(e) => return Err(e.to_string()),
                }
            }

            let py_kwargs = PyDict::new(py);
            for (key, val) in &kwargs {
                match parsed_element_to_py(py, val) {
                    Ok(py_val) => { py_kwargs.set_item(key, py_val).map_err(|e| e.to_string())?; }
                    Err(e) => return Err(e.to_string()),
                }
            }

            // Build function call string for error messages
            let call_str = build_function_call_string(name, &args, &kwargs);

            // Call functions.eval() with a FunctionCall object
            // First we need to create a FunctionCall instance
            let func_call_mod = py.import("lerna._internal.grammar.functions")
                .map_err(|e| e.to_string())?;
            let func_call_class = func_call_mod.getattr("FunctionCall")
                .map_err(|e| e.to_string())?;
            let func_call = func_call_class.call1((name, py_args, py_kwargs.into_mapping()))
                .map_err(|e| e.to_string())?;

            // Now call functions.eval(func_call)
            let result = func_obj.call_method1("eval", (func_call,));

            match result {
                Ok(r) => py_to_parsed_element(py, &r),
                Err(e) => {
                    let msg = e.to_string();
                    // Format TypeError errors specially for Hydra compatibility
                    if msg.contains("TypeError:") || msg.contains("TypeError(") {
                        let type_msg = msg.strip_prefix("TypeError: ").unwrap_or(&msg);
                        Err(format!("TypeError while evaluating '{}': {}", call_str, type_msg))
                    } else {
                        Err(msg)
                    }
                }
            }
        })
    }
}

/// Build a string representation of a function call for error messages
fn build_function_call_string(
    name: &str,
    args: &[RustParsedElement],
    kwargs: &[(String, RustParsedElement)],
) -> String {
    let mut parts = Vec::new();

    for arg in args {
        parts.push(elem_to_source(arg));
    }

    for (key, val) in kwargs {
        parts.push(format!("{}={}", key, elem_to_source(val)));
    }

    format!("{}({})", name, parts.join(","))
}

/// Convert a ParsedElement to its source representation for error messages
fn elem_to_source(elem: &RustParsedElement) -> String {
    match elem {
        RustParsedElement::Null => "null".to_string(),
        RustParsedElement::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
        RustParsedElement::Int(i) => i.to_string(),
        RustParsedElement::Float(f) => {
            let s = f.to_string();
            if s.contains('.') { s } else { format!("{}.0", s) }
        }
        RustParsedElement::String(s) => s.clone(),
        RustParsedElement::QuotedString(qs) => {
            let q = match qs.quote {
                lerna::Quote::Single => "'",
                lerna::Quote::Double => "\"",
            };
            format!("{}{}{}", q, qs.text, q)
        }
        RustParsedElement::List(items) => {
            let parts: Vec<_> = items.iter().map(elem_to_source).collect();
            format!("[{}]", parts.join(","))
        }
        RustParsedElement::Dict(pairs) => {
            let parts: Vec<_> = pairs.iter()
                .map(|(k, v)| format!("{}:{}", k, elem_to_source(v)))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}

/// Convert a Python object to a ParsedElement
fn py_to_parsed_element(py: Python<'_>, obj: &Bound<'_, PyAny>) -> Result<RustParsedElement, String> {
    // Check for None
    if obj.is_none() {
        return Ok(RustParsedElement::Null);
    }

    // Check for bool (before int, since bool is subclass of int in Python)
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(RustParsedElement::Bool(b));
    }

    // Check for int
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(RustParsedElement::Int(i));
    }

    // Check for float
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(RustParsedElement::Float(f));
    }

    // Check for string
    if let Ok(s) = obj.extract::<String>() {
        return Ok(RustParsedElement::String(s));
    }

    // Check for QuotedString (our custom type)
    if obj.hasattr("text").unwrap_or(false) && obj.hasattr("quote").unwrap_or(false) {
        if let Ok(text) = obj.getattr("text").and_then(|t| t.extract::<String>()) {
            // It's a QuotedString
            let quote = if let Ok(q) = obj.getattr("quote").and_then(|q| q.extract::<String>()) {
                match q.as_str() {
                    "'" => lerna::Quote::Single,
                    _ => lerna::Quote::Double,
                }
            } else {
                lerna::Quote::Double
            };
            return Ok(RustParsedElement::QuotedString(lerna::QuotedString {
                text,
                quote,
            }));
        }
    }

    // Check for list
    if let Ok(list) = obj.cast::<PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(py_to_parsed_element(py, &item)?);
        }
        return Ok(RustParsedElement::List(items));
    }

    // Check for dict
    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut pairs = Vec::new();
        for (k, v) in dict.iter() {
            let key = k.extract::<String>().map_err(|e| e.to_string())?;
            let val = py_to_parsed_element(py, &v)?;
            pairs.push((key, val));
        }
        return Ok(RustParsedElement::Dict(pairs));
    }

    // Fallback: convert to string representation
    if let Ok(s) = obj.str().and_then(|s| s.extract::<String>()) {
        return Ok(RustParsedElement::String(s));
    }

    Err(format!("Cannot convert Python object to ParsedElement: {:?}", obj))
}


/// Convert a ParsedElement to a Python object
fn parsed_element_to_py(py: Python<'_>, elem: &RustParsedElement) -> PyResult<Py<PyAny>> {
    match elem {
        RustParsedElement::Null => Ok(py.None()),
        RustParsedElement::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().unbind().into_any()),
        RustParsedElement::Int(i) => Ok(i.into_pyobject(py)?.to_owned().unbind().into_any()),
        RustParsedElement::Float(f) => Ok(f.into_pyobject(py)?.to_owned().unbind().into_any()),
        RustParsedElement::String(s) => Ok(s.into_pyobject(py)?.to_owned().unbind().into_any()),
        RustParsedElement::QuotedString(qs) => {
            // Use PyO3 PyQuotedString class - keep consistent with other parsing
            let py_qs: PyQuotedString = qs.clone().into();
            Ok(Py::new(py, py_qs)?.into_any())
        }
        RustParsedElement::List(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(parsed_element_to_py(py, item)?)?;
            }
            Ok(list.into_pyobject(py)?.to_owned().unbind().into_any())
        }
        RustParsedElement::Dict(pairs) => {
            let dict = PyDict::new(py);
            for (k, v) in pairs {
                dict.set_item(k, parsed_element_to_py(py, v)?)?;
            }
            Ok(dict.into_pyobject(py)?.to_owned().unbind().into_any())
        }
    }
}

/// Convert OverrideValue to get the element for Python
fn override_value_to_element(value: &Option<RustOverrideValue>) -> Option<RustParsedElement> {
    match value {
        Some(RustOverrideValue::Element(elem)) => Some(elem.clone()),
        _ => None, // TODO: Handle sweep types
    }
}

/// Convert Rust ValueType to Python ValueType
fn value_type_to_py(value: &Option<RustOverrideValue>) -> PyValueType {
    match value {
        Some(v) => {
            let vt: RustValueType = v.value_type();
            match vt {
                RustValueType::Element => PyValueType::Element,
                RustValueType::ChoiceSweep => PyValueType::ChoiceSweep,
                RustValueType::GlobChoiceSweep => PyValueType::GlobChoiceSweep,
                RustValueType::SimpleChoiceSweep => PyValueType::SimpleChoiceSweep,
                RustValueType::RangeSweep => PyValueType::RangeSweep,
                RustValueType::IntervalSweep => PyValueType::IntervalSweep,
                RustValueType::ListExtension => PyValueType::ListExtension,
            }
        }
        None => PyValueType::Element, // Default for deletions
    }
}


/// Python-exposed Override
#[pyclass(name = "Override")]
#[derive(Clone, Debug)]
pub struct PyOverride {
    type_: PyOverrideType,
    key_or_group: String,
    value_type: PyValueType,
    package: Option<String>,
    input_line: String,
    // Store the parsed value
    value: Option<RustParsedElement>,
}

#[pymethods]
impl PyOverride {
    #[new]
    #[pyo3(signature = (type_, key_or_group, value_type, package=None, input_line=String::new()))]
    fn new(
        type_: PyOverrideType,
        key_or_group: String,
        value_type: PyValueType,
        package: Option<String>,
        input_line: String,
    ) -> Self {
        Self {
            type_,
            key_or_group,
            value_type,
            package,
            input_line,
            value: None,
        }
    }

    #[getter(r#type)]
    fn get_type(&self) -> PyOverrideType {
        self.type_
    }

    #[getter]
    fn override_type(&self) -> PyOverrideType {
        self.type_
    }

    #[getter]
    fn key_or_group(&self) -> &str {
        &self.key_or_group
    }

    #[getter]
    fn value_type(&self) -> PyValueType {
        self.value_type
    }

    #[getter]
    fn package(&self) -> Option<&str> {
        self.package.as_deref()
    }

    #[getter]
    fn input_line(&self) -> &str {
        &self.input_line
    }

    fn is_delete(&self) -> bool {
        matches!(self.type_, PyOverrideType::Del)
    }

    fn is_add(&self) -> bool {
        matches!(self.type_, PyOverrideType::Add | PyOverrideType::ForceAdd)
    }

    fn is_force_add(&self) -> bool {
        matches!(self.type_, PyOverrideType::ForceAdd)
    }

    fn is_extend_list(&self) -> bool {
        matches!(self.type_, PyOverrideType::ExtendList)
    }

    fn is_sweep_override(&self) -> bool {
        matches!(
            self.value_type,
            PyValueType::ChoiceSweep
                | PyValueType::GlobChoiceSweep
                | PyValueType::SimpleChoiceSweep
                | PyValueType::RangeSweep
                | PyValueType::IntervalSweep
        )
    }

    fn get_key_element(&self) -> PyKey {
        PyKey::from_parts(self.key_or_group.clone(), self.package.clone())
    }

    fn value(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match &self.value {
            Some(elem) => parsed_element_to_py(py, elem),
            None => Ok(py.None()),
        }
    }

    fn __str__(&self) -> &str {
        &self.input_line
    }

    fn __repr__(&self) -> String {
        format!(
            "Override(type={:?}, key_or_group={:?}, value_type={:?}, package={:?})",
            self.type_, self.key_or_group, self.value_type, self.package
        )
    }
}

impl From<RustOverride> for PyOverride {
    fn from(o: RustOverride) -> Self {
        let override_type = match o.override_type {
            lerna::OverrideType::Change => PyOverrideType::Change,
            lerna::OverrideType::Add => PyOverrideType::Add,
            lerna::OverrideType::ForceAdd => PyOverrideType::ForceAdd,
            lerna::OverrideType::Del => PyOverrideType::Del,
            lerna::OverrideType::ExtendList => PyOverrideType::ExtendList,
        };

        let value = override_value_to_element(&o.value);
        let value_type = value_type_to_py(&o.value);

        Self {
            type_: override_type,
            key_or_group: o.key.key_or_group,
            value_type,
            package: o.key.package,
            input_line: o.input_line.unwrap_or_default(),
            value,
        }
    }
}

/// Convert a ChoiceSweep to a Python dictionary
fn choice_sweep_to_py(py: Python<'_>, cs: &RustChoiceSweep) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("type", "choice_sweep")?;
    dict.set_item("simple_form", cs.simple_form)?;
    dict.set_item("shuffle", cs.shuffle)?;

    // Convert tags to a set
    let tags = PySet::empty(py)?;
    for tag in &cs.tags {
        tags.add(tag.as_str())?;
    }
    dict.set_item("tags", tags)?;

    // Convert list of elements
    let list = PyList::empty(py);
    for elem in &cs.list {
        list.append(parsed_element_to_py(py, elem)?)?;
    }
    dict.set_item("list", list)?;

    Ok(dict.unbind())
}

/// Convert a RangeSweep to a Python dictionary
fn range_sweep_to_py(py: Python<'_>, rs: &RustRangeSweep) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("type", "range_sweep")?;
    dict.set_item("start", rs.start)?;
    dict.set_item("stop", rs.stop)?;
    dict.set_item("step", rs.step)?;
    dict.set_item("shuffle", rs.shuffle)?;
    dict.set_item("is_int", rs.is_int)?;

    // Convert tags to a set
    let tags = PySet::empty(py)?;
    for tag in &rs.tags {
        tags.add(tag.as_str())?;
    }
    dict.set_item("tags", tags)?;

    Ok(dict.unbind())
}

/// Convert an IntervalSweep to a Python dictionary
fn interval_sweep_to_py(py: Python<'_>, is: &RustIntervalSweep) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("type", "interval_sweep")?;
    dict.set_item("start", is.start)?;
    dict.set_item("end", is.end)?;
    dict.set_item("is_int", is.is_int)?;

    // Convert tags to a set
    let tags = PySet::empty(py)?;
    for tag in &is.tags {
        tags.add(tag.as_str())?;
    }
    dict.set_item("tags", tags)?;

    Ok(dict.unbind())
}

/// Convert OverrideValue to Python object (element, choice_sweep, range_sweep, or interval_sweep)
fn override_value_to_py(py: Python<'_>, value: &RustOverrideValue) -> PyResult<Py<PyAny>> {
    match value {
        RustOverrideValue::Element(elem) => parsed_element_to_py(py, elem),
        RustOverrideValue::ChoiceSweep(cs) => {
            Ok(choice_sweep_to_py(py, cs)?.into_any())
        }
        RustOverrideValue::RangeSweep(rs) => {
            Ok(range_sweep_to_py(py, rs)?.into_any())
        }
        RustOverrideValue::IntervalSweep(is) => {
            Ok(interval_sweep_to_py(py, is)?.into_any())
        }
        RustOverrideValue::GlobChoiceSweep(glob) => {
            // Return glob info as dict
            let dict = PyDict::new(py);
            dict.set_item("type", "glob_choice_sweep")?;
            let include = PyList::empty(py);
            for s in &glob.include {
                include.append(s)?;
            }
            let exclude = PyList::empty(py);
            for s in &glob.exclude {
                exclude.append(s)?;
            }
            dict.set_item("include", include)?;
            dict.set_item("exclude", exclude)?;
            Ok(dict.unbind().into_any())
        }
        RustOverrideValue::ListExtension(ext) => {
            // Return list extension info as dict with list of values
            let dict = PyDict::new(py);
            dict.set_item("type", "list_extension")?;
            let values = PyList::empty(py);
            for elem in &ext.values {
                values.append(parsed_element_to_py(py, elem)?)?;
            }
            dict.set_item("values", values)?;
            Ok(dict.unbind().into_any())
        }
    }
}


/// Python-exposed OverrideParser
#[pyclass(name = "OverrideParser")]
pub struct PyOverrideParser {
    /// Optional callback for user-defined functions
    callback: Option<Arc<dyn FunctionCallback>>,
}

#[pymethods]
impl PyOverrideParser {
    #[new]
    #[pyo3(signature = (functions=None))]
    fn new(functions: Option<Py<PyAny>>) -> Self {
        let callback = functions.map(|f| {
            Arc::new(PyFunctionCallback::new(f)) as Arc<dyn FunctionCallback>
        });
        Self { callback }
    }

    /// Parse a single override string
    fn parse(&self, s: &str) -> PyResult<PyOverride> {
        let result = if let Some(ref callback) = self.callback {
            RustOverrideParser::parse_with_callback(s, callback.clone())
        } else {
            RustOverrideParser::parse(s)
        };
        result
            .map(|o| o.into())
            .map_err(|e| PyValueError::new_err(format!("{}", e)))
    }

    /// Parse and return full data as a dictionary for Python to use
    fn parse_to_dict(&self, py: Python<'_>, s: &str) -> PyResult<Py<PyDict>> {
        let result = if let Some(ref callback) = self.callback {
            RustOverrideParser::parse_with_callback(s, callback.clone())
        } else {
            RustOverrideParser::parse(s)
        }.map_err(|e| PyValueError::new_err(format!("{}", e)))?;

        let dict = PyDict::new(py);

        // Override type
        let type_str = match result.override_type {
            lerna::OverrideType::Change => "CHANGE",
            lerna::OverrideType::Add => "ADD",
            lerna::OverrideType::ForceAdd => "FORCE_ADD",
            lerna::OverrideType::Del => "DEL",
            lerna::OverrideType::ExtendList => "EXTEND_LIST",
        };
        dict.set_item("type", type_str)?;

        // Key
        dict.set_item("key_or_group", &result.key.key_or_group)?;
        dict.set_item("package", result.key.package.as_deref())?;

        // Value type
        let value_type = match &result.value {
            Some(v) => match v.value_type() {
                RustValueType::Element => "ELEMENT",
                RustValueType::ChoiceSweep => "CHOICE_SWEEP",
                RustValueType::GlobChoiceSweep => "GLOB_CHOICE_SWEEP",
                RustValueType::SimpleChoiceSweep => "SIMPLE_CHOICE_SWEEP",
                RustValueType::RangeSweep => "RANGE_SWEEP",
                RustValueType::IntervalSweep => "INTERVAL_SWEEP",
                RustValueType::ListExtension => "LIST_EXTENSION",
            },
            None => "ELEMENT",
        };
        dict.set_item("value_type", value_type)?;

        // Value
        if let Some(ref value) = result.value {
            dict.set_item("value", override_value_to_py(py, value)?)?;
        } else {
            dict.set_item("value", py.None())?;
        }

        dict.set_item("input_line", s)?;

        Ok(dict.unbind())
    }

    /// Parse multiple override strings
    fn parse_many(&self, py: Python<'_>, overrides: Vec<String>) -> PyResult<Py<PyList>> {
        let str_refs: Vec<&str> = overrides.iter().map(|s| s.as_str()).collect();
        let results = if let Some(ref callback) = self.callback {
            RustOverrideParser::parse_many_with_callback(&str_refs, callback.clone())
        } else {
            RustOverrideParser::parse_many(&str_refs)
        }.map_err(|e| PyValueError::new_err(format!("{}", e)))?;

        let list = PyList::empty(py);
        for o in results {
            let py_override: PyOverride = o.into();
            list.append(Py::new(py, py_override)?)?;
        }
        Ok(list.unbind())
    }

    /// Parse many overrides and return full data as dictionaries
    fn parse_many_to_dict(&self, py: Python<'_>, overrides: Vec<String>) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        for s in &overrides {
            let dict = self.parse_to_dict(py, s)?;
            list.append(dict)?;
        }
        Ok(list.unbind())
    }

    fn __repr__(&self) -> &'static str {
        "OverrideParser()"
    }
}
