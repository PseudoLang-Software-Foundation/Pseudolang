/*
TODO:
1. sometimes, unknown tokens cause the rest of the program to not be recognized, outputting []
also make it so that unknown tokens are printed as themselves with Identifier
EX: a < prints [] instead of [Identifier("a"), LessThan]
< DISPLAY aaa prints [LessThan, DISPLAY] instead of [LessThan, DISPLAY, Identifier("aaa")]
a < prints [] instead of [Identifier("a"), LessThan]
IMPORTANT: If the token is not recognized, it should only be an identifier if before/after assignment, in a parameter, or in a list, this logic should be in the parser.

2. Also, strings are not recognized, anything in double quotes should be a string

3. Parameters DISPLAY("hi") should work by having tokens in between parentheses, so DISPLAY("hi") would be [DISPLAY(String("hi"))]
Current it prints [Display, Unknown, Unknown, Identifier("hi"), Unknown, Unknown] instead of [Display(String("hi"))]

4. List lexing: Ex: ["1", 1, 0.1, true] should be [ListCreate(String("1"), Integer(1), Float(0.1), Boolean(true))]

5. Comment lexing: Ex: COMMENT asdasd should be [Comment], and anything after on the same line should be ignored.
For comment blocks, remove everything in between the commentblock types.

6. Raw strings r"asd", and multiline strings with """asd""" along with formatted strings f"asd {var}".
*/

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
            '"' => {
                let mut string = String::new();
                while let Some(next_char) = self.chars.clone().next() {
                    if next_char != '"' {
                        string.push(next_char);
                        self.chars.next();
                    } else {
                        self.chars.next();
                        break;
                    }
                }
                Some(Token::String(string))
            },
            '(' => {
                let mut params = Vec::new();
                while let Some(next_char) = self.chars.clone().next() {
                    if next_char != ')' {
                        if let Some(token) = self.next_token() {
                            params.push(token);
                        }
                    } else {
                        self.chars.next();
                        break;
                    }
                }
                Some(Token::Parameters(params))
            },
            '=' => Some(Token::Equal),
            '>' => Some(Token::GreaterThan),
            '<' => {
                if let Some('-') = self.chars.clone().next() {
                    self.chars.next();
                    Some(Token::Assignment)
                } else {
                    Some(Token::LessThan)
                }
            },    
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
                    "RETURN" => Some(Token::Return)
                }
            }
        }
    }
}
