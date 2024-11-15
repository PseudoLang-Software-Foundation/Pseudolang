#[derive(Debug, PartialEq, Clone)]
#[allow(dead_code)]
pub enum Token {
    Unknown,
    Identifier(String),

    // Assignment, Display, Input
    Assignment,
    Assign,
    Display(Option<Box<Token>>), // For DISPLAY("hi") -> Display(Some(Box::new(String("hi"))))
    DisplayInline,
    Input,

    // Arithmetic Operators
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,

    // Relational and Boolean Operators
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    And,
    Or,
    Not,

    // Selection
    If,
    Else,
    Repeat,
    RepeatUntil,
    Until,
    Times, // For REPEAT 5 TIMES -> Repeat(5)

    // List operations
    ListCreate(Vec<Token>), // For ["1", 1] -> ListCreate(vec![String("1"), Integer(1)]), unsure about this for the future
    ListAssign,
    ListAccess,
    ListInsert,
    ListAppend,
    ListRemove,
    ListLength,
    ForEach,

    // Procedures
    Procedure,
    Return,

    // Data Types
    Integer(i32),
    Float(f32),
    String(String),
    RawString(String),
    MultilineString(String),
    FormattedString(String, Vec<String>),
    Boolean(bool),

    // Comments
    Comment,
    CommentBlock,

    // Special chars
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    Comma,
    Indent,
    Dedent,
    Newline,
    OpenBrace,
    CloseBrace,

    // Miscellaneous Operations
    Class,
    ToString,
    ToNum,
    For,
    Each,
    In,
    Substring,
    Concat,
    Import,

    True,
    False,
    Random,
}

pub struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            chars: input.chars().peekable(),
            input,
            pos: 0,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token() {
            match token {
                Token::Comment => {
                    // Skip rest of line for comments
                    while let Some(c) = self.chars.next() {
                        if c == '\n' {
                            break;
                        }
                    }
                }
                Token::CommentBlock => {
                    let mut depth = 1;
                    while depth > 0 {
                        match self.chars.next() {
                            Some('C') => {
                                let block = self.input[self.pos..].starts_with("COMMENTBLOCK");
                                if block {
                                    depth -= 1;
                                    for _ in 0..11 {
                                        self.chars.next();
                                    }
                                }
                            }
                            Some(_) => {}
                            None => break,
                        }
                    }
                }
                _ => tokens.push(token),
            }
        }
        tokens
    }

    fn next_token(&mut self) -> Option<Token> {
        let next_char = self.chars.next()?;
        self.pos += 1;

        match next_char {
            // Add handling for C-style comments
            '/' => {
                if let Some(&'/') = self.chars.peek() {
                    // Skip the second '/'
                    self.chars.next();
                    self.pos += 1;

                    // Skip the rest of the line
                    while let Some(c) = self.chars.next() {
                        self.pos += 1;
                        if c == '\n' {
                            return self.next_token();
                        }
                    }
                    self.next_token()
                } else {
                    Some(Token::Divide)
                }
            }
            // Change whitespace handling to preserve newlines
            '\n' => Some(Token::Newline),
            ' ' | '\t' | '\r' => self.next_token(),
            '{' => Some(Token::OpenBrace),
            '}' => Some(Token::CloseBrace),
            '=' => Some(Token::Equal),
            '>' => {
                if self.chars.peek() == Some(&'=') {
                    self.chars.next();
                    self.pos += 1;
                    Some(Token::GreaterThanOrEqual)
                } else {
                    Some(Token::GreaterThan)
                }
            }
            '<' => {
                if self.chars.peek() == Some(&'-') {
                    self.chars.next();
                    self.pos += 1;
                    Some(Token::Assign) // Changed from Assignment
                } else if self.chars.peek() == Some(&'=') {
                    self.chars.next();
                    self.pos += 1;
                    Some(Token::LessThanOrEqual)
                } else {
                    Some(Token::LessThan)
                }
            }
            '+' => Some(Token::Plus),
            '-' => Some(Token::Minus),
            '*' => Some(Token::Multiply),
            '/' => Some(Token::Divide),
            '(' => Some(Token::OpenParen),
            ')' => Some(Token::CloseParen),
            '[' => Some(Token::OpenBracket),
            ']' => Some(Token::CloseBracket),
            ',' => Some(Token::Comma),

            'r' if self.chars.peek() == Some(&'"') => {
                self.chars.next();
                self.pos += 1;
                let mut string = String::new();
                while let Some(c) = self.chars.next() {
                    self.pos += 1;
                    if c == '"' {
                        break;
                    }
                    string.push(c);
                }
                Some(Token::RawString(string))
            }

            'f' if self.chars.peek() == Some(&'"') => {
                self.chars.next();
                self.pos += 1;
                let mut string = String::new();
                let mut vars = Vec::new();
                while let Some(c) = self.chars.next() {
                    self.pos += 1;
                    if c == '"' {
                        break;
                    }
                    if c == '{' {
                        let mut var = String::new();
                        while let Some(c) = self.chars.next() {
                            self.pos += 1;
                            if c == '}' {
                                break;
                            }
                            var.push(c);
                        }
                        vars.push(var);
                        string.push_str("{}");
                    } else {
                        string.push(c);
                    }
                }
                Some(Token::FormattedString(string, vars))
            }

            '"' => {
                if self.chars.peek() == Some(&'"') && self.chars.clone().nth(1) == Some('"') {
                    // Multiline string
                    self.chars.next();
                    self.chars.next();
                    self.pos += 2;
                    let mut string = String::new();
                    while let Some(c) = self.chars.next() {
                        self.pos += 1;
                        if c == '"'
                            && self.chars.peek() == Some(&'"')
                            && self.chars.clone().nth(1) == Some('"')
                        {
                            self.chars.next();
                            self.chars.next();
                            self.pos += 2;
                            break;
                        }
                        string.push(c);
                    }
                    Some(Token::MultilineString(string))
                } else {
                    // Regular string
                    let mut string = String::new();
                    while let Some(c) = self.chars.next() {
                        self.pos += 1;
                        if c == '"' {
                            break;
                        }
                        string.push(c);
                    }
                    Some(Token::String(string))
                }
            }

            '0'..='9' => {
                let mut number = String::from(next_char);
                let mut is_float = false;

                while let Some(&c) = self.chars.peek() {
                    if c == '.' && !is_float {
                        is_float = true;
                        number.push(c);
                        self.chars.next();
                        self.pos += 1;
                    } else if c.is_digit(10) {
                        number.push(c);
                        self.chars.next();
                        self.pos += 1;
                    } else {
                        break;
                    }
                }

                if is_float {
                    Some(Token::Float(number.parse().unwrap()))
                } else {
                    Some(Token::Integer(number.parse().unwrap()))
                }
            }

            'N' => {
                // Fix: Handle NOT= as a special case
                if self.input[self.pos..].starts_with("OT=") {
                    for _ in 0..3 {
                        // Skip "OT="
                        self.chars.next();
                        self.pos += 1;
                    }
                    Some(Token::NotEqual)
                } else {
                    Some(Token::Identifier("N".to_string()))
                }
            }

            c @ ('a'..='z' | 'A'..='Z') => {
                let mut identifier = String::from(c);
                while let Some(&c) = self.chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        identifier.push(c);
                        self.chars.next();
                        self.pos += 1;
                    } else {
                        break;
                    }
                }

                match identifier.as_str() {
                    // Make sure MOD is recognized before general identifiers
                    "MOD" => Some(Token::Modulo),
                    "DISPLAY" => {
                        // Skip any whitespace
                        while let Some(&c) = self.chars.peek() {
                            if c.is_whitespace() {
                                self.chars.next();
                                self.pos += 1;
                                continue;
                            }
                            break;
                        }

                        // Check if there's a string literal immediately after DISPLAY
                        if let Some(&'"') = self.chars.peek() {
                            self.chars.next(); // consume quote
                            self.pos += 1;
                            let mut string = String::new();
                            while let Some(c) = self.chars.next() {
                                self.pos += 1;
                                if c == '"' {
                                    break;
                                }
                                string.push(c);
                            }
                            Some(Token::Display(Some(Box::new(Token::String(string)))))
                        } else {
                            // Otherwise just return Display token
                            Some(Token::Display(None))
                        }
                    }
                    "INPUT" => Some(Token::Input),
                    "IF" => Some(Token::If),
                    "ELSE" => Some(Token::Else),
                    "REPEAT" => Some(Token::Repeat),
                    "NOT" => Some(Token::Not),
                    "AND" => Some(Token::And),
                    "OR" => Some(Token::Or),
                    "COMMENT" => Some(Token::Comment),
                    "COMMENTBLOCK" => Some(Token::CommentBlock),
                    "RETURN" => Some(Token::Return),
                    "TRUE" => Some(Token::Boolean(true)),
                    "FALSE" => Some(Token::Boolean(false)),
                    "CLASS" => Some(Token::Class),
                    "TOSTRING" => Some(Token::ToString),
                    "TONUM" => Some(Token::ToNum),
                    "FOR" => Some(Token::For),
                    "EACH" => Some(Token::Each),
                    "IN" => Some(Token::In),
                    "PROCEDURE" => Some(Token::Procedure),
                    "SUBSTRING" => Some(Token::Substring),
                    "CONCAT" => Some(Token::Concat),
                    "IMPORT" => Some(Token::Import),
                    "DISPLAYINLINE" => Some(Token::DisplayInline),
                    "UNTIL" => Some(Token::Until),
                    "TIMES" => Some(Token::Times),
                    "NOT=" => Some(Token::NotEqual),
                    "INSERT" => Some(Token::ListInsert),
                    "APPEND" => Some(Token::ListAppend),
                    "REMOVE" => Some(Token::ListRemove),
                    "LENGTH" => Some(Token::ListLength),
                    "RANDOM" => Some(Token::Random),
                    _ => Some(Token::Identifier(identifier)),
                }
            }
            _ => Some(Token::Identifier(next_char.to_string())),
        }
    }
}
