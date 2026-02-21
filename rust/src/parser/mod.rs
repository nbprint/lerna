// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Override parser for configuration overrides.

use crate::core::{
    ChoiceSweep, IntervalSweep, Key, ListExtension, ListOperationType, Override, OverrideType,
    OverrideValue, ParsedElement, Quote, QuotedString, RangeSweep,
};
use rand::seq::SliceRandom;
use std::sync::Arc;

/// Parser errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Don't add position prefix for errors that already have context
        if self.message.starts_with("TypeError while evaluating")
            || self.message.starts_with("ValueError while evaluating")
        {
            write!(f, "{}", self.message)
        } else {
            write!(
                f,
                "Parse error at position {}: {}",
                self.position, self.message
            )
        }
    }
}

impl std::error::Error for ParseError {}

/// Trait for custom function evaluation.
///
/// Implementations can call user-defined functions (e.g., Python callbacks via PyO3).
/// The pure Rust parser works without any callback - this is optional.
pub trait FunctionCallback: Send + Sync {
    /// Check if a function with this name is registered.
    fn has_function(&self, name: &str) -> bool;

    /// Call a user-defined function with the given arguments.
    /// Returns the result as a ParsedElement, or an error message.
    fn call(
        &self,
        name: &str,
        args: Vec<ParsedElement>,
        kwargs: Vec<(String, ParsedElement)>,
    ) -> Result<ParsedElement, String>;
}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// A simple override parser
pub struct OverrideParser {
    input: Vec<char>,
    pos: usize,
    /// Optional callback for user-defined functions.
    /// When None, only built-in functions are available (pure Rust mode).
    /// When Some, unknown functions are delegated to the callback (PyO3 mode).
    function_callback: Option<Arc<dyn FunctionCallback>>,
}

impl OverrideParser {
    /// Create a new parser for the given input (pure Rust mode, no user-defined functions)
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            function_callback: None,
        }
    }

    /// Create a new parser with a function callback for user-defined functions
    pub fn with_callback(input: &str, callback: Arc<dyn FunctionCallback>) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            function_callback: Some(callback),
        }
    }

    /// Parse a complete override string (pure Rust mode)
    pub fn parse(input: &str) -> ParseResult<Override> {
        let mut parser = Self::new(input);
        let result = parser.parse_override()?;

        // Ensure we consumed all input
        parser.skip_whitespace();
        if parser.pos < parser.input.len() {
            return Err(ParseError {
                message: format!("Unexpected character: '{}'", parser.current()),
                position: parser.pos,
            });
        }

        Ok(result)
    }

    /// Parse a complete override string with user-defined function support
    pub fn parse_with_callback(
        input: &str,
        callback: Arc<dyn FunctionCallback>,
    ) -> ParseResult<Override> {
        let mut parser = Self::with_callback(input, callback);
        let result = parser.parse_override()?;

        // Ensure we consumed all input
        parser.skip_whitespace();
        if parser.pos < parser.input.len() {
            return Err(ParseError {
                message: format!("Unexpected character: '{}'", parser.current()),
                position: parser.pos,
            });
        }

        Ok(result)
    }

    /// Parse multiple overrides (pure Rust mode)
    pub fn parse_many(overrides: &[&str]) -> ParseResult<Vec<Override>> {
        overrides
            .iter()
            .enumerate()
            .map(|(idx, s)| {
                Self::parse(s).map_err(|e| ParseError {
                    message: format!("Error parsing override {}: {}", idx, e.message),
                    position: e.position,
                })
            })
            .collect()
    }

    /// Parse multiple overrides with user-defined function support
    pub fn parse_many_with_callback(
        overrides: &[&str],
        callback: Arc<dyn FunctionCallback>,
    ) -> ParseResult<Vec<Override>> {
        overrides
            .iter()
            .enumerate()
            .map(|(idx, s)| {
                Self::parse_with_callback(s, callback.clone()).map_err(|e| ParseError {
                    message: format!("Error parsing override {}: {}", idx, e.message),
                    position: e.position,
                })
            })
            .collect()
    }

    fn parse_override(&mut self) -> ParseResult<Override> {
        self.skip_whitespace();

        // Check for override type prefix
        let override_type = self.parse_override_type();

        // Parse the key
        let key = self.parse_key()?;

        // Check for value
        if override_type == OverrideType::Del {
            // Delete operations can optionally have values (~key or ~key=value)
            if self.consume('=') {
                // Delete with value match
                let value = self.parse_value()?;
                return Ok(Override {
                    override_type,
                    key,
                    value: Some(value),
                    input_line: None,
                });
            } else {
                // Delete unconditionally
                return Ok(Override {
                    override_type,
                    key,
                    value: None,
                    input_line: None,
                });
            }
        }

        // Expect '='
        if !self.consume('=') {
            return Err(ParseError {
                message: "Expected '=' after key".to_string(),
                position: self.pos,
            });
        }

        // Parse the value
        let value = self.parse_value()?;

        Ok(Override {
            override_type,
            key,
            value: Some(value),
            input_line: None,
        })
    }

    fn parse_override_type(&mut self) -> OverrideType {
        if self.consume('~') {
            OverrideType::Del
        } else if self.consume('+') {
            if self.consume('+') {
                OverrideType::ForceAdd
            } else {
                OverrideType::Add
            }
        } else {
            OverrideType::Change
        }
    }

    fn parse_key(&mut self) -> ParseResult<Key> {
        let mut key = String::new();
        let mut package = None;

        // Check for package prefix (@pkg:)
        if self.peek() == Some('@') {
            self.advance();
            let pkg = self.parse_package_name()?;
            if !self.consume(':') {
                return Err(ParseError {
                    message: "Expected ':' after package name".to_string(),
                    position: self.pos,
                });
            }
            package = Some(pkg);
        }

        // Parse the key itself (can include dots, slashes, and brackets)
        // For group overrides: group1/group2@package=value
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '.' || c == '/' || c == '[' || c == ']' {
                key.push(c);
                self.advance();
            } else if c == '@' && package.is_none() {
                // Package suffix: group1/group2@pkg=value
                self.advance();
                let pkg = self.parse_package_name()?;
                package = Some(pkg);
                break;
            } else {
                break;
            }
        }

        if key.is_empty() {
            return Err(ParseError {
                message: "Expected key".to_string(),
                position: self.pos,
            });
        }

        Ok(Key {
            key_or_group: key,
            package,
        })
    }

    fn parse_package_name(&mut self) -> ParseResult<String> {
        // Parse a package name which can include dots (e.g., "group1.pkg2")
        let mut name = String::new();

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '.' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if name.is_empty() {
            return Err(ParseError {
                message: "Expected package name".to_string(),
                position: self.pos,
            });
        }

        Ok(name)
    }

    fn parse_identifier(&mut self) -> ParseResult<String> {
        let mut ident = String::new();

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if ident.is_empty() {
            return Err(ParseError {
                message: "Expected identifier".to_string(),
                position: self.pos,
            });
        }

        Ok(ident)
    }

    fn parse_value(&mut self) -> ParseResult<OverrideValue> {
        self.skip_whitespace();

        // Handle empty value (key= with nothing after)
        if self.peek().is_none() {
            return Ok(OverrideValue::Element(ParsedElement::String(String::new())));
        }

        // Check if this starts with an identifier (might be a function call)
        if let Some(c) = self.peek() {
            if c.is_alphabetic() || c == '_' {
                let start_pos = self.pos;
                let ident = self.parse_identifier()?;
                self.skip_whitespace();

                // Check if this is a function call
                if self.peek() == Some('(') {
                    return self.parse_function_call(&ident);
                }

                // Not a function call, reset and continue
                self.pos = start_pos;
            }
        }

        // Check if this is a simple choice sweep (a,b,c without function call)
        if let Some(sweep) = self.try_parse_simple_choice()? {
            return Ok(sweep);
        }

        let elem = self.parse_element()?;
        Ok(OverrideValue::Element(elem))
    }

    /// Try to parse a simple choice sweep (comma-separated values without function call)
    fn try_parse_simple_choice(&mut self) -> ParseResult<Option<OverrideValue>> {
        let start_pos = self.pos;

        // Parse first element
        let first = match self.parse_element() {
            Ok(e) => e,
            Err(_) => {
                self.pos = start_pos;
                return Ok(None);
            }
        };

        self.skip_whitespace();

        // Check if there's a comma (indicating a choice)
        if self.peek() != Some(',') {
            self.pos = start_pos;
            return Ok(None);
        }

        // This is a simple choice sweep
        let mut choices = vec![first];

        while self.consume(',') {
            self.skip_whitespace();
            let elem = self.parse_element()?;
            choices.push(elem);
            self.skip_whitespace();
        }

        Ok(Some(OverrideValue::ChoiceSweep(ChoiceSweep {
            tags: std::collections::HashSet::new(),
            list: choices,
            simple_form: true,
            shuffle: false,
        })))
    }

    fn parse_element(&mut self) -> ParseResult<ParsedElement> {
        self.skip_whitespace();

        match self.peek() {
            None => Err(ParseError {
                message: "Unexpected end of input".to_string(),
                position: self.pos,
            }),
            Some('\'') => self.parse_quoted_string(Quote::Single),
            Some('"') => self.parse_quoted_string(Quote::Double),
            Some('[') => self.parse_list(),
            Some('{') => self.parse_dict(),
            Some('$') => self.parse_interpolation(),
            Some(c) if c.is_numeric() || c == '-' || c == '+' => {
                // Try number parsing, but fall back to unquoted value if it doesn't
                // consume everything up to a delimiter (e.g., "1___0___" should be a string)
                let saved_pos = self.pos;
                let result = self.parse_number();

                // Check if we stopped at a character that should be part of the value
                if let Some(next) = self.peek() {
                    // If next char is part of an identifier/value (not a delimiter),
                    // fall back to unquoted value parsing
                    if next == '_' || next.is_alphanumeric() {
                        self.pos = saved_pos;
                        return self.parse_unquoted_value();
                    }
                    // Also check for dot followed by an identifier char (e.g., "0.foo")
                    if next == '.' && self.pos + 1 < self.input.len() {
                        let after_dot = self.input[self.pos + 1];
                        if after_dot.is_alphabetic() || after_dot == '_' {
                            self.pos = saved_pos;
                            return self.parse_unquoted_value();
                        }
                    }
                }
                result
            }
            Some(c) if c.is_alphabetic() || c == '_' => self.parse_identifier_or_function(),
            _ => self.parse_unquoted_value(),
        }
    }

    /// Parse an interpolation like ${xyz} or ${now:%H-%M-%S}
    /// OR a $BARE_VAR (non-interpolation, just passed through as string)
    /// OR a $\{escaped\} which becomes a literal ${escaped}
    fn parse_interpolation(&mut self) -> ParseResult<ParsedElement> {
        let _start_pos = self.pos;

        // Consume $
        if !self.consume('$') {
            return Err(ParseError {
                message: "Expected '$'".to_string(),
                position: self.pos,
            });
        }

        // Check for { for actual interpolation
        if !self.consume('{') {
            // Not an interpolation - just $SOMETHING, treat as unquoted value
            // Collect the rest as a string value
            let mut value = String::from("$");
            while let Some(c) = self.peek() {
                // Handle backslash escapes
                if c == '\\' {
                    self.advance();
                    if let Some(escaped) = self.peek() {
                        value.push(escaped);
                        self.advance();
                    }
                } else if c.is_alphanumeric() || c == '_' || c == '.' || c == '-' || c == ':' {
                    value.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            return Ok(ParsedElement::String(value));
        }

        // Collect everything until the matching }
        let mut depth = 1;
        let mut content = String::new();

        while depth > 0 {
            match self.peek() {
                None => {
                    return Err(ParseError {
                        message: "Unterminated interpolation".to_string(),
                        position: self.pos,
                    });
                }
                Some('{') => {
                    depth += 1;
                    content.push('{');
                    self.advance();
                }
                Some('}') => {
                    depth -= 1;
                    if depth > 0 {
                        content.push('}');
                    }
                    self.advance();
                }
                Some(c) => {
                    content.push(c);
                    self.advance();
                }
            }
        }

        // Return the full interpolation string including ${...}
        Ok(ParsedElement::String(format!("${{{}}}", content)))
    }

    /// Parse an identifier that might be a function call
    fn parse_identifier_or_function(&mut self) -> ParseResult<ParsedElement> {
        let start_pos = self.pos;
        let mut ident = self.parse_identifier()?;

        // Check if there are glob wildcards, interpolations, or other chars immediately after
        // If so, continue collecting them as part of the value
        while let Some(c) = self.peek() {
            // Handle backslash - only escape special characters, preserve for Windows paths
            if c == '\\' {
                if self.pos + 1 < self.input.len() {
                    let next = self.input[self.pos + 1];
                    // Handle special escape sequences
                    if next == 't' {
                        // \t -> tab
                        self.advance(); // consume backslash
                        self.advance(); // consume 't'
                        ident.push('\t');
                    } else if next == 'n' {
                        // \n -> newline
                        self.advance();
                        self.advance();
                        ident.push('\n');
                    } else if next == 'r' {
                        // \r -> carriage return
                        self.advance();
                        self.advance();
                        ident.push('\r');
                    } else if next == '\t' {
                        // Backslash + actual tab character -> just tab (escape stripped)
                        self.advance();
                        ident.push('\t');
                        self.advance();
                    } else if next == '\n' {
                        // Backslash + actual newline -> just newline
                        self.advance();
                        ident.push('\n');
                        self.advance();
                    } else if next == '\r' {
                        // Backslash + actual carriage return -> just CR
                        self.advance();
                        ident.push('\r');
                        self.advance();
                    } else if next == ' '
                        || next == '='
                        || next == ','
                        || next == ':'
                        || next == '['
                        || next == ']'
                        || next == '{'
                        || next == '}'
                        || next == '('
                        || next == ')'
                        || next == '\''
                        || next == '"'
                        || next == '\\'
                    {
                        // Escape sequence: consume backslash and take next char literally
                        self.advance();
                        ident.push(next);
                        self.advance();
                    } else {
                        // Not a special escape - treat backslash as literal (Windows paths)
                        ident.push(c);
                        self.advance();
                    }
                } else {
                    // Backslash at end of input - treat as literal
                    ident.push(c);
                    self.advance();
                }
            // Handle interpolation ${...} as part of the value
            } else if c == '$' {
                self.advance();
                if self.peek() == Some('{') {
                    // It's an interpolation - include it in the value
                    ident.push('$');
                    ident.push('{');
                    self.advance();
                    let mut depth = 1;
                    while depth > 0 {
                        match self.peek() {
                            None => {
                                return Err(ParseError {
                                    message: "Unterminated interpolation".to_string(),
                                    position: self.pos,
                                });
                            }
                            Some('{') => {
                                depth += 1;
                                ident.push('{');
                                self.advance();
                            }
                            Some('}') => {
                                depth -= 1;
                                ident.push('}');
                                self.advance();
                            }
                            Some(ch) => {
                                ident.push(ch);
                                self.advance();
                            }
                        }
                    }
                } else {
                    // It's a $BARE_VAR - include it in the value
                    ident.push('$');
                    while let Some(ch) = self.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            ident.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            } else if c == '*'
                || c == '?'
                || c == '$'
                || c == '%'
                || c == '+'
                || c == '@'
                || c == '|'
            {
                ident.push(c);
                self.advance();
            } else if c.is_alphanumeric()
                || c == '_'
                || c == '-'
                || c == '.'
                || c == '/'
                || c == ':'
            {
                // Continue collecting identifier-like characters (including : for URIs)
                ident.push(c);
                self.advance();
            // Whitespace can be included if followed by more value chars (lookahead)
            } else if (c == ' ' || c == '\t') && !ident.is_empty() {
                // Look ahead to see if there's more value content after whitespace
                let saved_pos = self.pos;
                let mut temp_ws = String::new();
                temp_ws.push(c);
                self.advance();
                // Collect all whitespace
                while let Some(ws) = self.peek() {
                    if ws == ' ' || ws == '\t' {
                        temp_ws.push(ws);
                        self.advance();
                    } else {
                        break;
                    }
                }
                // Check if followed by more value content (not delimiter/bracket/paren)
                if let Some(next) = self.peek() {
                    if next.is_alphanumeric()
                        || next == '_'
                        || next == '-'
                        || next == '.'
                        || next == '/'
                        || next == ':'
                        || next == '*'
                        || next == '?'
                        || next == '$'
                        || next == '%'
                        || next == '+'
                        || next == '@'
                        || next == '|'
                        || next == '\\'
                    {
                        ident.push_str(&temp_ws);
                        // Don't advance - the while loop will handle the next char
                    } else {
                        // Not followed by value content, revert
                        self.pos = saved_pos;
                        break;
                    }
                } else {
                    // End of input, revert whitespace
                    self.pos = saved_pos;
                    break;
                }
            } else {
                break;
            }
        }

        self.skip_whitespace();

        // Check if this is a function call
        if self.peek() == Some('(') {
            // This is a function call - but parse_element returns ParsedElement, not OverrideValue
            // We need to handle this specially
            let func_result = self.parse_function_call(&ident)?;

            // Convert OverrideValue back to ParsedElement if it's a simple element
            match func_result {
                OverrideValue::Element(elem) => Ok(elem),
                _ => {
                    // For sweeps, we can't represent them as ParsedElement
                    // This is a design limitation - function calls returning sweeps
                    // need to be handled at the value level, not element level
                    self.pos = start_pos;
                    Err(ParseError {
                        message: format!(
                            "Function '{}' returns a sweep, which cannot be used here",
                            ident
                        ),
                        position: self.pos,
                    })
                }
            }
        } else {
            // Just an identifier - treat as string or special value
            let lower = ident.to_lowercase();
            match lower.as_str() {
                "null" | "~" => Ok(ParsedElement::Null),
                "true" | "yes" | "on" => Ok(ParsedElement::Bool(true)),
                "false" | "no" | "off" => Ok(ParsedElement::Bool(false)),
                "inf" => Ok(ParsedElement::Float(f64::INFINITY)),
                "-inf" => Ok(ParsedElement::Float(f64::NEG_INFINITY)),
                "nan" => Ok(ParsedElement::Float(f64::NAN)),
                _ => Ok(ParsedElement::String(ident)),
            }
        }
    }

    fn parse_quoted_string(&mut self, quote: Quote) -> ParseResult<ParsedElement> {
        let quote_char = quote.char();

        // Consume opening quote
        if !self.consume(quote_char) {
            return Err(ParseError {
                message: format!("Expected opening {}", quote_char),
                position: self.pos,
            });
        }

        // Collect the raw content between quotes, handling escaped quotes
        // Following ANTLR's approach: only backslashes before quotes are escape sequences
        let mut raw = String::new();

        while let Some(c) = self.peek() {
            self.advance();

            if c == quote_char {
                // Found a quote - check if it's escaped by counting preceding backslashes
                let mut num_backslashes = 0;
                let chars: Vec<char> = raw.chars().collect();
                for i in (0..chars.len()).rev() {
                    if chars[i] == '\\' {
                        num_backslashes += 1;
                    } else {
                        break;
                    }
                }

                if num_backslashes % 2 == 1 {
                    // Odd number of backslashes - the quote is escaped
                    raw.push(c);
                } else {
                    // Even number (or zero) - this is the closing quote
                    // Now unescape: for sequences of backslashes before a quote,
                    // each pair becomes a single backslash
                    let text = self.unescape_quoted_string(&raw, quote_char);
                    return Ok(ParsedElement::QuotedString(QuotedString::new(text, quote)));
                }
            } else {
                raw.push(c);
            }
        }

        Err(ParseError {
            message: "Unterminated quoted string".to_string(),
            position: self.pos,
        })
    }

    /// Unescape a quoted string, processing backslash sequences.
    /// - \\ at the end of string becomes \
    /// - \' or \" (escaped quotes) become the quote character
    /// - Other backslash sequences are kept as-is
    fn unescape_quoted_string(&self, s: &str, quote_char: char) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '\\' {
                // Count consecutive backslashes
                let start = i;
                while i < chars.len() && chars[i] == '\\' {
                    i += 1;
                }
                let num_backslashes = i - start;

                // Check if followed by quote
                if i < chars.len() && chars[i] == quote_char {
                    // Backslashes before quote: unescape pairs
                    // n backslashes before quote => (n-1)/2 backslashes + quote
                    let output_backslashes = (num_backslashes - 1) / 2;
                    for _ in 0..output_backslashes {
                        result.push('\\');
                    }
                    result.push(quote_char);
                    i += 1; // skip the quote
                } else if i >= chars.len() {
                    // Trailing backslashes at end of string: unescape pairs
                    // n trailing backslashes => n/2 backslashes
                    let output_backslashes = num_backslashes / 2;
                    for _ in 0..output_backslashes {
                        result.push('\\');
                    }
                } else {
                    // Backslashes not before quote: keep as-is
                    for _ in 0..num_backslashes {
                        result.push('\\');
                    }
                }
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }

        result
    }

    fn parse_list(&mut self) -> ParseResult<ParsedElement> {
        if !self.consume('[') {
            return Err(ParseError {
                message: "Expected '['".to_string(),
                position: self.pos,
            });
        }

        let mut items = Vec::new();
        self.skip_whitespace();

        if self.peek() == Some(']') {
            self.advance();
            return Ok(ParsedElement::List(items));
        }

        loop {
            items.push(self.parse_element()?);
            self.skip_whitespace();

            if self.consume(']') {
                break;
            }

            if !self.consume(',') {
                return Err(ParseError {
                    message: "Expected ',' or ']'".to_string(),
                    position: self.pos,
                });
            }

            self.skip_whitespace();
        }

        Ok(ParsedElement::List(items))
    }

    fn parse_dict(&mut self) -> ParseResult<ParsedElement> {
        if !self.consume('{') {
            return Err(ParseError {
                message: "Expected '{'".to_string(),
                position: self.pos,
            });
        }

        let mut items = Vec::new();
        self.skip_whitespace();

        if self.peek() == Some('}') {
            self.advance();
            return Ok(ParsedElement::Dict(items));
        }

        loop {
            // Parse key
            let key = self.parse_dict_key()?;

            self.skip_whitespace();

            // Expect : or =
            if !self.consume(':') && !self.consume('=') {
                return Err(ParseError {
                    message: "Expected ':' or '='".to_string(),
                    position: self.pos,
                });
            }

            // Parse value
            let value = self.parse_element()?;
            items.push((key, value));

            self.skip_whitespace();

            if self.consume('}') {
                break;
            }

            if !self.consume(',') {
                return Err(ParseError {
                    message: "Expected ',' or '}'".to_string(),
                    position: self.pos,
                });
            }

            self.skip_whitespace();
        }

        Ok(ParsedElement::Dict(items))
    }

    fn parse_dict_key(&mut self) -> ParseResult<String> {
        self.skip_whitespace();

        match self.peek() {
            Some('\'') | Some('"') => {
                // Dict keys cannot be quoted strings in override syntax
                Err(ParseError {
                    message: format!(
                        "no viable alternative at input '{{{}",
                        self.input[self.pos..].iter().take(10).collect::<String>()
                    ),
                    position: self.pos,
                })
            }
            Some(c) if c.is_numeric() || c == '-' || c == '+' => {
                // Numeric key - parse number and return as string, but check for alphanumeric suffix
                let saved_pos = self.pos;
                let elem = self.parse_number()?;

                // Check if there's more content that should be part of the key (e.g., "123id")
                if let Some(next) = self.peek() {
                    if next.is_alphanumeric() || next == '_' {
                        // Fall back to unquoted key parsing
                        self.pos = saved_pos;
                        return self.parse_dict_key_unquoted();
                    }
                }

                match elem {
                    ParsedElement::Int(i) => Ok(i.to_string()),
                    ParsedElement::Float(f) => Ok(f.to_string()),
                    _ => unreachable!(),
                }
            }
            _ => self.parse_dict_key_unquoted(),
        }
    }

    /// Parse an unquoted dict key that may contain whitespace and escaped characters
    fn parse_dict_key_unquoted(&mut self) -> ParseResult<String> {
        let mut key = String::new();
        let mut i = 0;

        // Characters that can be escaped in dict keys
        let escapable = ['\\', ':', '{', '}', '[', ']', '(', ')', '=', ',', ' ', '\t'];

        // Collect characters until we find an unescaped ':' or end of dict
        while let Some(c) = self.peek_at(i) {
            if c == '\\' {
                // Check if this is an escape sequence
                if let Some(next) = self.peek_at(i + 1) {
                    if escapable.contains(&next) {
                        // Valid escape sequence - skip backslash, include next char
                        i += 2;
                        key.push(next);
                    } else {
                        // Not an escapable character - keep the backslash
                        key.push(c);
                        i += 1;
                    }
                } else {
                    break;
                }
            } else if c == ':' {
                // End of key
                break;
            } else if c == '}' || c == ',' {
                // Unexpected end - key is incomplete
                break;
            } else {
                key.push(c);
                i += 1;
            }
        }

        // Trim trailing whitespace from key
        let trimmed = key.trim_end().to_string();

        // Advance the parser position by the number of characters consumed
        for _ in 0..i {
            self.advance();
        }

        // If we over-consumed due to trailing whitespace, we need to account for that
        // The loop consumed `i` characters but the key after trim may be shorter
        // We need to leave the trailing whitespace after the key for the parser to handle

        if trimmed.is_empty() {
            return Err(ParseError {
                message: "Expected dict key".to_string(),
                position: self.pos,
            });
        }

        Ok(trimmed)
    }

    fn parse_number(&mut self) -> ParseResult<ParsedElement> {
        let _start_pos = self.pos;
        let mut num_str = String::new();
        let mut has_dot = false;
        let mut has_exp = false;
        let mut has_underscore = false;
        let mut last_was_underscore = false;

        // Handle sign
        let sign_char = self.peek();
        if let Some(c) = sign_char {
            if c == '-' || c == '+' {
                num_str.push(c);
                self.advance();

                // Check for +inf/-inf
                if let Some(next) = self.peek() {
                    if next == 'i' || next == 'I' {
                        // Try to parse "inf"
                        let saved = self.pos;
                        let mut keyword = String::new();
                        while let Some(ch) = self.peek() {
                            if ch.is_alphabetic() {
                                keyword.push(ch);
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        if keyword.to_lowercase() == "inf" {
                            if c == '+' {
                                return Ok(ParsedElement::Float(f64::INFINITY));
                            } else {
                                return Ok(ParsedElement::Float(f64::NEG_INFINITY));
                            }
                        }
                        // Not inf, restore
                        self.pos = saved;
                    }
                }
            }
        }

        while let Some(c) = self.peek() {
            if c.is_numeric() {
                num_str.push(c);
                self.advance();
                last_was_underscore = false;
            } else if c == '_' {
                // Python-style underscore in numbers (e.g., 10_000)
                // Underscore must be between digits, not at start/end/consecutive
                if num_str.is_empty() || last_was_underscore {
                    // Invalid: starts with underscore or consecutive underscores
                    break;
                }
                // Check next char is a digit
                if self.pos + 1 < self.input.len() && self.input[self.pos + 1].is_numeric() {
                    has_underscore = true;
                    num_str.push(c);
                    self.advance();
                    last_was_underscore = true;
                } else {
                    // Underscore not followed by digit - stop
                    break;
                }
            } else if c == '.' && !has_dot && !has_exp {
                // Check if next char is a digit (to distinguish from key.subkey)
                if self.pos + 1 < self.input.len() && self.input[self.pos + 1].is_numeric() {
                    has_dot = true;
                    num_str.push(c);
                    self.advance();
                    last_was_underscore = false;
                } else {
                    break;
                }
            } else if (c == 'e' || c == 'E') && !has_exp {
                has_exp = true;
                num_str.push(c);
                self.advance();
                last_was_underscore = false;
                // Handle optional sign after exponent
                if let Some(s) = self.peek() {
                    if s == '-' || s == '+' {
                        num_str.push(s);
                        self.advance();
                    }
                }
            } else {
                break;
            }
        }

        // Remove underscores for parsing
        let parse_str = if has_underscore {
            num_str.replace('_', "")
        } else {
            num_str.clone()
        };

        if has_dot || has_exp {
            parse_str
                .parse::<f64>()
                .map(ParsedElement::Float)
                .map_err(|_| ParseError {
                    message: format!("Invalid float: {}", num_str),
                    position: self.pos,
                })
        } else {
            parse_str
                .parse::<i64>()
                .map(ParsedElement::Int)
                .map_err(|_| ParseError {
                    message: format!("Invalid integer: {}", num_str),
                    position: self.pos,
                })
        }
    }

    fn parse_unquoted_value(&mut self) -> ParseResult<ParsedElement> {
        let mut value = String::new();
        let _start_pos = self.pos;

        while let Some(c) = self.peek() {
            // Handle backslash escapes - only escape special characters
            // For Windows paths, treat backslash as literal unless followed by special chars
            if c == '\\' {
                if self.pos + 1 < self.input.len() {
                    let next = self.input[self.pos + 1];
                    // Handle special escape sequences
                    if next == 't' {
                        // \t -> tab
                        self.advance(); // consume backslash
                        self.advance(); // consume 't'
                        value.push('\t');
                    } else if next == 'n' {
                        // \n -> newline
                        self.advance();
                        self.advance();
                        value.push('\n');
                    } else if next == 'r' {
                        // \r -> carriage return
                        self.advance();
                        self.advance();
                        value.push('\r');
                    } else if next == '\t' {
                        // Backslash + actual tab character -> just tab (escape stripped)
                        self.advance();
                        value.push('\t');
                        self.advance();
                    } else if next == '\n' {
                        // Backslash + actual newline -> just newline
                        self.advance();
                        value.push('\n');
                        self.advance();
                    } else if next == '\r' {
                        // Backslash + actual carriage return -> just CR
                        self.advance();
                        value.push('\r');
                        self.advance();
                    } else if next == ' '
                        || next == '='
                        || next == ','
                        || next == ':'
                        || next == '['
                        || next == ']'
                        || next == '{'
                        || next == '}'
                        || next == '('
                        || next == ')'
                        || next == '\''
                        || next == '"'
                        || next == '\\'
                    {
                        // Escape sequence: consume backslash and take next char literally
                        self.advance();
                        value.push(next);
                        self.advance();
                    } else {
                        // Not a special escape - treat backslash as literal (Windows paths)
                        value.push(c);
                        self.advance();
                    }
                } else {
                    // Backslash at end of input - treat as literal
                    value.push(c);
                    self.advance();
                }
            // Allow alphanumeric, underscore, dash, dot, slash, colon (for URIs), glob wildcards (*?), and special chars ($%+@|)
            } else if c.is_alphanumeric()
                || c == '_'
                || c == '-'
                || c == '.'
                || c == '/'
                || c == ':'
                || c == '*'
                || c == '?'
                || c == '$'
                || c == '%'
                || c == '+'
                || c == '@'
                || c == '|'
            {
                value.push(c);
                self.advance();
            // Whitespace can be included if followed by more value chars (lookahead)
            } else if (c == ' ' || c == '\t') && !value.is_empty() {
                // Look ahead to see if there's more value content after whitespace
                let saved_pos = self.pos;
                let mut temp_ws = String::new();
                temp_ws.push(c);
                self.advance();
                // Collect all whitespace
                while let Some(ws) = self.peek() {
                    if ws == ' ' || ws == '\t' {
                        temp_ws.push(ws);
                        self.advance();
                    } else {
                        break;
                    }
                }
                // Check if followed by more value content (not delimiter/bracket/paren)
                if let Some(next) = self.peek() {
                    if next.is_alphanumeric()
                        || next == '_'
                        || next == '-'
                        || next == '.'
                        || next == '/'
                        || next == ':'
                        || next == '*'
                        || next == '?'
                        || next == '$'
                        || next == '%'
                        || next == '+'
                        || next == '@'
                        || next == '|'
                        || next == '\\'
                    {
                        value.push_str(&temp_ws);
                        // Don't advance - the while loop will handle the next char
                    } else {
                        // Not followed by value content, revert
                        self.pos = saved_pos;
                        break;
                    }
                } else {
                    // End of input, revert whitespace
                    self.pos = saved_pos;
                    break;
                }
            } else {
                break;
            }
        }

        if value.is_empty() {
            return Err(ParseError {
                message: "Expected value".to_string(),
                position: self.pos,
            });
        }

        // Check for special values (case-insensitive)
        let lower = value.to_lowercase();
        match lower.as_str() {
            "null" | "~" => Ok(ParsedElement::Null),
            "true" | "yes" | "on" => Ok(ParsedElement::Bool(true)),
            "false" | "no" | "off" => Ok(ParsedElement::Bool(false)),
            "inf" => Ok(ParsedElement::Float(f64::INFINITY)),
            "-inf" => Ok(ParsedElement::Float(f64::NEG_INFINITY)),
            "nan" => Ok(ParsedElement::Float(f64::NAN)),
            _ => Ok(ParsedElement::String(value)),
        }
    }

    /// Parse a function call like choice(a,b,c) or range(1,10)
    fn parse_function_call(&mut self, name: &str) -> ParseResult<OverrideValue> {
        // Check if user has defined a function with this name FIRST (allows shadowing built-ins)
        // But only if callback exists and has this function registered
        let use_user_function = if let Some(callback) = &self.function_callback {
            callback.has_function(name)
        } else {
            false
        };

        // For shuffle, sort, tag, and cast functions, use special parsing that allows sweep-returning functions as args
        // But only if user hasn't shadowed them
        if !use_user_function
            && (name == "shuffle"
                || name == "sort"
                || name == "tag"
                || name == "int"
                || name == "float"
                || name == "str"
                || name == "bool"
                || name == "json_str")
        {
            return self.parse_function_call_with_sweep_args(name);
        }

        // Consume opening paren
        if !self.consume('(') {
            return Err(ParseError {
                message: format!("Expected '(' after function name '{}'", name),
                position: self.pos,
            });
        }

        // Parse arguments
        let mut args: Vec<ParsedElement> = Vec::new();
        let mut kwargs: Vec<(String, ParsedElement)> = Vec::new();
        let mut seen_kwarg = false;
        self.skip_whitespace();

        if self.peek() != Some(')') {
            loop {
                self.skip_whitespace();

                // Check for keyword argument
                let arg_start = self.pos;
                if let Some(c) = self.peek() {
                    if c.is_alphabetic() {
                        let ident = self.parse_identifier()?;
                        self.skip_whitespace();
                        if self.consume('=') {
                            // This is a keyword argument
                            seen_kwarg = true;
                            self.skip_whitespace();
                            let value = self.parse_element()?;
                            kwargs.push((ident, value));
                        } else {
                            // Not a kwarg, reset and parse as element
                            self.pos = arg_start;
                            // Check if we've seen a kwarg before this positional
                            if seen_kwarg {
                                return Err(ParseError {
                                    message: "positional argument follows keyword argument"
                                        .to_string(),
                                    position: self.pos,
                                });
                            }
                            let arg = self.parse_element()?;
                            args.push(arg);
                        }
                    } else {
                        // Check if we've seen a kwarg before this positional
                        if seen_kwarg {
                            return Err(ParseError {
                                message: "positional argument follows keyword argument".to_string(),
                                position: self.pos,
                            });
                        }
                        let arg = self.parse_element()?;
                        args.push(arg);
                    }
                }

                self.skip_whitespace();

                if self.peek() == Some(')') {
                    break;
                }

                if !self.consume(',') {
                    return Err(ParseError {
                        message: "Expected ',' or ')' in function arguments".to_string(),
                        position: self.pos,
                    });
                }
            }
        }

        // Consume closing paren
        if !self.consume(')') {
            return Err(ParseError {
                message: "Expected ')' to close function call".to_string(),
                position: self.pos,
            });
        }

        // Handle different functions
        // First, check if user has defined a function with this name (allows shadowing built-ins)
        if let Some(callback) = &self.function_callback {
            if callback.has_function(name) {
                match callback.call(name, args, kwargs) {
                    Ok(result) => return Ok(OverrideValue::Element(result)),
                    Err(e) => {
                        return Err(ParseError {
                            message: e,
                            position: self.pos,
                        })
                    }
                }
            }
        }

        // Fall back to built-in functions
        match name {
            "choice" => self.build_choice_sweep(args),
            "range" => self.build_range_sweep(args, &kwargs),
            "interval" => self.build_interval_sweep(args, &kwargs),
            "glob" => self.build_glob(args, &kwargs),
            "tag" => self.build_tagged_sweep(args),
            "shuffle" => self.build_shuffle(args, &kwargs), // won't reach here, handled above
            "sort" => self.build_sort(args, &kwargs),       // won't reach here, handled above
            "extend_list" => self.build_list_append(args),
            "append" => self.build_list_append(args),
            "prepend" => self.build_list_prepend(args),
            "insert" => self.build_list_insert(args),
            "remove_at" => self.build_list_remove_at(args),
            "remove_value" => self.build_list_remove_value(args),
            "list_clear" => self.build_list_clear(args),
            "int" | "float" | "str" | "bool" | "json_str" => {
                if args.is_empty() {
                    return Err(ParseError {
                        message: format!("{}() requires at least 1 argument", name),
                        position: self.pos,
                    });
                }
                if args.len() == 1 {
                    // Single arg: apply cast directly
                    Ok(OverrideValue::Element(
                        self.apply_cast(name, args.into_iter().next().unwrap())?,
                    ))
                } else {
                    // Multiple args: create a ChoiceSweep with simple_form=true, cast each element
                    // Build source representation for error messages
                    let source_parts: Vec<_> = args.iter().map(Self::elem_to_source).collect();
                    let source = source_parts.join(",");

                    let mut cast_elements: Vec<ParsedElement> = Vec::new();
                    for arg in args {
                        match self.apply_cast(name, arg) {
                            Ok(cast_elem) => cast_elements.push(cast_elem),
                            Err(e) => {
                                // Re-wrap error with full simple choice context
                                if let Some(pos) = e.message.find("': ") {
                                    let reason = &e.message[pos + 3..];
                                    return Err(ParseError {
                                        message: format!(
                                            "ValueError while evaluating '{}({})': {}",
                                            name, source, reason
                                        ),
                                        position: e.position,
                                    });
                                }
                                return Err(e);
                            }
                        }
                    }
                    Ok(OverrideValue::ChoiceSweep(ChoiceSweep {
                        simple_form: true,
                        shuffle: false,
                        list: cast_elements,
                        tags: std::collections::HashSet::new(),
                    }))
                }
            }
            _ => {
                // Unknown function and no callback
                Err(ParseError {
                    message: format!("Unknown function: {}", name),
                    position: self.pos,
                })
            }
        }
    }

    /// Parse a function call that can accept sweep-returning functions as arguments
    fn parse_function_call_with_sweep_args(&mut self, name: &str) -> ParseResult<OverrideValue> {
        // Consume opening paren
        if !self.consume('(') {
            return Err(ParseError {
                message: format!("Expected '(' after function name '{}'", name),
                position: self.pos,
            });
        }

        // Parse arguments, allowing sweep-returning functions
        let mut args: Vec<ParsedElement> = Vec::new();
        let mut kwargs: Vec<(String, ParsedElement)> = Vec::new();
        let mut nested_sweep: Option<OverrideValue> = None;
        self.skip_whitespace();

        if self.peek() != Some(')') {
            loop {
                self.skip_whitespace();

                // Check for keyword argument
                let arg_start = self.pos;
                if let Some(c) = self.peek() {
                    if c.is_alphabetic() {
                        let ident = self.parse_identifier()?;
                        self.skip_whitespace();
                        if self.consume('=') {
                            // This is a keyword argument
                            self.skip_whitespace();
                            // Check if the value is a sweep-returning function
                            if let Some(next_c) = self.peek() {
                                if next_c.is_alphabetic() {
                                    let inner_start = self.pos;
                                    let inner_ident = self.parse_identifier()?;
                                    self.skip_whitespace();
                                    if self.peek() == Some('(')
                                        && (inner_ident == "choice"
                                            || inner_ident == "range"
                                            || inner_ident == "interval")
                                    {
                                        // It's a sweep function as kwarg value
                                        let sweep = self.parse_function_call(&inner_ident)?;
                                        // Store as kwarg referencing sweep
                                        if ident == "sweep" || ident == "list" {
                                            nested_sweep = Some(sweep);
                                        }
                                        // continue to next argument
                                    } else {
                                        // Not a sweep function, parse normally
                                        self.pos = inner_start;
                                        let value = self.parse_element()?;
                                        kwargs.push((ident, value));
                                    }
                                } else {
                                    let value = self.parse_element()?;
                                    kwargs.push((ident, value));
                                }
                            } else {
                                let value = self.parse_element()?;
                                kwargs.push((ident, value));
                            }
                        } else if self.peek() == Some('(') {
                            // This is a function call - check if it's a sweep-returning function
                            if ident == "choice"
                                || ident == "range"
                                || ident == "interval"
                                || ident == "tag"
                                || ident == "sort"
                                || ident == "shuffle"
                                || ident == "int"
                                || ident == "float"
                                || ident == "str"
                                || ident == "bool"
                                || ident == "json_str"
                            {
                                // Parse as a full function call and capture the sweep (cast functions with multiple args return sweeps)
                                let sweep = self.parse_function_call(&ident)?;
                                match &sweep {
                                    OverrideValue::ChoiceSweep(_)
                                    | OverrideValue::RangeSweep(_)
                                    | OverrideValue::IntervalSweep(_)
                                    | OverrideValue::GlobChoiceSweep(_) => {
                                        nested_sweep = Some(sweep);
                                    }
                                    OverrideValue::Element(elem) => {
                                        // Cast function with single arg returned element, add to args
                                        args.push(elem.clone());
                                    }
                                    OverrideValue::ListExtension(_) => {
                                        // ListExtension can't be used as a nested sweep
                                        args.push(ParsedElement::Null); // This case shouldn't happen
                                    }
                                }
                            } else {
                                // Parse as normal function call (which returns an element)
                                self.pos = arg_start;
                                let arg = self.parse_element()?;
                                args.push(arg);
                            }
                        } else {
                            // Not a kwarg and not a function call, reset and parse as element
                            self.pos = arg_start;
                            let arg = self.parse_element()?;
                            args.push(arg);
                        }
                    } else {
                        let arg = self.parse_element()?;
                        args.push(arg);
                    }
                }

                self.skip_whitespace();

                if self.peek() == Some(')') {
                    break;
                }

                if !self.consume(',') {
                    return Err(ParseError {
                        message: "Expected ',' or ')' in function arguments".to_string(),
                        position: self.pos,
                    });
                }
            }
        }

        // Consume closing paren
        if !self.consume(')') {
            return Err(ParseError {
                message: "Expected ')' to close function call".to_string(),
                position: self.pos,
            });
        }

        // Handle shuffle and sort with potential nested sweep
        match name {
            "shuffle" => {
                if let Some(sweep) = nested_sweep {
                    // shuffle(choice(...)) or shuffle(range(...))
                    match sweep {
                        OverrideValue::ChoiceSweep(cs) => {
                            Ok(OverrideValue::ChoiceSweep(ChoiceSweep {
                                tags: cs.tags,
                                list: cs.list,
                                simple_form: false,
                                shuffle: true,
                            }))
                        }
                        OverrideValue::RangeSweep(rs) => {
                            // Convert range to choice sweep with shuffle
                            Ok(OverrideValue::RangeSweep(RangeSweep {
                                tags: rs.tags,
                                start: rs.start,
                                stop: rs.stop,
                                step: rs.step,
                                shuffle: true,
                                is_int: rs.is_int,
                            }))
                        }
                        _ => self.build_shuffle(args, &kwargs),
                    }
                } else {
                    self.build_shuffle(args, &kwargs)
                }
            }
            "sort" => {
                if let Some(sweep) = nested_sweep {
                    // sort(choice(...)) - sort the items in the sweep
                    match sweep {
                        OverrideValue::ChoiceSweep(mut cs) => {
                            let reverse = kwargs
                                .iter()
                                .find(|(k, _)| k == "reverse")
                                .map(|(_, v)| matches!(v, ParsedElement::Bool(true)))
                                .unwrap_or(false);

                            cs.list.sort_by(|a, b| match (a.as_float(), b.as_float()) {
                                (Some(fa), Some(fb)) => {
                                    fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
                                }
                                _ => match (a.as_str(), b.as_str()) {
                                    (Some(sa), Some(sb)) => sa.cmp(sb),
                                    _ => std::cmp::Ordering::Equal,
                                },
                            });
                            if reverse {
                                cs.list.reverse();
                            }
                            Ok(OverrideValue::ChoiceSweep(cs))
                        }
                        OverrideValue::RangeSweep(rs) => {
                            // sort(range(...)) - ensure range is in the correct order
                            let reverse = kwargs
                                .iter()
                                .find(|(k, _)| k == "reverse")
                                .map(|(_, v)| matches!(v, ParsedElement::Bool(true)))
                                .unwrap_or(false);

                            let start = rs.start.unwrap_or(0.0);
                            let stop = rs.stop.unwrap_or(0.0);
                            let step = rs.step;

                            // Check if the range is ascending (step > 0) or descending (step < 0)
                            let is_ascending = step > 0.0;
                            let want_descending = reverse;

                            if is_ascending == want_descending {
                                // Need to flip the range
                                // Calculate number of values and last value
                                let n = ((stop - start) / step).floor();
                                let last_val = start + (n - 1.0) * step;

                                Ok(OverrideValue::RangeSweep(RangeSweep {
                                    tags: rs.tags,
                                    start: Some(last_val),
                                    stop: Some(start - step),
                                    step: -step,
                                    shuffle: false,
                                    is_int: rs.is_int,
                                }))
                            } else {
                                // Already in correct order
                                Ok(OverrideValue::RangeSweep(rs))
                            }
                        }
                        OverrideValue::IntervalSweep(_) => {
                            // sort(interval(...)) is not valid
                            Err(ParseError {
                                message:
                                    "Function 'interval' returns a sweep, which cannot be used here"
                                        .to_string(),
                                position: self.pos,
                            })
                        }
                        _ => self.build_sort(args, &kwargs),
                    }
                } else {
                    self.build_sort(args, &kwargs)
                }
            }
            "tag" => {
                // tag(tag1, tag2, ..., sweep) - add tags to the sweep
                if let Some(sweep) = nested_sweep {
                    // Collect all tags from args
                    let tags: std::collections::HashSet<String> = args
                        .iter()
                        .filter_map(|a| a.as_str().map(|s| s.to_string()))
                        .collect();

                    match sweep {
                        OverrideValue::ChoiceSweep(mut cs) => {
                            cs.tags = tags;
                            Ok(OverrideValue::ChoiceSweep(cs))
                        }
                        OverrideValue::IntervalSweep(mut is) => {
                            is.tags = tags;
                            Ok(OverrideValue::IntervalSweep(is))
                        }
                        OverrideValue::RangeSweep(mut rs) => {
                            rs.tags = tags;
                            Ok(OverrideValue::RangeSweep(rs))
                        }
                        _ => Err(ParseError {
                            message: "tag() requires a sweep as final argument".to_string(),
                            position: self.pos,
                        }),
                    }
                } else {
                    // No nested sweep - treat trailing args as sweep elements
                    // tag(tag1, a, b, c) - creates a ChoiceSweep with tags
                    if args.len() < 2 {
                        return Err(ParseError {
                            message: "tag() requires at least one tag and a sweep or values"
                                .to_string(),
                            position: self.pos,
                        });
                    }
                    // All but last are tags, last is sweep value or we treat multiple values as sweep
                    let tags: std::collections::HashSet<String> = args[..args.len() - 1]
                        .iter()
                        .filter_map(|a| a.as_str().map(|s| s.to_string()))
                        .collect();
                    let last = args.last().unwrap().clone();

                    // If last is a list, treat as choice
                    if let ParsedElement::List(items) = last {
                        Ok(OverrideValue::ChoiceSweep(ChoiceSweep {
                            tags,
                            list: items,
                            simple_form: false,
                            shuffle: false,
                        }))
                    } else {
                        // Single value gets wrapped in a choice
                        Ok(OverrideValue::ChoiceSweep(ChoiceSweep {
                            tags,
                            list: vec![last],
                            simple_form: false,
                            shuffle: false,
                        }))
                    }
                }
            }
            "int" | "float" | "str" | "bool" | "json_str" => {
                // Cast functions can receive a nested sweep
                if let Some(ref sweep) = nested_sweep {
                    let sweep_source = Self::sweep_to_source(sweep);
                    // Apply cast to each element of the sweep
                    match nested_sweep.unwrap() {
                        OverrideValue::ChoiceSweep(cs) => {
                            let mut cast_list: Vec<ParsedElement> = Vec::new();
                            for elem in cs.list {
                                match self.apply_cast(name, elem) {
                                    Ok(cast_elem) => cast_list.push(cast_elem),
                                    Err(e) => {
                                        // Re-wrap error with full sweep context
                                        if let Some(pos) = e.message.find("': ") {
                                            let reason = &e.message[pos + 3..];
                                            return Err(ParseError {
                                                message: format!(
                                                    "ValueError while evaluating '{}({})': {}",
                                                    name, sweep_source, reason
                                                ),
                                                position: e.position,
                                            });
                                        }
                                        return Err(e);
                                    }
                                }
                            }
                            Ok(OverrideValue::ChoiceSweep(ChoiceSweep {
                                simple_form: cs.simple_form,
                                shuffle: cs.shuffle,
                                list: cast_list,
                                tags: cs.tags,
                            }))
                        }
                        OverrideValue::RangeSweep(rs) => {
                            // For range sweeps, we need to apply cast appropriately
                            match name {
                                "int" => {
                                    Ok(OverrideValue::RangeSweep(RangeSweep {
                                        tags: rs.tags,
                                        start: rs.start.map(|s| s.floor()),
                                        stop: rs.stop.map(|s| s.floor()),
                                        step: rs.step.floor(),
                                        shuffle: rs.shuffle,
                                        is_int: true,
                                    }))
                                }
                                "float" => {
                                    // float(range(...)) returns the range with is_int=false
                                    Ok(OverrideValue::RangeSweep(RangeSweep {
                                        is_int: false,
                                        ..rs
                                    }))
                                }
                                _ => {
                                    Err(ParseError {
                                        message: format!("ValueError while evaluating '{}({})': Range can only be cast to int or float", name, sweep_source),
                                        position: self.pos,
                                    })
                                }
                            }
                        }
                        OverrideValue::IntervalSweep(is) => {
                            // For interval sweeps, we can apply int/float casts
                            match name {
                                "int" => {
                                    Ok(OverrideValue::IntervalSweep(IntervalSweep {
                                        tags: is.tags,
                                        start: is.start.map(|s| s.floor()),
                                        end: is.end.map(|e| e.floor()),
                                        is_int: true,
                                    }))
                                }
                                "float" => {
                                    Ok(OverrideValue::IntervalSweep(IntervalSweep {
                                        is_int: false,
                                        ..is
                                    }))
                                }
                                _ => {
                                    Err(ParseError {
                                        message: format!("ValueError while evaluating '{}({})': Intervals cannot be cast to {}", name, sweep_source, name),
                                        position: self.pos,
                                    })
                                }
                            }
                        }
                        _ => Err(ParseError {
                            message: format!("{}() cannot be applied to this sweep type", name),
                            position: self.pos,
                        }),
                    }
                } else if args.is_empty() {
                    Err(ParseError {
                        message: format!("{}() requires at least 1 argument", name),
                        position: self.pos,
                    })
                } else if args.len() == 1 {
                    // Single arg: apply cast directly
                    Ok(OverrideValue::Element(
                        self.apply_cast(name, args.into_iter().next().unwrap())?,
                    ))
                } else {
                    // Multiple args: create a ChoiceSweep with simple_form=true, cast each element
                    // Build source representation for error messages
                    let source_parts: Vec<_> = args.iter().map(Self::elem_to_source).collect();
                    let source = source_parts.join(",");

                    let mut cast_elements: Vec<ParsedElement> = Vec::new();
                    for arg in args {
                        match self.apply_cast(name, arg) {
                            Ok(cast_elem) => cast_elements.push(cast_elem),
                            Err(e) => {
                                // Re-wrap error with full simple choice context
                                if let Some(pos) = e.message.find("': ") {
                                    let reason = &e.message[pos + 3..];
                                    return Err(ParseError {
                                        message: format!(
                                            "ValueError while evaluating '{}({})': {}",
                                            name, source, reason
                                        ),
                                        position: e.position,
                                    });
                                }
                                return Err(e);
                            }
                        }
                    }
                    Ok(OverrideValue::ChoiceSweep(ChoiceSweep {
                        simple_form: true,
                        shuffle: false,
                        list: cast_elements,
                        tags: std::collections::HashSet::new(),
                    }))
                }
            }
            _ => Err(ParseError {
                message: format!("Internal error: unexpected function {}", name),
                position: self.pos,
            }),
        }
    }

    fn build_choice_sweep(&self, args: Vec<ParsedElement>) -> ParseResult<OverrideValue> {
        if args.is_empty() {
            return Err(ParseError {
                message: "choice() requires at least one argument".to_string(),
                position: self.pos,
            });
        }
        Ok(OverrideValue::ChoiceSweep(ChoiceSweep {
            tags: std::collections::HashSet::new(),
            list: args,
            simple_form: false,
            shuffle: false,
        }))
    }

    fn build_range_sweep(
        &self,
        args: Vec<ParsedElement>,
        kwargs: &[(String, ParsedElement)],
    ) -> ParseResult<OverrideValue> {
        // Helper to check if an element is a float (not an integer)
        fn is_float_element(elem: &ParsedElement) -> bool {
            matches!(elem, ParsedElement::Float(_))
        }

        // Check for kwargs-only call: range(start=x, stop=y, step=z)
        let start_kwarg = kwargs.iter().find(|(k, _)| k == "start");
        let stop_kwarg = kwargs.iter().find(|(k, _)| k == "stop");
        let step_kwarg = kwargs.iter().find(|(k, _)| k == "step");

        // Track if any explicit float was used
        let mut has_explicit_float = false;

        let (start, stop, step) =
            if args.is_empty() && (start_kwarg.is_some() || stop_kwarg.is_some()) {
                // All kwargs form
                let start = if let Some((_, v)) = start_kwarg {
                    has_explicit_float |= is_float_element(v);
                    self.element_to_f64(v)?
                } else {
                    0.0
                };
                let stop = if let Some((_, v)) = stop_kwarg {
                    has_explicit_float |= is_float_element(v);
                    self.element_to_f64(v)?
                } else {
                    return Err(ParseError {
                        message: "range() requires 'stop' argument".to_string(),
                        position: self.pos,
                    });
                };
                let step = if let Some((_, v)) = step_kwarg {
                    has_explicit_float |= is_float_element(v);
                    self.element_to_f64(v)?
                } else {
                    1.0
                };
                (start, stop, step)
            } else {
                // Positional args, with optional step kwarg override
                for arg in &args {
                    has_explicit_float |= is_float_element(arg);
                }
                if let Some((_, v)) = step_kwarg {
                    has_explicit_float |= is_float_element(v);
                }
                if let Some((_, v)) = stop_kwarg {
                    has_explicit_float |= is_float_element(v);
                }

                match args.len() {
                    1 => {
                        // range(stop) -> 0 to stop, check for step kwarg
                        let stop = self.element_to_f64(&args[0])?;
                        let step = if let Some((_, v)) = step_kwarg {
                            self.element_to_f64(v)?
                        } else {
                            1.0
                        };
                        // Also check for stop kwarg when first arg might be start
                        if let Some((_, v)) = stop_kwarg {
                            let start = self.element_to_f64(&args[0])?;
                            let stop = self.element_to_f64(v)?;
                            let step = if let Some((_, v)) = step_kwarg {
                                self.element_to_f64(v)?
                            } else {
                                1.0
                            };
                            (start, stop, step)
                        } else {
                            (0.0, stop, step)
                        }
                    }
                    2 => {
                        // range(start, stop), check for step kwarg
                        let start = self.element_to_f64(&args[0])?;
                        let stop = self.element_to_f64(&args[1])?;
                        let step = if let Some((_, v)) = step_kwarg {
                            self.element_to_f64(v)?
                        } else {
                            1.0
                        };
                        (start, stop, step)
                    }
                    3 => {
                        // range(start, stop, step)
                        let start = self.element_to_f64(&args[0])?;
                        let stop = self.element_to_f64(&args[1])?;
                        let step = self.element_to_f64(&args[2])?;
                        (start, stop, step)
                    }
                    _ => {
                        return Err(ParseError {
                            message: "range() requires 1, 2, or 3 arguments".to_string(),
                            position: self.pos,
                        });
                    }
                }
            };

        // is_int is true only if no explicit floats were used AND all values have no fractional part
        let is_int = !has_explicit_float
            && start.fract() == 0.0
            && stop.fract() == 0.0
            && step.fract() == 0.0;

        Ok(OverrideValue::RangeSweep(RangeSweep {
            tags: std::collections::HashSet::new(),
            start: Some(start),
            stop: Some(stop),
            step,
            shuffle: false,
            is_int,
        }))
    }

    fn build_interval_sweep(
        &self,
        args: Vec<ParsedElement>,
        kwargs: &[(String, ParsedElement)],
    ) -> ParseResult<OverrideValue> {
        // Support both positional and keyword arguments
        let start_kwarg = kwargs.iter().find(|(k, _)| k == "start");
        let end_kwarg = kwargs.iter().find(|(k, _)| k == "end");

        let (start, end) = if args.is_empty() && (start_kwarg.is_some() || end_kwarg.is_some()) {
            // All kwargs form
            let start = if let Some((_, v)) = start_kwarg {
                self.element_to_f64(v)?
            } else {
                return Err(ParseError {
                    message: "interval() requires 'start' argument".to_string(),
                    position: self.pos,
                });
            };
            let end = if let Some((_, v)) = end_kwarg {
                self.element_to_f64(v)?
            } else {
                return Err(ParseError {
                    message: "interval() requires 'end' argument".to_string(),
                    position: self.pos,
                });
            };
            (start, end)
        } else if args.len() == 2 {
            let start = self.element_to_f64(&args[0])?;
            let end = self.element_to_f64(&args[1])?;
            (start, end)
        } else {
            return Err(ParseError {
                message: "interval() requires exactly 2 arguments".to_string(),
                position: self.pos,
            });
        };

        Ok(OverrideValue::IntervalSweep(IntervalSweep {
            tags: std::collections::HashSet::new(),
            start: Some(start),
            end: Some(end),
            is_int: false,
        }))
    }

    fn build_glob(
        &self,
        args: Vec<ParsedElement>,
        kwargs: &[(String, ParsedElement)],
    ) -> ParseResult<OverrideValue> {
        // glob(include, exclude=None) or glob(include=*, exclude=*)
        let include = if !args.is_empty() {
            self.element_to_string_list(&args[0])?
        } else if let Some((_, v)) = kwargs.iter().find(|(k, _)| k == "include") {
            self.element_to_string_list(v)?
        } else {
            return Err(ParseError {
                message: "glob() requires at least include pattern".to_string(),
                position: self.pos,
            });
        };

        let exclude = kwargs
            .iter()
            .find(|(k, _)| k == "exclude")
            .map(|(_, v)| self.element_to_string_list(v))
            .transpose()?
            .unwrap_or_default();

        // Return as a special element - glob needs custom handling
        Ok(OverrideValue::Element(ParsedElement::Dict(vec![
            (
                "_type".to_string(),
                ParsedElement::String("glob".to_string()),
            ),
            (
                "include".to_string(),
                ParsedElement::List(include.into_iter().map(ParsedElement::String).collect()),
            ),
            (
                "exclude".to_string(),
                ParsedElement::List(exclude.into_iter().map(ParsedElement::String).collect()),
            ),
        ])))
    }

    fn build_tagged_sweep(&self, args: Vec<ParsedElement>) -> ParseResult<OverrideValue> {
        // tag(tag1, tag2, ..., sweep)
        if args.len() < 2 {
            return Err(ParseError {
                message: "tag() requires at least one tag and a sweep".to_string(),
                position: self.pos,
            });
        }

        // Last argument should be a sweep - for now just return as is
        // TODO: Handle tags on sweeps
        Err(ParseError {
            message: "tag() function not fully implemented yet".to_string(),
            position: self.pos,
        })
    }

    fn build_shuffle(
        &self,
        args: Vec<ParsedElement>,
        kwargs: &[(String, ParsedElement)],
    ) -> ParseResult<OverrideValue> {
        // Check for list= kwarg first: shuffle(list=[1,2,3])
        let list_from_kwarg = kwargs.iter().find(|(k, _)| k == "list").and_then(|(_, v)| {
            if let ParsedElement::List(items) = v {
                Some(items.clone())
            } else {
                None
            }
        });

        if let Some(mut items) = list_from_kwarg {
            // Shuffle the list
            let mut rng = rand::rng();
            items.shuffle(&mut rng);
            return Ok(OverrideValue::Element(ParsedElement::List(items)));
        }

        if args.is_empty() {
            return Err(ParseError {
                message: "shuffle() requires at least 1 argument".to_string(),
                position: self.pos,
            });
        }

        // shuffle([list]) - shuffle the list and return it
        // shuffle(a, b, c) returns a ChoiceSweep (simple_form=true, shuffle=true)
        if args.len() == 1 {
            match &args[0] {
                ParsedElement::List(items) => {
                    // Shuffle the list and return it
                    let mut shuffled = items.clone();
                    let mut rng = rand::rng();
                    shuffled.shuffle(&mut rng);
                    return Ok(OverrideValue::Element(ParsedElement::List(shuffled)));
                }
                // Single non-list argument - return the element (no shuffling needed)
                _ => return Ok(OverrideValue::Element(args.into_iter().next().unwrap())),
            }
        }

        // Multiple arguments - varargs form shuffle(1,2,3) -> ChoiceSweep
        let items = args.clone();

        // Single element list - return the element directly
        if items.len() == 1 {
            return Ok(OverrideValue::Element(items.into_iter().next().unwrap()));
        }

        Ok(OverrideValue::ChoiceSweep(ChoiceSweep {
            tags: std::collections::HashSet::new(),
            list: items,
            simple_form: true,
            shuffle: true,
        }))
    }

    fn build_sort(
        &self,
        args: Vec<ParsedElement>,
        kwargs: &[(String, ParsedElement)],
    ) -> ParseResult<OverrideValue> {
        let reverse = kwargs
            .iter()
            .find(|(k, _)| k == "reverse")
            .map(|(_, v)| matches!(v, ParsedElement::Bool(true)))
            .unwrap_or(false);

        // Helper to check if all elements are numeric or all are strings
        fn validate_comparable(items: &[ParsedElement]) -> Result<(), (String, String)> {
            if items.is_empty() {
                return Ok(());
            }
            let first = &items[0];
            let first_is_numeric = first.as_float().is_some();

            for item in items.iter() {
                let item_is_numeric = item.as_float().is_some();
                if item_is_numeric != first_is_numeric {
                    // Mixed types - return the two conflicting types
                    let first_type = if first_is_numeric { "int" } else { "str" };
                    let item_type = if item_is_numeric { "int" } else { "str" };
                    return Err((item_type.to_string(), first_type.to_string()));
                }
            }
            Ok(())
        }

        // Check for named 'list' kwarg: sort(list=[1,2,3])
        let list_from_kwarg = kwargs.iter().find(|(k, _)| k == "list").and_then(|(_, v)| {
            if let ParsedElement::List(items) = v {
                Some(items.clone())
            } else {
                None
            }
        });

        // If list is provided as kwarg, use it
        if let Some(items) = list_from_kwarg {
            // Validate types
            if let Err((t1, t2)) = validate_comparable(&items) {
                let args_source: Vec<_> = items.iter().map(Self::elem_to_source).collect();
                return Err(ParseError {
                    message: format!("TypeError while evaluating 'sort([{args_source}])': '<' not supported between instances of '{}' and '{}'", t1, t2, args_source=args_source.join(",")),
                    position: self.pos,
                });
            }

            let mut sorted = items;
            sorted.sort_by(|a, b| match (a.as_float(), b.as_float()) {
                (Some(fa), Some(fb)) => fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal),
                _ => match (a.as_str(), b.as_str()) {
                    (Some(sa), Some(sb)) => sa.cmp(sb),
                    _ => std::cmp::Ordering::Equal,
                },
            });
            if reverse {
                sorted.reverse();
            }
            return Ok(OverrideValue::Element(ParsedElement::List(sorted)));
        }

        if args.is_empty() {
            return Err(ParseError {
                message: "sort() requires at least 1 argument".to_string(),
                position: self.pos,
            });
        }

        // Check if first argument is a list or if we have multiple args (varargs)
        // sort([list]) returns a sorted list
        // sort(a, b, c) returns a ChoiceSweep (simple_form=true)
        if args.len() == 1 {
            if let ParsedElement::List(items) = &args[0] {
                // Validate types
                if let Err((t1, t2)) = validate_comparable(items) {
                    let args_source: Vec<_> = items.iter().map(Self::elem_to_source).collect();
                    return Err(ParseError {
                        message: format!("TypeError while evaluating 'sort([{args_source}])': '<' not supported between instances of '{}' and '{}'", t1, t2, args_source=args_source.join(",")),
                        position: self.pos,
                    });
                }

                // sort([list]) returns a plain sorted list
                let mut sorted = items.clone();
                sorted.sort_by(|a, b| match (a.as_float(), b.as_float()) {
                    (Some(fa), Some(fb)) => {
                        fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    _ => match (a.as_str(), b.as_str()) {
                        (Some(sa), Some(sb)) => sa.cmp(sb),
                        _ => std::cmp::Ordering::Equal,
                    },
                });
                if reverse {
                    sorted.reverse();
                }
                return Ok(OverrideValue::Element(ParsedElement::List(sorted)));
            }
        }

        // Validate types for varargs
        if let Err((t1, t2)) = validate_comparable(&args) {
            let args_source: Vec<_> = args.iter().map(Self::elem_to_source).collect();
            return Err(ParseError {
                message: format!("TypeError while evaluating 'sort({})': '<' not supported between instances of '{}' and '{}'", args_source.join(","), t1, t2),
                position: self.pos,
            });
        }

        // Multiple arguments (varargs) - returns ChoiceSweep, or single element if only one
        let mut sorted = args.clone();
        sorted.sort_by(|a, b| match (a.as_float(), b.as_float()) {
            (Some(fa), Some(fb)) => fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal),
            _ => match (a.as_str(), b.as_str()) {
                (Some(sa), Some(sb)) => sa.cmp(sb),
                _ => std::cmp::Ordering::Equal,
            },
        });
        if reverse {
            sorted.reverse();
        }
        // Single element - return the element directly
        if sorted.len() == 1 {
            return Ok(OverrideValue::Element(sorted.remove(0)));
        }
        Ok(OverrideValue::ChoiceSweep(ChoiceSweep {
            tags: std::collections::HashSet::new(),
            list: sorted,
            simple_form: true,
            shuffle: false,
        }))
    }

    fn build_list_append(&self, args: Vec<ParsedElement>) -> ParseResult<OverrideValue> {
        Ok(OverrideValue::ListExtension(ListExtension {
            operation: ListOperationType::Append,
            values: args,
            index: None,
        }))
    }

    fn build_list_prepend(&self, args: Vec<ParsedElement>) -> ParseResult<OverrideValue> {
        Ok(OverrideValue::ListExtension(ListExtension {
            operation: ListOperationType::Prepend,
            values: args,
            index: None,
        }))
    }

    fn build_list_insert(&self, args: Vec<ParsedElement>) -> ParseResult<OverrideValue> {
        // insert(index, value) - requires at least 2 arguments
        if args.len() < 2 {
            return Err(ParseError {
                message: "insert() requires at least 2 arguments: insert(index, value, ...)"
                    .to_string(),
                position: self.pos,
            });
        }

        // First argument must be an integer index
        let index = match &args[0] {
            ParsedElement::Int(i) => *i,
            _ => {
                return Err(ParseError {
                    message: "insert() first argument must be an integer index".to_string(),
                    position: self.pos,
                })
            }
        };

        // Remaining arguments are values to insert
        let values = args.into_iter().skip(1).collect();

        Ok(OverrideValue::ListExtension(ListExtension {
            operation: ListOperationType::Insert,
            values,
            index: Some(index),
        }))
    }

    fn build_list_remove_at(&self, args: Vec<ParsedElement>) -> ParseResult<OverrideValue> {
        // remove_at(index) - requires exactly 1 argument
        if args.len() != 1 {
            return Err(ParseError {
                message: "remove_at() requires exactly 1 argument: remove_at(index)".to_string(),
                position: self.pos,
            });
        }

        // Argument must be an integer index
        let index = match &args[0] {
            ParsedElement::Int(i) => *i,
            _ => {
                return Err(ParseError {
                    message: "remove_at() argument must be an integer index".to_string(),
                    position: self.pos,
                })
            }
        };

        Ok(OverrideValue::ListExtension(ListExtension {
            operation: ListOperationType::RemoveAt,
            values: vec![],
            index: Some(index),
        }))
    }

    fn build_list_remove_value(&self, args: Vec<ParsedElement>) -> ParseResult<OverrideValue> {
        // remove_value(value) - requires at least 1 argument
        if args.is_empty() {
            return Err(ParseError {
                message: "remove_value() requires at least 1 argument".to_string(),
                position: self.pos,
            });
        }

        Ok(OverrideValue::ListExtension(ListExtension {
            operation: ListOperationType::RemoveValue,
            values: args,
            index: None,
        }))
    }

    fn build_list_clear(&self, args: Vec<ParsedElement>) -> ParseResult<OverrideValue> {
        // list_clear() - no arguments
        if !args.is_empty() {
            return Err(ParseError {
                message: "list_clear() takes no arguments".to_string(),
                position: self.pos,
            });
        }

        Ok(OverrideValue::ListExtension(ListExtension {
            operation: ListOperationType::Clear,
            values: vec![],
            index: None,
        }))
    }

    /// Convert a ParsedElement to its source representation for error messages
    fn elem_to_source(elem: &ParsedElement) -> String {
        match elem {
            ParsedElement::Int(i) => i.to_string(),
            ParsedElement::Float(f) => {
                if f.is_nan() {
                    "nan".to_string()
                } else if f.is_infinite() {
                    if *f > 0.0 {
                        "inf".to_string()
                    } else {
                        "-inf".to_string()
                    }
                } else {
                    let s = f.to_string();
                    if !s.contains('.') {
                        format!("{}.0", s)
                    } else {
                        s
                    }
                }
            }
            ParsedElement::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            ParsedElement::String(s) => s.clone(),
            ParsedElement::QuotedString(qs) => format!("'{}'", qs.text),
            ParsedElement::Null => "null".to_string(),
            ParsedElement::List(items) => {
                let parts: Vec<_> = items.iter().map(Self::elem_to_source).collect();
                format!("[{}]", parts.join(","))
            }
            ParsedElement::Dict(entries) => {
                let parts: Vec<_> = entries
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, Self::elem_to_source(v)))
                    .collect();
                format!("{{{}}}", parts.join(","))
            }
        }
    }

    /// Convert an OverrideValue to its source representation for error messages
    fn sweep_to_source(val: &OverrideValue) -> String {
        match val {
            OverrideValue::Element(e) => Self::elem_to_source(e),
            OverrideValue::ChoiceSweep(cs) => {
                let parts: Vec<_> = cs.list.iter().map(Self::elem_to_source).collect();
                if cs.simple_form {
                    parts.join(",")
                } else {
                    format!("choice({})", parts.join(","))
                }
            }
            OverrideValue::RangeSweep(rs) => {
                let start = rs
                    .start
                    .map(|s| {
                        if rs.is_int {
                            // Integer range: show without decimal
                            format!("{}", s as i64)
                        } else {
                            // Float range: always show decimal point
                            let str_val = s.to_string();
                            if str_val.contains('.') {
                                str_val
                            } else {
                                format!("{}.0", str_val)
                            }
                        }
                    })
                    .unwrap_or_default();
                let stop = rs
                    .stop
                    .map(|s| {
                        if rs.is_int {
                            format!("{}", s as i64)
                        } else {
                            let str_val = s.to_string();
                            if str_val.contains('.') {
                                str_val
                            } else {
                                format!("{}.0", str_val)
                            }
                        }
                    })
                    .unwrap_or_default();
                format!("range({},{})", start, stop)
            }
            OverrideValue::IntervalSweep(is) => {
                let start = is
                    .start
                    .map(|s| {
                        let str_val = s.to_string();
                        // Always show decimal point for floats
                        if str_val.contains('.') {
                            str_val
                        } else {
                            format!("{}.0", str_val)
                        }
                    })
                    .unwrap_or("?".to_string());
                let end = is
                    .end
                    .map(|e| {
                        let str_val = e.to_string();
                        if str_val.contains('.') {
                            str_val
                        } else {
                            format!("{}.0", str_val)
                        }
                    })
                    .unwrap_or("?".to_string());
                format!("interval({}, {})", start, end)
            }
            OverrideValue::GlobChoiceSweep(gs) => {
                let patterns = gs.include.join(",");
                format!("glob({})", patterns)
            }
            OverrideValue::ListExtension(le) => {
                let parts: Vec<_> = le.values.iter().map(Self::elem_to_source).collect();
                format!("[{}]", parts.join(","))
            }
        }
    }

    fn apply_cast(&self, cast_type: &str, elem: ParsedElement) -> ParseResult<ParsedElement> {
        match cast_type {
            "int" => {
                match &elem {
                    ParsedElement::Int(i) => Ok(ParsedElement::Int(*i)),
                    ParsedElement::Float(f) => {
                        if f.is_infinite() {
                            Err(ParseError {
                                message: "OverflowError while evaluating 'int(inf)': cannot convert float infinity to integer".to_string(),
                                position: self.pos,
                            })
                        } else if f.is_nan() {
                            Err(ParseError {
                                message: "ValueError while evaluating 'int(nan)': cannot convert float NaN to integer".to_string(),
                                position: self.pos,
                            })
                        } else {
                            Ok(ParsedElement::Int(*f as i64))
                        }
                    }
                    ParsedElement::String(s)
                    | ParsedElement::QuotedString(QuotedString { text: s, .. }) => {
                        let source = Self::elem_to_source(&elem);
                        s.parse::<i64>()
                            .map(ParsedElement::Int)
                            .map_err(|_| ParseError {
                                message: format!("ValueError while evaluating 'int({})': invalid literal for int() with base 10: '{}'", source, s),
                                position: self.pos,
                            })
                    }
                    ParsedElement::Bool(b) => Ok(ParsedElement::Int(if *b { 1 } else { 0 })),
                    ParsedElement::List(items) => {
                        // Apply int cast to each element
                        let source = Self::elem_to_source(&elem);
                        let converted: Result<Vec<_>, _> = items
                            .iter()
                            .map(|item| self.apply_cast("int", item.clone()))
                            .collect();
                        converted.map(ParsedElement::List).map_err(|e| {
                            // Re-wrap error with full list context
                            if let Some(caps) = e.message.find("': ") {
                                let reason = &e.message[caps + 3..];
                                ParseError {
                                    message: format!(
                                        "ValueError while evaluating 'int({})': {}",
                                        source, reason
                                    ),
                                    position: e.position,
                                }
                            } else {
                                e
                            }
                        })
                    }
                    ParsedElement::Dict(entries) => {
                        // Apply int cast to each value
                        let source = Self::elem_to_source(&elem);
                        let converted: Result<Vec<_>, _> = entries
                            .iter()
                            .map(|(k, v)| {
                                self.apply_cast("int", v.clone()).map(|cv| (k.clone(), cv))
                            })
                            .collect();
                        converted.map(ParsedElement::Dict).map_err(|e| {
                            // Re-wrap error with full dict context
                            if let Some(caps) = e.message.find("': ") {
                                let reason = &e.message[caps + 3..];
                                ParseError {
                                    message: format!(
                                        "ValueError while evaluating 'int({})': {}",
                                        source, reason
                                    ),
                                    position: e.position,
                                }
                            } else {
                                e
                            }
                        })
                    }
                    ParsedElement::Null => Ok(ParsedElement::Null),
                }
            }
            "float" => {
                match &elem {
                    ParsedElement::Int(i) => Ok(ParsedElement::Float(*i as f64)),
                    ParsedElement::Float(f) => Ok(ParsedElement::Float(*f)),
                    ParsedElement::String(s)
                    | ParsedElement::QuotedString(QuotedString { text: s, .. }) => {
                        let source = Self::elem_to_source(&elem);
                        s.parse::<f64>()
                            .map(ParsedElement::Float)
                            .map_err(|_| ParseError {
                                message: format!("ValueError while evaluating 'float({})': could not convert string to float: '{}'", source, s),
                                position: self.pos,
                            })
                    }
                    ParsedElement::Bool(b) => Ok(ParsedElement::Float(if *b { 1.0 } else { 0.0 })),
                    ParsedElement::List(items) => {
                        // Apply float cast to each element
                        let source = Self::elem_to_source(&elem);
                        let converted: Result<Vec<_>, _> = items
                            .iter()
                            .map(|item| self.apply_cast("float", item.clone()))
                            .collect();
                        converted.map(ParsedElement::List).map_err(|e| {
                            // Re-wrap error with full list context
                            if let Some(caps) = e.message.find("': ") {
                                let reason = &e.message[caps + 3..];
                                ParseError {
                                    message: format!(
                                        "ValueError while evaluating 'float({})': {}",
                                        source, reason
                                    ),
                                    position: e.position,
                                }
                            } else {
                                e
                            }
                        })
                    }
                    ParsedElement::Dict(entries) => {
                        // Apply float cast to each value
                        let source = Self::elem_to_source(&elem);
                        let converted: Result<Vec<_>, _> = entries
                            .iter()
                            .map(|(k, v)| {
                                self.apply_cast("float", v.clone())
                                    .map(|cv| (k.clone(), cv))
                            })
                            .collect();
                        converted.map(ParsedElement::Dict).map_err(|e| {
                            // Re-wrap error with full dict context
                            if let Some(caps) = e.message.find("': ") {
                                let reason = &e.message[caps + 3..];
                                ParseError {
                                    message: format!(
                                        "ValueError while evaluating 'float({})': {}",
                                        source, reason
                                    ),
                                    position: e.position,
                                }
                            } else {
                                e
                            }
                        })
                    }
                    ParsedElement::Null => Ok(ParsedElement::Null),
                }
            }
            "str" => {
                match &elem {
                    ParsedElement::Int(i) => Ok(ParsedElement::String(i.to_string())),
                    ParsedElement::Float(f) => {
                        // Handle special cases: nan, inf, -inf
                        if f.is_nan() {
                            Ok(ParsedElement::String("nan".to_string()))
                        } else if f.is_infinite() {
                            if *f > 0.0 {
                                Ok(ParsedElement::String("inf".to_string()))
                            } else {
                                Ok(ParsedElement::String("-inf".to_string()))
                            }
                        } else {
                            let s = f.to_string();
                            if s.contains('.') || s.contains('e') || s.contains('E') {
                                Ok(ParsedElement::String(s))
                            } else {
                                Ok(ParsedElement::String(format!("{}.0", s)))
                            }
                        }
                    }
                    ParsedElement::String(s) => Ok(ParsedElement::String(s.clone())),
                    ParsedElement::QuotedString(qs) => Ok(ParsedElement::String(qs.text.clone())),
                    ParsedElement::Bool(b) => Ok(ParsedElement::String(
                        if *b { "true" } else { "false" }.to_string(),
                    )),
                    ParsedElement::Null => Ok(ParsedElement::String("null".to_string())),
                    ParsedElement::List(items) => {
                        // Apply str cast to each element
                        let converted: Result<Vec<_>, _> = items
                            .iter()
                            .map(|item| self.apply_cast("str", item.clone()))
                            .collect();
                        converted.map(ParsedElement::List)
                    }
                    ParsedElement::Dict(entries) => {
                        // Apply str cast to each value
                        let converted: Result<Vec<_>, _> = entries
                            .iter()
                            .map(|(k, v)| {
                                self.apply_cast("str", v.clone()).map(|cv| (k.clone(), cv))
                            })
                            .collect();
                        converted.map(ParsedElement::Dict)
                    }
                }
            }
            "bool" => {
                match &elem {
                    ParsedElement::Bool(b) => Ok(ParsedElement::Bool(*b)),
                    ParsedElement::Int(i) => Ok(ParsedElement::Bool(*i != 0)),
                    ParsedElement::Float(f) => Ok(ParsedElement::Bool(*f != 0.0)),
                    ParsedElement::String(s)
                    | ParsedElement::QuotedString(QuotedString { text: s, .. }) => {
                        let source = Self::elem_to_source(&elem);
                        match s.to_lowercase().as_str() {
                            "true" | "yes" | "on" | "1" => Ok(ParsedElement::Bool(true)),
                            "false" | "no" | "off" | "0" => Ok(ParsedElement::Bool(false)),
                            _ => Err(ParseError {
                                message: format!("ValueError while evaluating 'bool({})': Cannot cast '{}' to bool", source, s),
                                position: self.pos,
                            }),
                        }
                    }
                    ParsedElement::List(items) => {
                        // Apply bool cast to each element
                        let source = Self::elem_to_source(&elem);
                        let converted: Result<Vec<_>, _> = items
                            .iter()
                            .map(|item| self.apply_cast("bool", item.clone()))
                            .collect();
                        converted.map(ParsedElement::List).map_err(|e| {
                            // Re-wrap error with full list context
                            if let Some(caps) = e.message.find("': ") {
                                let reason = &e.message[caps + 3..];
                                ParseError {
                                    message: format!(
                                        "ValueError while evaluating 'bool({})': {}",
                                        source, reason
                                    ),
                                    position: e.position,
                                }
                            } else {
                                e
                            }
                        })
                    }
                    ParsedElement::Dict(entries) => {
                        // Apply bool cast to each value
                        let source = Self::elem_to_source(&elem);
                        let converted: Result<Vec<_>, _> = entries
                            .iter()
                            .map(|(k, v)| {
                                self.apply_cast("bool", v.clone()).map(|cv| (k.clone(), cv))
                            })
                            .collect();
                        converted.map(ParsedElement::Dict).map_err(|e| {
                            // Re-wrap error with full dict context
                            if let Some(caps) = e.message.find("': ") {
                                let reason = &e.message[caps + 3..];
                                ParseError {
                                    message: format!(
                                        "ValueError while evaluating 'bool({})': {}",
                                        source, reason
                                    ),
                                    position: e.position,
                                }
                            } else {
                                e
                            }
                        })
                    }
                    ParsedElement::Null => Ok(ParsedElement::Null),
                }
            }
            "json_str" => {
                // Convert element to JSON string representation
                match &elem {
                    ParsedElement::Int(i) => Ok(ParsedElement::String(i.to_string())),
                    ParsedElement::Float(f) => {
                        // Format float specially - inf/nan as Infinity/NaN
                        if f.is_infinite() {
                            if *f > 0.0 {
                                Ok(ParsedElement::String("Infinity".to_string()))
                            } else {
                                Ok(ParsedElement::String("-Infinity".to_string()))
                            }
                        } else if f.is_nan() {
                            Ok(ParsedElement::String("NaN".to_string()))
                        } else {
                            let s = f.to_string();
                            // Preserve decimal for whole numbers
                            if !s.contains('.') {
                                Ok(ParsedElement::String(format!("{}.0", s)))
                            } else {
                                Ok(ParsedElement::String(s))
                            }
                        }
                    }
                    ParsedElement::String(s) => {
                        // For unquoted strings, check if they look like special values
                        match s.as_str() {
                            "true" | "false" | "null" => Ok(ParsedElement::String(s.clone())),
                            _ => Ok(ParsedElement::String(format!("\"{}\"", s))),
                        }
                    }
                    ParsedElement::QuotedString(qs) => {
                        // Quoted strings get quoted in JSON
                        Ok(ParsedElement::String(format!("\"{}\"", qs.text)))
                    }
                    ParsedElement::Bool(b) => Ok(ParsedElement::String(
                        if *b { "true" } else { "false" }.to_string(),
                    )),
                    ParsedElement::Null => Ok(ParsedElement::String("null".to_string())),
                    ParsedElement::List(items) => {
                        // Convert list to JSON array
                        let parts: Result<Vec<_>, _> = items
                            .iter()
                            .map(|item| {
                                self.apply_cast("json_str", item.clone()).map(|e| {
                                    if let ParsedElement::String(s) = e {
                                        s
                                    } else {
                                        "".to_string()
                                    }
                                })
                            })
                            .collect();
                        parts.map(|p| ParsedElement::String(format!("[{}]", p.join(", "))))
                    }
                    ParsedElement::Dict(entries) => {
                        // Convert dict to JSON object
                        let parts: Result<Vec<_>, _> = entries
                            .iter()
                            .map(|(k, v)| {
                                self.apply_cast("json_str", v.clone()).map(|e| {
                                    let val = if let ParsedElement::String(s) = e {
                                        s
                                    } else {
                                        "".to_string()
                                    };
                                    format!("\"{}\": {}", k, val)
                                })
                            })
                            .collect();
                        parts.map(|p| ParsedElement::String(format!("{{{}}}", p.join(", "))))
                    }
                }
            }
            _ => Err(ParseError {
                message: format!("Unknown cast type: {}", cast_type),
                position: self.pos,
            }),
        }
    }

    fn element_to_f64(&self, elem: &ParsedElement) -> ParseResult<f64> {
        match elem {
            ParsedElement::Int(i) => Ok(*i as f64),
            ParsedElement::Float(f) => Ok(*f),
            ParsedElement::String(s) => s.parse::<f64>().map_err(|_| ParseError {
                message: format!("Expected number, got '{}'", s),
                position: self.pos,
            }),
            _ => Err(ParseError {
                message: "Expected number".to_string(),
                position: self.pos,
            }),
        }
    }

    fn element_to_string_list(&self, elem: &ParsedElement) -> ParseResult<Vec<String>> {
        match elem {
            ParsedElement::String(s) => Ok(vec![s.clone()]),
            ParsedElement::QuotedString(qs) => Ok(vec![qs.text.clone()]),
            ParsedElement::List(items) => items
                .iter()
                .map(|e| match e {
                    ParsedElement::String(s) => Ok(s.clone()),
                    ParsedElement::QuotedString(qs) => Ok(qs.text.clone()),
                    _ => Err(ParseError {
                        message: "Expected string in list".to_string(),
                        position: self.pos,
                    }),
                })
                .collect(),
            _ => Err(ParseError {
                message: "Expected string or list of strings".to_string(),
                position: self.pos,
            }),
        }
    }

    // Helper methods
    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.input.get(self.pos + offset).copied()
    }

    fn current(&self) -> char {
        self.input.get(self.pos).copied().unwrap_or('\0')
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_string() {
        let result = OverrideParser::parse("key=value").unwrap();
        assert_eq!(result.override_type, OverrideType::Change);
        assert_eq!(result.key.key_or_group, "key");
        assert_eq!(
            result.value,
            Some(OverrideValue::Element(ParsedElement::String(
                "value".to_string()
            )))
        );
    }

    #[test]
    fn test_parse_integer() {
        let result = OverrideParser::parse("port=3306").unwrap();
        assert_eq!(
            result.value,
            Some(OverrideValue::Element(ParsedElement::Int(3306)))
        );
    }

    #[test]
    fn test_parse_float() {
        let result = OverrideParser::parse("rate=0.5").unwrap();
        assert_eq!(
            result.value,
            Some(OverrideValue::Element(ParsedElement::Float(0.5)))
        );
    }

    #[test]
    fn test_parse_boolean() {
        let result = OverrideParser::parse("enabled=true").unwrap();
        assert_eq!(
            result.value,
            Some(OverrideValue::Element(ParsedElement::Bool(true)))
        );

        let result = OverrideParser::parse("enabled=false").unwrap();
        assert_eq!(
            result.value,
            Some(OverrideValue::Element(ParsedElement::Bool(false)))
        );
    }

    #[test]
    fn test_parse_null() {
        let result = OverrideParser::parse("db=null").unwrap();
        assert_eq!(
            result.value,
            Some(OverrideValue::Element(ParsedElement::Null))
        );
    }

    #[test]
    fn test_parse_quoted_string() {
        let result = OverrideParser::parse("name='hello world'").unwrap();
        if let Some(OverrideValue::Element(ParsedElement::QuotedString(qs))) = result.value {
            assert_eq!(qs.text, "hello world");
            assert_eq!(qs.quote, Quote::Single);
        } else {
            panic!("Expected quoted string");
        }
    }

    #[test]
    fn test_parse_dotted_key() {
        let result = OverrideParser::parse("db.driver=mysql").unwrap();
        assert_eq!(result.key.key_or_group, "db.driver");
    }

    #[test]
    fn test_parse_add_override() {
        let result = OverrideParser::parse("+db=mysql").unwrap();
        assert_eq!(result.override_type, OverrideType::Add);
        assert_eq!(result.key.key_or_group, "db");
    }

    #[test]
    fn test_parse_force_add_override() {
        let result = OverrideParser::parse("++db=mysql").unwrap();
        assert_eq!(result.override_type, OverrideType::ForceAdd);
    }

    #[test]
    fn test_parse_delete_override() {
        let result = OverrideParser::parse("~db").unwrap();
        assert_eq!(result.override_type, OverrideType::Del);
        assert_eq!(result.key.key_or_group, "db");
        assert!(result.value.is_none());
    }

    #[test]
    fn test_parse_list() {
        let result = OverrideParser::parse("items=[1, 2, 3]").unwrap();
        if let Some(OverrideValue::Element(ParsedElement::List(items))) = result.value {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], ParsedElement::Int(1));
            assert_eq!(items[1], ParsedElement::Int(2));
            assert_eq!(items[2], ParsedElement::Int(3));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_dict() {
        let result = OverrideParser::parse("db={host: localhost, port: 3306}").unwrap();
        if let Some(OverrideValue::Element(ParsedElement::Dict(items))) = result.value {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].0, "host");
            assert_eq!(items[0].1, ParsedElement::String("localhost".to_string()));
            assert_eq!(items[1].0, "port");
            assert_eq!(items[1].1, ParsedElement::Int(3306));
        } else {
            panic!("Expected dict");
        }
    }

    #[test]
    fn test_parse_package() {
        let result = OverrideParser::parse("@pkg:db=mysql").unwrap();
        assert_eq!(result.key.package, Some("pkg".to_string()));
        assert_eq!(result.key.key_or_group, "db");
    }

    #[test]
    fn test_parse_negative_number() {
        let result = OverrideParser::parse("offset=-10").unwrap();
        assert_eq!(
            result.value,
            Some(OverrideValue::Element(ParsedElement::Int(-10)))
        );
    }

    #[test]
    fn test_parse_scientific_notation() {
        let result = OverrideParser::parse("epsilon=1e-6").unwrap();
        if let Some(OverrideValue::Element(ParsedElement::Float(f))) = result.value {
            assert!((f - 1e-6).abs() < 1e-12);
        } else {
            panic!("Expected float");
        }
    }

    // Grammar function tests
    #[test]
    fn test_parse_choice_function() {
        let result = OverrideParser::parse("db=choice(mysql, postgres)").unwrap();
        if let Some(OverrideValue::ChoiceSweep(sweep)) = result.value {
            assert_eq!(sweep.list.len(), 2);
            assert_eq!(sweep.list[0], ParsedElement::String("mysql".to_string()));
            assert_eq!(sweep.list[1], ParsedElement::String("postgres".to_string()));
            assert!(!sweep.simple_form);
        } else {
            panic!("Expected choice sweep");
        }
    }

    #[test]
    fn test_parse_simple_choice() {
        let result = OverrideParser::parse("db=mysql,postgres,sqlite").unwrap();
        if let Some(OverrideValue::ChoiceSweep(sweep)) = result.value {
            assert_eq!(sweep.list.len(), 3);
            assert!(sweep.simple_form);
        } else {
            panic!("Expected simple choice sweep");
        }
    }

    #[test]
    fn test_parse_range_function() {
        let result = OverrideParser::parse("x=range(1, 10)").unwrap();
        if let Some(OverrideValue::RangeSweep(sweep)) = result.value {
            assert_eq!(sweep.start, Some(1.0));
            assert_eq!(sweep.stop, Some(10.0));
            assert_eq!(sweep.step, 1.0);
        } else {
            panic!("Expected range sweep");
        }
    }

    #[test]
    fn test_parse_range_with_step() {
        let result = OverrideParser::parse("x=range(0, 100, 10)").unwrap();
        if let Some(OverrideValue::RangeSweep(sweep)) = result.value {
            assert_eq!(sweep.start, Some(0.0));
            assert_eq!(sweep.stop, Some(100.0));
            assert_eq!(sweep.step, 10.0);
        } else {
            panic!("Expected range sweep");
        }
    }

    #[test]
    fn test_parse_interval_function() {
        let result = OverrideParser::parse("lr=interval(0.0, 1.0)").unwrap();
        if let Some(OverrideValue::IntervalSweep(sweep)) = result.value {
            assert_eq!(sweep.start, Some(0.0));
            assert_eq!(sweep.end, Some(1.0));
        } else {
            panic!("Expected interval sweep");
        }
    }

    #[test]
    fn test_parse_cast_int() {
        let result = OverrideParser::parse("x=int(3.14)").unwrap();
        if let Some(OverrideValue::Element(ParsedElement::Int(i))) = result.value {
            assert_eq!(i, 3);
        } else {
            panic!("Expected int");
        }
    }

    #[test]
    fn test_parse_cast_float() {
        let result = OverrideParser::parse("x=float(42)").unwrap();
        if let Some(OverrideValue::Element(ParsedElement::Float(f))) = result.value {
            assert_eq!(f, 42.0);
        } else {
            panic!("Expected float");
        }
    }

    #[test]
    fn test_parse_cast_str() {
        let result = OverrideParser::parse("x=str(123)").unwrap();
        if let Some(OverrideValue::Element(ParsedElement::String(s))) = result.value {
            assert_eq!(s, "123");
        } else {
            panic!("Expected string");
        }
    }
}
