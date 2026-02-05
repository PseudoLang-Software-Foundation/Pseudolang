use std::fmt;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[allow(dead_code)]
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start,
            end: other.end,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PSLError {
    pub message: String,
    pub span: Option<Span>,
    pub stack_trace: Vec<StackFrame>,
}

pub fn resolve_span(source: &str, span: &Span) -> (usize, usize, String) {
    let mut line = 1;
    let mut col = 1;
    let mut line_start = 0;

    for (i, ch) in source.char_indices() {
        if i >= span.start {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
            line_start = i + 1;
        } else {
            col += 1;
        }
    }

    let line_end = source[line_start..]
        .find('\n')
        .map_or(source.len(), |pos| line_start + pos);
    let line_content = source[line_start..line_end].to_string();

    (line, col, line_content)
}

impl PSLError {
    #[allow(dead_code)]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
            stack_trace: Vec::new(),
        }
    }

    pub fn with_span(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span: Some(span),
            stack_trace: Vec::new(),
        }
    }

    pub fn format(&self, source: &str) -> String {
        match self.span {
            Some(span) => {
                let (line, col, line_content) = resolve_span(source, &span);
                let trimmed = line_content.trim_start();
                let indent = line_content.len() - trimmed.len();
                let adjusted_col = if col > indent { col - indent } else { col };

                let mut result = format!(
                    "Line {}, Column {}: {}\n    {}\n    {}^",
                    line,
                    col,
                    self.message,
                    trimmed,
                    " ".repeat(adjusted_col.saturating_sub(1))
                );

                for frame in &self.stack_trace {
                    let (frame_line, _col, _line_content) = resolve_span(source, &frame.span);
                    result.push_str(&format!("\n  in {} (line {})", frame.name, frame_line));
                }

                result
            }
            None => self.message.clone(),
        }
    }
}

impl fmt::Display for PSLError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
