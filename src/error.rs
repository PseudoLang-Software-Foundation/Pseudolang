use regex::Regex;
use std::fmt;

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub line_content: String,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize, line_content: String) -> Self {
        Self {
            line,
            column,
            line_content,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PseudoError {
    pub message: String,
    pub location: Option<SourceLocation>,
}

impl PseudoError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            location: None,
        }
    }

    pub fn with_location(message: &str, line: usize, column: usize, line_content: String) -> Self {
        Self {
            message: message.to_string(),
            location: Some(SourceLocation::new(line, column, line_content)),
        }
    }

    pub fn format(&self) -> String {
        if let Some(loc) = &self.location {
            let pointer = " ".repeat(loc.column) + "^";

            let line_content = if loc.line_content.trim().is_empty() {
                "[empty line]".to_string()
            } else {
                loc.line_content.clone()
            };

            format!(
                "Line {}, Column {}: {}\n{}\n{}",
                loc.line,
                loc.column,
                self.format_message(),
                line_content,
                pointer
            )
        } else {
            self.message.clone()
        }
    }

    fn format_message(&self) -> String {
        let mut message = self.message.clone();

        if let Some(caps) = Regex::new(r"List index out of bounds: (\d+) \(size: (\d+)\)")
            .ok()
            .and_then(|re| re.captures(&self.message))
        {
            let index = caps.get(1).unwrap().as_str();
            let size = caps.get(2).unwrap().as_str();
            message = format!(
                "List index out of bounds: index {} exceeds list length {}",
                index, size
            );
        } else if let Some(caps) = Regex::new(r"String index out of bounds: (\d+) \(size: (\d+)\)")
            .ok()
            .and_then(|re| re.captures(&self.message))
        {
            let index = caps.get(1).unwrap().as_str();
            let size = caps.get(2).unwrap().as_str();
            message = format!(
                "String index out of bounds: index {} exceeds string length {}",
                index, size
            );
        } else if self.message.contains("Division by zero") {
            message = "Division by zero error: cannot divide by zero".to_string();
        } else if self.message.contains("Modulo by zero") {
            message = "Modulo by zero error: cannot perform modulo operation with zero divisor"
                .to_string();
        } else if let Some(caps) = Regex::new(r"Undefined variable: (.+)")
            .ok()
            .and_then(|re| re.captures(&self.message))
        {
            let var_name = caps.get(1).unwrap().as_str();
            message = format!(
                "Undefined variable: '{}' is not defined in the current scope",
                var_name
            );
        } else if let Some(caps) = Regex::new(r"Variable (.+) is not a list")
            .ok()
            .and_then(|re| re.captures(&self.message))
        {
            let var_name = caps.get(1).unwrap().as_str();
            message = format!(
                "Type error: '{}' is not a list, but list operations were attempted on it",
                var_name
            );
        } else if self.message.contains("List index out of bounds") {
            if self.message.contains("cannot be less than 1") {
                message =
                    "List index out of bounds: index cannot be less than 1 (lists are 1-indexed)"
                        .to_string();
            } else {
                message =
                    "List index out of bounds: index is outside the valid range for this list"
                        .to_string();
            }
        } else if self.message.contains("String index out of bounds") {
            if self.message.contains("cannot be less than 1") {
                message = "String index out of bounds: index cannot be less than 1 (strings are 1-indexed)".to_string();
            } else {
                message =
                    "String index out of bounds: index is outside the valid range for this string"
                        .to_string();
            }
        } else if self.message.contains("Stack overflow") {
            message = "Stack overflow: maximum recursion depth exceeded. Check for infinite recursion in your code.".to_string();
        } else if self.message.contains("Maximum loop iterations exceeded") {
            message = "Maximum loop iterations exceeded: your loop may be infinite. Check your loop condition.".to_string();
        } else if let Some(caps) = Regex::new(r"Procedure '(.+)' not found")
            .ok()
            .and_then(|re| re.captures(&self.message))
        {
            let proc_name = caps.get(1).unwrap().as_str();
            message = format!("Procedure not found: '{}' is not defined. Check for typos or ensure it's defined before use.", proc_name);
        } else if self.message.contains("Cannot convert string to number") {
            message =
                "Cannot convert string to number: the string does not represent a valid number"
                    .to_string();
        } else if self.message.contains("Condition must be a boolean") {
            message = "Type error: condition must be a boolean expression".to_string();
        } else if self.message.contains("REPEAT count must be an integer") {
            message = "Type error: REPEAT count must be an integer value".to_string();
        } else if self.message.contains("Expected boolean in condition") {
            message = "Type error: expected boolean in condition, got a different type".to_string();
        } else if let Some(caps) = Regex::new(r"(.+) requires .+ argument")
            .ok()
            .and_then(|re| re.captures(&self.message))
        {
            // Preserving function requirement errors but making them more detailed
            let _func_name = caps.get(1).unwrap().as_str();
            message = format!("Function argument error: {}", self.message);
        }

        message
    }
}

impl fmt::Display for PseudoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

impl From<PseudoError> for String {
    fn from(error: PseudoError) -> Self {
        error.format()
    }
}

impl From<&PseudoError> for String {
    fn from(error: &PseudoError) -> Self {
        error.format()
    }
}

/// Helper function to convert String errors to PseudoError
/// This is for backwards compatibility with existing code
#[allow(dead_code)]
pub fn from_string(error: String) -> PseudoError {
    PseudoError::new(&error)
}

#[derive(Debug, Clone)]
pub struct SourceTracker {
    source: String,
    lines: Vec<String>,
}

impl SourceTracker {
    pub fn new(source: &str) -> Self {
        let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        Self {
            source: source.to_string(),
            lines,
        }
    }

    pub fn get_location(&self, pos: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;

        for (i, ch) in self.source.char_indices() {
            if i >= pos {
                break;
            }

            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        (line, col)
    }

    pub fn get_line_content(&self, line: usize) -> String {
        if line == 0 || line > self.lines.len() {
            String::new()
        } else {
            self.lines[line - 1].clone()
        }
    }

    pub fn scan_for_error_location(
        &self,
        error_type: &str,
        details: &str,
    ) -> Option<(usize, usize)> {
        if error_type.contains("Undefined variable") {
            if let Some(var_name) = details.strip_prefix("'").and_then(|s| s.strip_suffix("'")) {
                for (line_num, line) in self.lines.iter().enumerate() {
                    if line.contains(&format!("DISPLAY({}", var_name)) {
                        let var_pos = line.find(var_name).unwrap_or(0);
                        return Some((line_num + 1, var_pos));
                    }
                }

                for (line_num, line) in self.lines.iter().enumerate() {
                    if line.trim_start().starts_with("COMMENT") {
                        continue;
                    }

                    if line.contains(var_name)
                        && !line.contains(&format!("{} <-", var_name))
                        && !line.contains(&format!("FUNCTION {}", var_name))
                    {
                        let var_pos = line.find(var_name).unwrap_or(0);
                        return Some((line_num + 1, var_pos));
                    }
                }
            }
        } else if error_type.contains("List index out of bounds")
            || error_type.contains("String index out of bounds")
        {
            if let Some(caps) = Regex::new(r"index (\d+) exceeds .* length (\d+)")
                .ok()
                .and_then(|re| re.captures(error_type))
            {
                let index = caps.get(1).unwrap().as_str();

                for (line_num, line) in self.lines.iter().enumerate() {
                    if line.trim_start().starts_with("COMMENT") {
                        continue;
                    }

                    let pattern = format!("[{}]", index);
                    if line.contains(&pattern) {
                        let idx_pos = line.find(&pattern).unwrap_or(0);

                        return Some((line_num + 1, idx_pos + 1));
                    }
                }
            }
        }

        None
    }

    pub fn find_error_position(&self, error_message: &str) -> usize {
        let position = 0;

        if error_message.contains("List index out of bounds")
            || error_message.contains("String index out of bounds")
        {
            if let Some(caps) = Regex::new(r"index (\d+) exceeds .* length (\d+)")
                .ok()
                .and_then(|re| re.captures(error_message))
            {
                let index = caps.get(1).unwrap().as_str();

                for (line_num, line) in self.lines.iter().enumerate() {
                    let pattern = format!("[{}]", index);
                    if let Some(idx_pos) = line.find(&pattern) {
                        let mut pos = 0;
                        for i in 0..line_num {
                            pos += self.lines[i].len() + 1;
                        }
                        return pos + idx_pos + 1;
                    }
                }
            }

            for (line_num, line) in self.lines.iter().enumerate() {
                if line.contains("[") && line.contains("]") {
                    if let Some(bracket_pos) = line.find("[") {
                        let mut pos = 0;
                        for i in 0..line_num {
                            pos += self.lines[i].len() + 1;
                        }
                        return pos + bracket_pos + 1;
                    }
                }
            }
        } else if error_message.contains("Division by zero")
            || error_message.contains("Modulo by zero")
        {
            for (line_num, line) in self.lines.iter().enumerate() {
                if line.contains("/") || line.contains("%") {
                    let op_pos = if line.contains("/") {
                        line.find("/")
                    } else {
                        line.find("%")
                    };

                    if let Some(op_pos) = op_pos {
                        let mut pos = 0;
                        for i in 0..line_num {
                            pos += self.lines[i].len() + 1;
                        }
                        return pos + op_pos;
                    }
                }
            }
        } else if error_message.contains("Undefined variable:") {
            if let Some(caps) = Regex::new(r"'([^']+)'")
                .ok()
                .and_then(|re| re.captures(error_message))
            {
                let var_name = caps.get(1).unwrap().as_str();

                for (line_num, line) in self.lines.iter().enumerate() {
                    if !line.trim_start().starts_with("COMMENT") && line.contains(var_name) {
                        if !line.contains(&format!("{} <-", var_name))
                            && !line.contains(&format!("{}:", var_name))
                        {
                            let var_pos = line.find(var_name).unwrap_or(0);
                            let mut pos = 0;
                            for i in 0..line_num {
                                pos += self.lines[i].len() + 1;
                            }
                            return pos + var_pos;
                        }
                    }
                }
            }
        }

        position
    }

    pub fn create_error(&self, message: &str, pos: usize) -> PseudoError {
        let (line, column) = self.get_location(pos);
        let line_content = self.get_line_content(line);

        PseudoError::with_location(message, line, column, line_content)
    }

    /// Convert a String error to a PseudoError with location information
    #[allow(dead_code)]
    pub fn wrap_error(&self, message: String, pos: usize) -> PseudoError {
        self.create_error(&message, pos)
    }

    pub fn create_smart_error(&self, message: &str) -> PseudoError {
        if message.contains("Undefined variable:") {
            if let Some(var_name) = Self::extract_var_name(message) {
                if let Some((line, column)) = self.find_error_line(&var_name, Some("<-")) {
                    let line_content = self.get_line_content(line);
                    return PseudoError::with_location(message, line, column, line_content);
                }
            }
        }

        if message.contains("List index out of bounds:")
            || message.contains("String index out of bounds:")
        {
            if let Some(caps) = Regex::new(r"index (\d+) exceeds")
                .ok()
                .and_then(|re| re.captures(message))
            {
                let index = caps.get(1).unwrap().as_str();
                let pattern = format!("[{}]", index);

                if let Some((line, column)) = self.find_error_line(&pattern, None) {
                    let line_content = self.get_line_content(line);
                    return PseudoError::with_location(message, line, column, line_content);
                }
            }
        }

        let error_parts: Vec<&str> = message.split(": ").collect();

        if error_parts.len() >= 2 {
            let error_type = error_parts[0];
            let details = error_parts[1];

            if let Some((line, column)) = self.scan_for_error_location(error_type, details) {
                let line_content = self.get_line_content(line);
                return PseudoError::with_location(message, line, column, line_content);
            }
        }

        // Last resort: use position-based error
        let pos = self.find_error_position(message);
        self.create_error(message, pos)
    }

    pub fn find_error_line(
        &self,
        element: &str,
        exclude_pattern: Option<&str>,
    ) -> Option<(usize, usize)> {
        for (line_num, line_content) in self.lines.iter().enumerate() {
            if let Some(pattern) = exclude_pattern {
                if line_content.contains(pattern) {
                    continue;
                }
            }

            if line_content.trim_start().starts_with("COMMENT") {
                continue;
            }

            if line_content.contains(element) {
                let col = line_content.find(element).unwrap_or(0);
                return Some((line_num + 1, col));
            }
        }
        None
    }

    pub fn extract_var_name(error_msg: &str) -> Option<String> {
        if let Some(caps) = Regex::new(r"Undefined variable: '?([^']+)'?")
            .ok()
            .and_then(|re| re.captures(error_msg))
        {
            Some(caps.get(1).unwrap().as_str().to_string())
        } else {
            None
        }
    }
}
