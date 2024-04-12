#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum Token {
    Unknown,
    Identifier(String),

    // Assignment, Display, Input
    Assignment,
    Display,
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

    // List operations
    ListCreate,
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
    Boolean(bool),

    // Comments
    Comment,
    CommentBlock,

    // Outside of AP
    DisplayInline,
}

pub struct Lexer<'a> {
    chars: std::str::Chars<'a>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer { chars: input.chars() }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token() {
            tokens.push(token);
        }
        tokens
    }

    fn next_token(&mut self) -> Option<Token> {
        let next_char = self.chars.next()?;
    
        if next_char.is_whitespace() {
            return self.next_token();
        }
    
        match next_char {
            '=' => Some(Token::Equal),
            '>' => Some(Token::GreaterThan),
            '<' => Some(Token::LessThan),
            '+' => Some(Token::Plus),
            '-' => Some(Token::Minus),
            '*' => Some(Token::Multiply),
            '/' => Some(Token::Divide),
            '0'..='9' => {
                let mut number = next_char.to_digit(10)? as i32;
                while let Some(next_char) = self.chars.clone().next() {
                    if let Some(digit) = next_char.to_digit(10) {
                        number = number * 10 + digit as i32;
                        self.chars.next();
                    } else {
                        break;
                    }
                }
                Some(Token::Integer(number))
            }
            'a'..='z' | 'A'..='Z' => {
                let mut identifier = String::new();
                identifier.push(next_char);
                while let Some(next_char) = self.chars.clone().next() {
                    if next_char.is_alphanumeric() || next_char == '_' {
                        identifier.push(next_char);
                        self.chars.next();
                    } else {
                        break;
                    }
                }
                match identifier.as_str() {
                    /*
                    TODO:
                    0. <- is classified as < and -, not just <-
                    EX: <- prints [LessThan, Minus] instead of [Assignment]

                    1. sometimes, unknown tokens cause the rest of the program to not be recognized, outputting []
                    also make it so that unknown tokens are printed as themselves with Identifier
                    EX: a < prints [] instead of [Identifier("a"), LessThan]
                    < DISPLAY aaa prints [LessThan, DISPLAY] instead of [LessThan, DISPLAY, Identifier("aaa")]
                    a < prints [] instead of [Identifier("a"), LessThan]

                    2. Also, strings are not recognized, anything in double quotes should be a string

                    3. Parameters DISPLAY("hi") should work by having tokens in between parentheses, so DISPLAY("hi") would be [DISPLAY(String("hi"))]

                    4. Lists
                    */
                    "<-" => Some(Token::Assignment),
                    "DISPLAY" => Some(Token::Display),
                    "INPUT" => Some(Token::Input),
                    "MOD" => Some(Token::Modulo),
                    ">=" => Some(Token::GreaterThanOrEqual),
                    "<=" => Some(Token::LessThanOrEqual),
                    "IF" => Some(Token::If),
                    "ELSE" => Some(Token::Else),
                    "REPEAT" => Some(Token::Repeat),
                    "REPEAT UNTIL" => Some(Token::RepeatUntil),
                    "NOT" => Some(Token::Not),
                    "AND" => Some(Token::And),
                    "OR" => Some(Token::Or),
                    "COMMENT" => Some(Token::Comment),
                    "COMMENTBLOCK" => Some(Token::CommentBlock),
                    "RETURN" => Some(Token::Return),
                    _ => Some(Token::Identifier(identifier)),
                }
            }
            _ => Some(Token::Unknown),
        }
    }
}
