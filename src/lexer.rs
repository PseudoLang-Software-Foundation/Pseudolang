#[derive(Debug, PartialEq, Clone)]
#[allow(dead_code)]
pub enum Token {
    Unknown,
    Identifier(String),

    Assignment,
    Assign,
    Display(Option<Box<Token>>),
    DisplayInline,
    Input,

    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,

    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    And,
    Or,
    Not,

    If,
    Else,
    Repeat,
    RepeatUntil,
    Until,
    Times,

    ListCreate(Vec<Token>),
    ListAssign,
    ListAccess,
    ListInsert,
    ListAppend,
    ListRemove,
    ListLength,
    ForEach,

    Procedure,
    Return,

    Integer(i64),
    Float(f64),
    String(String),
    RawString(String),
    MultilineString(String),
    FormattedString(String, Vec<String>),
    Boolean(bool),

    Comment,
    CommentBlock,

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
    Sort,
    Try,
    Catch,

    Null,
    NaN,
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
                    while let Some(c) = self.chars.next() {
                        self.pos += 1;
                        if c == '\n' {
                            break;
                        }
                    }
                    continue;
                }
                Token::CommentBlock => {
                    let mut found_end = false;
                    while self.chars.next().is_some() {
                        self.pos += 1;

                        if self.input[self.pos..].starts_with("COMMENTBLOCK") {
                            for _ in 0.."COMMENTBLOCK".len() {
                                self.chars.next();
                                self.pos += 1;
                            }
                            found_end = true;
                            break;
                        }
                    }
                    if !found_end {
                        return tokens;
                    }
                    continue;
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
            '/' => {
                if let Some(&'/') = self.chars.peek() {
                    self.chars.next();
                    self.pos += 1;

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
                    Some(Token::Assign)
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
                        let mut brace_count = 1;
                        while let Some(c) = self.chars.next() {
                            self.pos += 1;
                            if c == '{' {
                                brace_count += 1;
                                var.push(c);
                            } else if c == '}' {
                                brace_count -= 1;
                                if brace_count == 0 {
                                    break;
                                } else {
                                    var.push(c);
                                }
                            } else {
                                var.push(c);
                            }
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
                    let mut string = String::new();
                    while let Some(c) = self.chars.next() {
                        self.pos += 1;
                        if c == '\\' {
                            if let Some(escaped_char) = self.chars.next() {
                                self.pos += 1;
                                match escaped_char {
                                    'n' => string.push('\n'),
                                    't' => string.push('\t'),
                                    'r' => string.push('\r'),
                                    'b' => string.push('\x08'),
                                    '\\' => string.push('\\'),
                                    '"' => string.push('"'),
                                    _ => string.push(escaped_char),
                                }
                            }
                        } else if c == '"' {
                            break;
                        } else {
                            string.push(c);
                        }
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
                let mut identifier = String::from('N');
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
                    "NULL" => Some(Token::Null),
                    "NAN" => Some(Token::NaN),
                    "NOT" => {
                        if self.chars.peek() == Some(&'=') {
                            self.chars.next();
                            self.pos += 1;
                            Some(Token::NotEqual)
                        } else {
                            Some(Token::Not)
                        }
                    }
                    _ => Some(Token::Identifier(identifier)),
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
                    "NULL" => Some(Token::Null),
                    "NAN" => Some(Token::NaN),
                    "MOD" => Some(Token::Modulo),
                    "DISPLAY" => {
                        while let Some(&c) = self.chars.peek() {
                            if c.is_whitespace() {
                                self.chars.next();
                                self.pos += 1;
                                continue;
                            }
                            break;
                        }

                        if let Some(&'"') = self.chars.peek() {
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
                            Some(Token::Display(Some(Box::new(Token::String(string)))))
                        } else {
                            Some(Token::Display(None))
                        }
                    }
                    "DISPLAYINLINE" => Some(Token::DisplayInline),
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
                    "TRIM" => Some(Token::Identifier("TRIM".to_string())),
                    "REPLACE" => Some(Token::Identifier("REPLACE".to_string())),
                    "UPPERCASE" => Some(Token::Identifier("UPPERCASE".to_string())),
                    "LOWERCASE" => Some(Token::Identifier("LOWERCASE".to_string())),
                    "EACH" => Some(Token::Each),
                    "IN" => Some(Token::In),
                    "PROCEDURE" => Some(Token::Procedure),
                    "SUBSTRING" => Some(Token::Substring),
                    "CONCAT" => Some(Token::Concat),
                    "IMPORT" => Some(Token::Import),
                    "UNTIL" => Some(Token::Until),
                    "TIMES" => Some(Token::Times),
                    "NOT=" => Some(Token::NotEqual),
                    "INSERT" => Some(Token::ListInsert),
                    "APPEND" => Some(Token::ListAppend),
                    "REMOVE" => Some(Token::ListRemove),
                    "LENGTH" => Some(Token::ListLength),
                    "RANDOM" => Some(Token::Random),
                    "SORT" => Some(Token::Sort),
                    "TRY" => Some(Token::Try),
                    "CATCH" => Some(Token::Catch),
                    _ => Some(Token::Identifier(identifier)),
                }
            }
            _ => Some(Token::Identifier(next_char.to_string())),
        }
    }
}
