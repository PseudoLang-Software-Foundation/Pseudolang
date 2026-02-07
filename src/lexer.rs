use crate::error::Span;
use num_bigint::BigInt;

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

    Integer(BigInt),
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
    Eval,
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

    pub fn tokenize(&mut self) -> Vec<(Token, Span)> {
        let mut tokens = Vec::new();
        while let Some((token, span)) = self.next_token() {
            match token {
                Token::Comment => {
                    for c in self.chars.by_ref() {
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
                _ => tokens.push((token, span)),
            }
        }
        tokens
    }

    // skipcq: RS-R1000
    fn next_token(&mut self) -> Option<(Token, Span)> {
        let next_char = self.chars.next()?;
        let token_start = self.pos;
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
                    Some((Token::Divide, Span::new(token_start, self.pos)))
                }
            }

            '#' => {
                while let Some(c) = self.chars.next() {
                    self.pos += 1;
                    if c == '\n' {
                        return self.next_token();
                    }
                }
                self.next_token()
            }

            '\n' => Some((Token::Newline, Span::new(token_start, self.pos))),
            ' ' | '\t' | '\r' => self.next_token(),
            '{' => Some((Token::OpenBrace, Span::new(token_start, self.pos))),
            '}' => Some((Token::CloseBrace, Span::new(token_start, self.pos))),
            '=' => Some((Token::Equal, Span::new(token_start, self.pos))),
            '>' => {
                if self.chars.peek() == Some(&'=') {
                    self.chars.next();
                    self.pos += 1;
                    Some((Token::GreaterThanOrEqual, Span::new(token_start, self.pos)))
                } else {
                    Some((Token::GreaterThan, Span::new(token_start, self.pos)))
                }
            }
            '<' => {
                if self.chars.peek() == Some(&'-') {
                    self.chars.next();
                    self.pos += 1;
                    Some((Token::Assign, Span::new(token_start, self.pos)))
                } else if self.chars.peek() == Some(&'=') {
                    self.chars.next();
                    self.pos += 1;
                    Some((Token::LessThanOrEqual, Span::new(token_start, self.pos)))
                } else {
                    Some((Token::LessThan, Span::new(token_start, self.pos)))
                }
            }
            '+' => Some((Token::Plus, Span::new(token_start, self.pos))),
            '-' => Some((Token::Minus, Span::new(token_start, self.pos))),
            '*' => Some((Token::Multiply, Span::new(token_start, self.pos))),
            '(' => Some((Token::OpenParen, Span::new(token_start, self.pos))),
            ')' => Some((Token::CloseParen, Span::new(token_start, self.pos))),
            '[' => Some((Token::OpenBracket, Span::new(token_start, self.pos))),
            ']' => Some((Token::CloseBracket, Span::new(token_start, self.pos))),
            ',' => Some((Token::Comma, Span::new(token_start, self.pos))),

            'r' if self.chars.peek() == Some(&'"') => {
                self.chars.next();
                self.pos += 1;
                let mut string = String::new(); // skipcq: RS-W1079
                for c in self.chars.by_ref() {
                    self.pos += 1;
                    if c == '"' {
                        break;
                    }
                    string.push(c);
                }
                Some((Token::RawString(string), Span::new(token_start, self.pos)))
            }

            'f' if self.chars.peek() == Some(&'"') => {
                self.chars.next();
                self.pos += 1;
                let mut string = String::new(); // skipcq: RS-W1079
                let mut vars = Vec::new(); // skipcq: RS-W1079
                while let Some(c) = self.chars.next() {
                    self.pos += 1;
                    if c == '"' {
                        break;
                    }
                    if c == '{' {
                        let mut var = String::new(); // skipcq: RS-W1079
                        let mut brace_count = 1;
                        for c in self.chars.by_ref() {
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
                Some((
                    Token::FormattedString(string, vars),
                    Span::new(token_start, self.pos),
                ))
            }

            '"' => {
                if self.chars.peek() == Some(&'"') && self.chars.clone().nth(1) == Some('"') {
                    self.chars.next();
                    self.chars.next();
                    self.pos += 2;
                    let mut string = String::new(); // skipcq: RS-W1079
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
                    Some((
                        Token::MultilineString(string),
                        Span::new(token_start, self.pos),
                    ))
                } else {
                    let mut string = String::new(); // skipcq: RS-W1079
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
                    Some((Token::String(string), Span::new(token_start, self.pos)))
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
                    } else if c.is_ascii_digit() {
                        number.push(c);
                        self.chars.next();
                        self.pos += 1;
                    } else {
                        break;
                    }
                }

                let span = Span::new(token_start, self.pos);
                if is_float {
                    Some((Token::Float(number.parse().unwrap()), span))
                } else {
                    Some((Token::Integer(number.parse().unwrap()), span))
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

                let token = match identifier.as_str() {
                    "NULL" => Token::Null,
                    "NAN" => Token::NaN,
                    "NOT" => {
                        if self.chars.peek() == Some(&'=') {
                            self.chars.next();
                            self.pos += 1;
                            Token::NotEqual
                        } else {
                            Token::Not
                        }
                    }
                    _ => Token::Identifier(identifier),
                };
                Some((token, Span::new(token_start, self.pos)))
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
                    "NULL" => Some((Token::Null, Span::new(token_start, self.pos))),
                    "NAN" => Some((Token::NaN, Span::new(token_start, self.pos))),
                    "MOD" => Some((Token::Modulo, Span::new(token_start, self.pos))),
                    "DISPLAY" => {
                        let before_ws = self.pos;
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
                            let mut string = String::new(); // skipcq: RS-W1079
                            for c in self.chars.by_ref() {
                                self.pos += 1;
                                if c == '"' {
                                    break;
                                }
                                string.push(c);
                            }
                            Some((
                                Token::Display(Some(Box::new(Token::String(string)))),
                                Span::new(token_start, self.pos),
                            ))
                        } else {
                            self.pos = before_ws;
                            // Reset chars iterator to before whitespace
                            // Actually, we already consumed whitespace. Restore pos for span only.
                            Some((Token::Display(None), Span::new(token_start, before_ws)))
                        }
                    }
                    "DISPLAYINLINE" => {
                        Some((Token::DisplayInline, Span::new(token_start, self.pos)))
                    }
                    "INPUT" => Some((Token::Input, Span::new(token_start, self.pos))),
                    "IF" => Some((Token::If, Span::new(token_start, self.pos))),
                    "ELSE" => Some((Token::Else, Span::new(token_start, self.pos))),
                    "REPEAT" => Some((Token::Repeat, Span::new(token_start, self.pos))),
                    "NOT" => Some((Token::Not, Span::new(token_start, self.pos))),
                    "AND" => Some((Token::And, Span::new(token_start, self.pos))),
                    "OR" => Some((Token::Or, Span::new(token_start, self.pos))),
                    "COMMENT" => Some((Token::Comment, Span::new(token_start, self.pos))),
                    "COMMENTBLOCK" => Some((Token::CommentBlock, Span::new(token_start, self.pos))),
                    "RETURN" => Some((Token::Return, Span::new(token_start, self.pos))),
                    "TRUE" => Some((Token::Boolean(true), Span::new(token_start, self.pos))),
                    "FALSE" => Some((Token::Boolean(false), Span::new(token_start, self.pos))),
                    "CLASS" => Some((Token::Class, Span::new(token_start, self.pos))),
                    "TOSTRING" => Some((Token::ToString, Span::new(token_start, self.pos))),
                    "TONUM" => Some((Token::ToNum, Span::new(token_start, self.pos))),
                    "FOR" => Some((Token::For, Span::new(token_start, self.pos))),
                    "TRIM" => Some((
                        Token::Identifier("TRIM".to_string()),
                        Span::new(token_start, self.pos),
                    )),
                    "REPLACE" => Some((
                        Token::Identifier("REPLACE".to_string()),
                        Span::new(token_start, self.pos),
                    )),
                    "UPPERCASE" => Some((
                        Token::Identifier("UPPERCASE".to_string()),
                        Span::new(token_start, self.pos),
                    )),
                    "LOWERCASE" => Some((
                        Token::Identifier("LOWERCASE".to_string()),
                        Span::new(token_start, self.pos),
                    )),
                    "EACH" => Some((Token::Each, Span::new(token_start, self.pos))),
                    "IN" => Some((Token::In, Span::new(token_start, self.pos))),
                    "PROCEDURE" => Some((Token::Procedure, Span::new(token_start, self.pos))),
                    "SUBSTRING" => Some((Token::Substring, Span::new(token_start, self.pos))),
                    "CONCAT" => Some((Token::Concat, Span::new(token_start, self.pos))),
                    "IMPORT" => Some((Token::Import, Span::new(token_start, self.pos))),
                    "UNTIL" => Some((Token::Until, Span::new(token_start, self.pos))),
                    "TIMES" => Some((Token::Times, Span::new(token_start, self.pos))),
                    "NOT=" => Some((Token::NotEqual, Span::new(token_start, self.pos))),
                    "INSERT" => Some((Token::ListInsert, Span::new(token_start, self.pos))),
                    "APPEND" => Some((Token::ListAppend, Span::new(token_start, self.pos))),
                    "REMOVE" => Some((Token::ListRemove, Span::new(token_start, self.pos))),
                    "LENGTH" => Some((Token::ListLength, Span::new(token_start, self.pos))),
                    "RANDOM" => Some((Token::Random, Span::new(token_start, self.pos))),
                    "SORT" => Some((Token::Sort, Span::new(token_start, self.pos))),
                    "TRY" => Some((Token::Try, Span::new(token_start, self.pos))),
                    "CATCH" => Some((Token::Catch, Span::new(token_start, self.pos))),
                    "EVAL" => Some((Token::Eval, Span::new(token_start, self.pos))),
                    _ => Some((
                        Token::Identifier(identifier),
                        Span::new(token_start, self.pos),
                    )),
                }
            }
            _ => Some((
                Token::Identifier(next_char.to_string()),
                Span::new(token_start, self.pos),
            )),
        }
    }
}
