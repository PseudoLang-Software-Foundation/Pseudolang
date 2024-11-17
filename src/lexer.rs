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

    Integer(i32),
    Float(f32),
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
                        if c == '\n' {
                            break;
                        }
                    }
                }
                Token::CommentBlock => {
                    while let Some(_) = self.chars.next() {
                        self.pos += 1;

                        if self.input[self.pos..].starts_with("COMMENTBLOCK") {
                            for _ in 0.."COMMENTBLOCK".len() {
                                self.chars.next();
                                self.pos += 1;
                            }
                            break;
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
                if self.pos + 2 <= self.input.len()
                    && &self.input[self.pos - 1..self.pos + 2] == "NOT"
                {
                    self.chars.next();
                    self.chars.next();
                    self.pos += 2;

                    if self.chars.peek() == Some(&'=') {
                        self.chars.next();
                        self.pos += 1;
                        Some(Token::NotEqual)
                    } else {
                        Some(Token::Not)
                    }
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
                    "SORT" => Some(Token::Sort),
                    _ => Some(Token::Identifier(identifier)),
                }
            }
            _ => Some(Token::Identifier(next_char.to_string())),
        }
    }
}
