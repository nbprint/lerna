// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyO3 bindings for override types

use pyo3::prelude::*;

use lerna::{
    Quote as RustQuote,
    QuotedString as RustQuotedString,
    OverrideType as RustOverrideType,
    ValueType as RustValueType,
    Key as RustKey,
};


/// Python-exposed Quote enum
#[pyclass(name = "Quote", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyQuote {
    #[pyo3(name = "single")]
    Single = 0,
    #[pyo3(name = "double")]
    Double = 1,
}

#[pymethods]
impl PyQuote {
    fn __str__(&self) -> &'static str {
        match self {
            PyQuote::Single => "single",
            PyQuote::Double => "double",
        }
    }

    fn __repr__(&self) -> String {
        format!("Quote.{}", self.__str__())
    }
}

impl From<RustQuote> for PyQuote {
    fn from(q: RustQuote) -> Self {
        match q {
            RustQuote::Single => PyQuote::Single,
            RustQuote::Double => PyQuote::Double,
        }
    }
}

impl From<PyQuote> for RustQuote {
    fn from(q: PyQuote) -> Self {
        match q {
            PyQuote::Single => RustQuote::Single,
            PyQuote::Double => RustQuote::Double,
        }
    }
}


/// Python-exposed OverrideType enum
#[pyclass(name = "OverrideType", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyOverrideType {
    #[pyo3(name = "CHANGE")]
    Change = 1,
    #[pyo3(name = "ADD")]
    Add = 2,
    #[pyo3(name = "FORCE_ADD")]
    ForceAdd = 3,
    #[pyo3(name = "DEL")]
    Del = 4,
    #[pyo3(name = "EXTEND_LIST")]
    ExtendList = 5,
}

#[pymethods]
impl PyOverrideType {
    fn __str__(&self) -> &'static str {
        match self {
            PyOverrideType::Change => "CHANGE",
            PyOverrideType::Add => "ADD",
            PyOverrideType::ForceAdd => "FORCE_ADD",
            PyOverrideType::Del => "DEL",
            PyOverrideType::ExtendList => "EXTEND_LIST",
        }
    }

    fn __repr__(&self) -> String {
        format!("OverrideType.{}", self.__str__())
    }
}

impl From<RustOverrideType> for PyOverrideType {
    fn from(ot: RustOverrideType) -> Self {
        match ot {
            RustOverrideType::Change => PyOverrideType::Change,
            RustOverrideType::Add => PyOverrideType::Add,
            RustOverrideType::ForceAdd => PyOverrideType::ForceAdd,
            RustOverrideType::Del => PyOverrideType::Del,
            RustOverrideType::ExtendList => PyOverrideType::ExtendList,
        }
    }
}


/// Python-exposed ValueType enum
#[pyclass(name = "ValueType", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyValueType {
    #[pyo3(name = "ELEMENT")]
    Element = 1,
    #[pyo3(name = "CHOICE_SWEEP")]
    ChoiceSweep = 2,
    #[pyo3(name = "GLOB_CHOICE_SWEEP")]
    GlobChoiceSweep = 3,
    #[pyo3(name = "SIMPLE_CHOICE_SWEEP")]
    SimpleChoiceSweep = 4,
    #[pyo3(name = "RANGE_SWEEP")]
    RangeSweep = 5,
    #[pyo3(name = "INTERVAL_SWEEP")]
    IntervalSweep = 6,
    #[pyo3(name = "LIST_EXTENSION")]
    ListExtension = 7,
}

#[pymethods]
impl PyValueType {
    fn __str__(&self) -> &'static str {
        match self {
            PyValueType::Element => "ELEMENT",
            PyValueType::ChoiceSweep => "CHOICE_SWEEP",
            PyValueType::GlobChoiceSweep => "GLOB_CHOICE_SWEEP",
            PyValueType::SimpleChoiceSweep => "SIMPLE_CHOICE_SWEEP",
            PyValueType::RangeSweep => "RANGE_SWEEP",
            PyValueType::IntervalSweep => "INTERVAL_SWEEP",
            PyValueType::ListExtension => "LIST_EXTENSION",
        }
    }

    fn __repr__(&self) -> String {
        format!("ValueType.{}", self.__str__())
    }
}

impl From<RustValueType> for PyValueType {
    fn from(vt: RustValueType) -> Self {
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
}


/// Python-exposed QuotedString
#[pyclass(name = "QuotedString")]
#[derive(Clone, Debug)]
pub struct PyQuotedString {
    inner: RustQuotedString,
}

#[pymethods]
impl PyQuotedString {
    #[new]
    fn new(text: String, quote: PyQuote) -> Self {
        Self {
            inner: RustQuotedString::new(text, quote.into()),
        }
    }

    #[getter]
    fn text(&self) -> &str {
        &self.inner.text
    }

    #[getter]
    fn quote(&self) -> PyQuote {
        self.inner.quote.into()
    }

    fn with_quotes(&self) -> String {
        self.inner.with_quotes()
    }

    fn __str__(&self) -> &str {
        &self.inner.text
    }

    fn __repr__(&self) -> String {
        format!("QuotedString(text={:?}, quote={:?})", self.inner.text, self.inner.quote)
    }
}

impl From<RustQuotedString> for PyQuotedString {
    fn from(qs: RustQuotedString) -> Self {
        Self { inner: qs }
    }
}


/// Python-exposed Key
#[pyclass(name = "Key")]
#[derive(Clone, Debug)]
pub struct PyKey {
    inner: RustKey,
}

impl PyKey {
    /// Create from parts (for internal use)
    pub fn from_parts(key_or_group: String, package: Option<String>) -> Self {
        Self {
            inner: RustKey {
                key_or_group,
                package,
            },
        }
    }
}

#[pymethods]
impl PyKey {
    #[new]
    #[pyo3(signature = (key_or_group, package=None))]
    fn new(key_or_group: String, package: Option<String>) -> Self {
        Self::from_parts(key_or_group, package)
    }

    #[getter]
    fn key_or_group(&self) -> &str {
        &self.inner.key_or_group
    }

    #[getter]
    fn package(&self) -> Option<&str> {
        self.inner.package.as_deref()
    }

    fn has_package(&self) -> bool {
        self.inner.has_package()
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    fn __repr__(&self) -> String {
        format!("Key(key_or_group={:?}, package={:?})", self.inner.key_or_group, self.inner.package)
    }
}

impl From<RustKey> for PyKey {
    fn from(key: RustKey) -> Self {
        Self { inner: key }
    }
}
