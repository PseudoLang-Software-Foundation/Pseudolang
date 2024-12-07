use crate::lexer::{Lexer, Token};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AstNode {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    List(Vec<AstNode>),
    Null,
    NaN,

    Identifier(String),
    Assignment(Box<AstNode>, Box<AstNode>),
    ListAccess(Box<AstNode>, Box<AstNode>),
    ListAssignment(Box<AstNode>, Box<AstNode>, Box<AstNode>),

    ListInsert(Box<AstNode>, Box<AstNode>, Box<AstNode>),
    ListAppend(Box<AstNode>, Box<AstNode>),
    ListRemove(Box<AstNode>, Box<AstNode>),

    BinaryOp(Box<AstNode>, BinaryOperator, Box<AstNode>),
    UnaryOp(UnaryOperator, Box<AstNode>),

    If(Box<AstNode>, Box<AstNode>, Option<Box<AstNode>>),
    RepeatTimes(Box<AstNode>, Box<AstNode>),
    RepeatUntil(Box<AstNode>, Box<AstNode>),
    ForEach(String, Box<AstNode>, Box<AstNode>),

    ProcedureDecl(String, Vec<String>, Box<AstNode>),
    ProcedureCall(String, Vec<AstNode>),
    Return(Box<AstNode>),

    Display(Option<Box<AstNode>>),
    DisplayInline(Box<AstNode>),
    Input(Option<Box<AstNode>>),
    Random(Box<AstNode>, Box<AstNode>),
    Insert(Box<AstNode>, Box<AstNode>, Box<AstNode>),
    Append(Box<AstNode>, Box<AstNode>),
    Remove(Box<AstNode>, Box<AstNode>),
    Length(Box<AstNode>),
    Substring(Box<AstNode>, Box<AstNode>, Box<AstNode>),
    Concat(Box<AstNode>, Box<AstNode>),
    ToString(Box<AstNode>),
    ToNum(Box<AstNode>),
    Sort(Box<AstNode>),

    ClassDecl(String, Box<AstNode>),

    Block(Vec<AstNode>),
    Program(Vec<AstNode>),
    Comment(String),
    Import(String),

    RawString(String),
    FormattedString(String, Vec<AstNode>),
    TryCatch {
        try_block: Box<AstNode>,
        error_var: Option<String>,
        catch_block: Box<AstNode>,
    },
    Eval(Box<AstNode>),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Gt,
    Lt,
    GtEq,
    LtEq,
    And,
    Or,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum UnaryOperator {
    Not,
    Neg,
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    fn debug_print(debug: bool, message: &str) {
        if debug {
            eprintln!("\x1b[33m[PARSER DEBUG]\x1b[0m {}", message);
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.current < self.tokens.len() {
            let token = self.tokens[self.current].clone();
            self.current += 1;
            Some(token)
        } else {
            None
        }
    }

    fn match_token(&mut self, expected: &Token) -> bool {
        if let Some(token) = self.peek() {
            if token == expected {
                self.advance();
                return true;
            }
        }
        false
    }

    fn parse_program(&mut self, debug: bool) -> Result<AstNode, String> {
        Self::debug_print(debug, "Starting program parse");
        let mut statements = Vec::new();

        while self.peek().is_some() {
            Self::debug_print(debug, &format!("Current token: {:?}", self.peek()));
            statements.push(self.parse_statement(debug)?);
        }

        Self::debug_print(
            debug,
            &format!(
                "Finished program parse with {} statements",
                statements.len()
            ),
        );
        Ok(AstNode::Program(statements))
    }

    fn parse_statement(&mut self, debug: bool) -> Result<AstNode, String> {
        Self::debug_print(
            debug,
            &format!("Parsing statement at position {}", self.current),
        );

        match self.peek() {
            Some(Token::Try) => {
                self.advance();
                let try_block = self.parse_block(debug)?;

                while matches!(self.peek(), Some(Token::Newline)) {
                    self.advance();
                }

                if !self.match_token(&Token::Catch) {
                    return Err("Expected 'catch' after try block".to_string());
                }

                let mut error_var = None;
                if self.match_token(&Token::OpenParen) {
                    if let Some(Token::Identifier(name)) = self.advance() {
                        error_var = Some(name);
                    } else {
                        return Err("Expected identifier after 'catch('".to_string());
                    }

                    if !self.match_token(&Token::CloseParen) {
                        return Err("Expected ')' after catch variable".to_string());
                    }
                }

                let catch_block = self.parse_block(debug)?;

                Ok(AstNode::TryCatch {
                    try_block: Box::new(try_block),
                    error_var,
                    catch_block: Box::new(catch_block),
                })
            }
            Some(Token::ListAppend) => self.parse_list_append(debug),
            Some(Token::ListRemove) => self.parse_list_remove(debug),
            Some(Token::ListInsert) => self.parse_list_insert(debug),
            Some(Token::Random) => self.parse_random(debug),
            Some(Token::Substring) => self.parse_substring(debug),
            Some(Token::Concat) => self.parse_concat(debug),
            Some(Token::ToString) => self.parse_to_string(debug),
            Some(Token::ToNum) => self.parse_to_num(debug),
            Some(Token::ListLength) => self.parse_list_length(debug),
            Some(Token::Sort) => {
                self.advance();
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after SORT".to_string());
                }
                let list_expr = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after list expression".to_string());
                }
                Ok(AstNode::Sort(Box::new(list_expr)))
            }
            Some(Token::Identifier(_)) => {
                let identifier = match self.advance() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected identifier".to_string()),
                };

                let mut list_accesses = Vec::new();
                while let Some(Token::OpenBracket) = self.peek() {
                    self.advance();
                    let index = self.parse_expression(debug)?;
                    if !self.match_token(&Token::CloseBracket) {
                        return Err("Expected ']'".to_string());
                    }
                    list_accesses.push(index);
                }

                match self.peek() {
                    Some(Token::Assign) => {
                        self.advance();
                        let value = self.parse_expression(debug)?;

                        if list_accesses.is_empty() {
                            Ok(AstNode::Assignment(
                                Box::new(AstNode::Identifier(identifier)),
                                Box::new(value),
                            ))
                        } else {
                            let mut current = AstNode::Identifier(identifier);
                            for (i, index) in list_accesses.iter().enumerate() {
                                if i == list_accesses.len() - 1 {
                                    current = AstNode::ListAssignment(
                                        Box::new(current),
                                        Box::new(index.clone()),
                                        Box::new(value.clone()),
                                    );
                                } else {
                                    current = AstNode::ListAccess(
                                        Box::new(current),
                                        Box::new(index.clone()),
                                    );
                                }
                            }
                            Ok(current)
                        }
                    }
                    Some(Token::OpenParen) => {
                        self.advance();
                        let mut args = Vec::new();
                        while !self.match_token(&Token::CloseParen) {
                            if !args.is_empty() {
                                if !self.match_token(&Token::Comma) {
                                    return Err("Expected comma between arguments".to_string());
                                }
                            }
                            args.push(self.parse_expression(debug)?);
                        }
                        Ok(AstNode::ProcedureCall(identifier, args))
                    }
                    _ => {
                        if list_accesses.is_empty() {
                            Ok(AstNode::Identifier(identifier))
                        } else {
                            let mut current = AstNode::Identifier(identifier);
                            for index in list_accesses {
                                current = AstNode::ListAccess(Box::new(current), Box::new(index));
                            }
                            Ok(current)
                        }
                    }
                }
            }
            None => {
                Self::debug_print(debug, "End of input reached");
                Ok(AstNode::Block(Vec::new()))
            }
            Some(Token::Newline) => {
                Self::debug_print(debug, "Found newline, skipping");
                self.advance();
                self.parse_statement(debug)
            }
            Some(Token::CloseBrace) => Ok(AstNode::Block(Vec::new())),
            Some(_) if self.is_expression_start() => {
                Self::debug_print(debug, "Starting expression parse");
                self.parse_expression(debug)
            }
            Some(Token::If) => {
                Self::debug_print(debug, "Starting if statement parse");
                self.parse_if(debug)
            }
            Some(Token::Procedure) => {
                Self::debug_print(debug, "Starting procedure parse");
                self.parse_procedure(debug)
            }
            Some(Token::Repeat) => {
                Self::debug_print(debug, "Starting repeat parse");
                self.parse_repeat(debug)
            }
            Some(Token::For) => self.parse_foreach(debug),
            Some(Token::Class) => self.parse_class(debug),
            Some(Token::Display(_)) => {
                self.advance();
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after DISPLAY".to_string());
                }
                let expr = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after expression".to_string());
                }
                Ok(AstNode::Display(Some(Box::new(expr))))
            }
            Some(Token::DisplayInline) => self.parse_display_inline(debug),
            Some(Token::Comment) => self.parse_comment(debug),
            Some(Token::Import) => self.parse_import(debug),
            Some(Token::Return) => {
                self.advance();
                if matches!(self.peek(), Some(Token::OpenParen)) {
                    self.advance();
                    if matches!(self.peek(), Some(Token::CloseParen)) {
                        self.advance();
                        Ok(AstNode::Return(Box::new(AstNode::Block(vec![]))))
                    } else {
                        let expr = self.parse_expression(debug)?;
                        if !self.match_token(&Token::CloseParen) {
                            return Err("Expected ')' after return expression".to_string());
                        }
                        Ok(AstNode::Return(Box::new(expr)))
                    }
                } else {
                    Ok(AstNode::Return(Box::new(AstNode::Block(vec![]))))
                }
            }
            Some(Token::Input) => {
                self.advance();
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after INPUT".to_string());
                }
                let prompt = if self.peek() != Some(&Token::CloseParen) {
                    Some(Box::new(self.parse_expression(debug)?))
                } else {
                    None
                };
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after INPUT".to_string());
                }
                Ok(AstNode::Input(prompt))
            }
            _ => {
                Self::debug_print(
                    debug,
                    &format!("Unexpected token in statement: {:?}", self.peek()),
                );
                Err("Unexpected token in statement".to_string())
            }
        }
    }

    fn is_expression_start(&self) -> bool {
        match self.peek() {
            Some(Token::Integer(_))
            | Some(Token::Float(_))
            | Some(Token::String(_))
            | Some(Token::Boolean(_))
            | Some(Token::Identifier(_))
            | Some(Token::OpenParen)
            | Some(Token::OpenBracket)
            | Some(Token::Not)
            | Some(Token::Minus)
            | Some(Token::Plus)
            | Some(Token::ToString)
            | Some(Token::ToNum)
            | Some(Token::ListLength)
            | Some(Token::GreaterThan)
            | Some(Token::GreaterThanOrEqual)
            | Some(Token::LessThan)
            | Some(Token::LessThanOrEqual)
            | Some(Token::Equal)
            | Some(Token::NotEqual)
            | Some(Token::Random)
            | Some(Token::ListRemove)
            | Some(Token::ListAppend)
            | Some(Token::ListInsert)
            | Some(Token::Sort)
            | Some(Token::Input) => true,
            _ => false,
        }
    }

    pub fn parse_expression(&mut self, debug: bool) -> Result<AstNode, String> {
        if self.match_token(&Token::Sort) {
            if !self.match_token(&Token::OpenParen) {
                return Err("Expected '(' after SORT".to_string());
            }
            let list_expr = self.parse_expression(debug)?;
            if !self.match_token(&Token::CloseParen) {
                return Err("Expected ')' after list expression".to_string());
            }
            Ok(AstNode::Sort(Box::new(list_expr)))
        } else {
            match self.peek() {
                Some(Token::Concat) | Some(Token::Substring) => self.parse_builtin_function(debug),
                _ => self.parse_logical_or(debug),
            }
        }
    }

    fn parse_builtin_function(&mut self, debug: bool) -> Result<AstNode, String> {
        let function_token = self.peek().cloned();
        match function_token {
            Some(Token::Concat) => {
                self.advance();
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after CONCAT".to_string());
                }
                let arg1 = self.parse_expression(debug)?;
                if !self.match_token(&Token::Comma) {
                    return Err("Expected comma after first argument".to_string());
                }
                let arg2 = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after second argument".to_string());
                }
                Ok(AstNode::Concat(Box::new(arg1), Box::new(arg2)))
            }
            Some(Token::Substring) => {
                self.advance();
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after SUBSTRING".to_string());
                }
                let string_expr = self.parse_expression(debug)?;
                if !self.match_token(&Token::Comma) {
                    return Err("Expected comma after string expression".to_string());
                }
                let start_expr = self.parse_expression(debug)?;
                if !self.match_token(&Token::Comma) {
                    return Err("Expected comma after start expression".to_string());
                }
                let end_expr = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after end expression".to_string());
                }
                Ok(AstNode::Substring(
                    Box::new(string_expr),
                    Box::new(start_expr),
                    Box::new(end_expr),
                ))
            }
            Some(Token::ListLength) => {
                self.advance();
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after LENGTH".to_string());
                }
                let arg = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after argument".to_string());
                }
                Ok(AstNode::Length(Box::new(arg)))
            }
            _ => Err("Unknown built-in function".to_string()),
        }
    }

    fn parse_logical_or(&mut self, debug: bool) -> Result<AstNode, String> {
        let mut expr = self.parse_logical_and(debug)?;

        while self.match_token(&Token::Or) {
            let right = self.parse_logical_and(debug)?;
            expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Or, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_logical_and(&mut self, debug: bool) -> Result<AstNode, String> {
        let mut expr = self.parse_equality(debug)?;

        while self.match_token(&Token::And) {
            let right = self.parse_equality(debug)?;
            expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::And, Box::new(right));
        }
        Ok(expr)
    }

    fn parse_equality(&mut self, debug: bool) -> Result<AstNode, String> {
        let mut expr = self.parse_comparison(debug)?;

        while let Some(token) = self.peek() {
            match token {
                Token::Equal | Token::NotEqual => {
                    let op = if self.match_token(&Token::Equal) {
                        BinaryOperator::Eq
                    } else {
                        self.advance();
                        BinaryOperator::NotEq
                    };
                    let right = self.parse_comparison(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), op, Box::new(right));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self, debug: bool) -> Result<AstNode, String> {
        let mut expr = self.parse_term(debug)?;

        while let Some(token) = self.peek() {
            match token {
                Token::GreaterThan
                | Token::GreaterThanOrEqual
                | Token::LessThan
                | Token::LessThanOrEqual => {
                    let op = match token {
                        Token::GreaterThan => BinaryOperator::Gt,
                        Token::GreaterThanOrEqual => BinaryOperator::GtEq,
                        Token::LessThan => BinaryOperator::Lt,
                        Token::LessThanOrEqual => BinaryOperator::LtEq,
                        _ => unreachable!(),
                    };
                    self.advance();
                    let right = self.parse_term(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), op, Box::new(right));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_term(&mut self, debug: bool) -> Result<AstNode, String> {
        let mut expr = self.parse_factor(debug)?;

        while let Some(token) = self.peek() {
            match token {
                Token::Plus => {
                    self.advance();
                    let right = self.parse_factor(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Add, Box::new(right));
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_factor(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Sub, Box::new(right));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_factor(&mut self, debug: bool) -> Result<AstNode, String> {
        let mut expr = self.parse_unary(debug)?;

        while let Some(token) = self.peek() {
            match token {
                Token::Multiply => {
                    self.advance();
                    let right = self.parse_unary(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Mul, Box::new(right));
                }
                Token::Divide => {
                    self.advance();
                    let right = self.parse_unary(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Div, Box::new(right));
                }
                Token::Modulo => {
                    self.advance();
                    let right = self.parse_unary(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Mod, Box::new(right));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_unary(&mut self, debug: bool) -> Result<AstNode, String> {
        if let Some(token) = self.peek() {
            match token {
                Token::Not => {
                    self.advance();
                    let expr = self.parse_unary(debug)?;
                    Ok(AstNode::UnaryOp(UnaryOperator::Not, Box::new(expr)))
                }
                Token::Minus => {
                    self.advance();
                    let expr = self.parse_unary(debug)?;
                    Ok(AstNode::UnaryOp(UnaryOperator::Neg, Box::new(expr)))
                }
                _ => self.parse_primary(debug),
            }
        } else {
            Err("Unexpected end of input".to_string())
        }
    }

    fn parse_primary(&mut self, debug: bool) -> Result<AstNode, String> {
        match self.peek() {
            Some(Token::ListAppend) => self.parse_list_append(debug),
            Some(Token::ListRemove) => self.parse_list_remove(debug),
            Some(Token::ListInsert) => self.parse_list_insert(debug),
            Some(Token::ListLength) => self.parse_list_length(debug),
            Some(Token::Random) => self.parse_random(debug),
            Some(Token::Substring) => self.parse_substring(debug),
            Some(Token::Concat) => self.parse_concat(debug),
            Some(Token::ToString) => self.parse_to_string(debug),
            Some(Token::ToNum) => self.parse_to_num(debug),
            Some(Token::Sort) => {
                self.advance();
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after SORT".to_string());
                }
                let list_expr = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after list".to_string());
                }
                Ok(AstNode::Sort(Box::new(list_expr)))
            }
            Some(Token::Identifier(_)) => {
                let name = match self.advance() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected identifier".to_string()),
                };

                let mut node = AstNode::Identifier(name.clone());

                while let Some(Token::OpenBracket) = self.peek() {
                    self.advance();
                    let index = self.parse_expression(debug)?;
                    if !self.match_token(&Token::CloseBracket) {
                        return Err("Expected ']' after list index".to_string());
                    }
                    node = AstNode::ListAccess(Box::new(node), Box::new(index));
                }

                if self.match_token(&Token::OpenParen) {
                    let mut args = Vec::new();
                    while !self.match_token(&Token::CloseParen) {
                        if !args.is_empty() {
                            if !self.match_token(&Token::Comma) {
                                return Err("Expected comma between arguments".to_string());
                            }
                        }
                        args.push(self.parse_expression(debug)?);
                    }
                    return Ok(AstNode::ProcedureCall(name, args));
                }

                Ok(node)
            }
            Some(Token::FormattedString(template, vars)) => {
                let template = template.clone();
                let vars = vars.clone();
                self.advance();
                let mut expressions = Vec::new();
                for var in vars {
                    let mut var_lexer = Lexer::new(&var);
                    let var_tokens = var_lexer.tokenize();
                    let mut var_parser = Parser::new(var_tokens);
                    let expr = var_parser.parse_expression(debug)?;
                    expressions.push(expr);
                }
                Ok(AstNode::FormattedString(template, expressions))
            }
            Some(Token::Input) => {
                self.advance();
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after INPUT".to_string());
                }
                let prompt = if self.peek() != Some(&Token::CloseParen) {
                    Some(Box::new(self.parse_expression(debug)?))
                } else {
                    None
                };
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after INPUT".to_string());
                }
                Ok(AstNode::Input(prompt))
            }
            Some(Token::Eval) => {
                self.advance();
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after EVAL".to_string());
                }
                let expr = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after EVAL expression".to_string());
                }
                Ok(AstNode::Eval(Box::new(expr)))
            }
            _ => match self.advance() {
                Some(Token::Integer(n)) => Ok(AstNode::Integer(n)),
                Some(Token::Float(f)) => Ok(AstNode::Float(f)),
                Some(Token::String(s)) => Ok(AstNode::String(s)),
                Some(Token::RawString(s)) => Ok(AstNode::RawString(s)),
                Some(Token::Boolean(b)) => Ok(AstNode::Boolean(b)),
                Some(Token::Null) => Ok(AstNode::Null),
                Some(Token::NaN) => Ok(AstNode::NaN),
                Some(Token::Identifier(name)) => Ok(AstNode::Identifier(name)),
                Some(Token::OpenParen) => {
                    let expr = self.parse_expression(debug)?;
                    if !self.match_token(&Token::CloseParen) {
                        return Err("Expected ')' after expression".to_string());
                    }
                    Ok(expr)
                }
                Some(Token::OpenBracket) => self.parse_list(debug),
                _ => Err("Unexpected token in expression".to_string()),
            },
        }
    }

    fn parse_class(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        let name = match self.advance() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected class name".to_string()),
        };
        let body = self.parse_block(debug)?;
        Ok(AstNode::ClassDecl(name, Box::new(body)))
    }

    fn parse_foreach(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        if !self.match_token(&Token::Each) {
            return Err("Expected EACH after FOR".to_string());
        }
        let var_name = match self.advance() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected identifier after EACH".to_string()),
        };
        if !self.match_token(&Token::In) {
            return Err("Expected IN after identifier".to_string());
        }
        let list = self.parse_expression(debug)?;
        let body = self.parse_block(debug)?;
        Ok(AstNode::ForEach(var_name, Box::new(list), Box::new(body)))
    }

    fn parse_block(&mut self, debug: bool) -> Result<AstNode, String> {
        Self::debug_print(
            debug,
            &format!("Parsing block, current token: {:?}", self.peek()),
        );

        while let Some(Token::Newline) = self.peek() {
            Self::debug_print(debug, "Skipping newline before block");
            self.advance();
        }

        match self.peek() {
            Some(Token::OpenBrace) => {
                Self::debug_print(debug, "Found opening brace");
                self.advance();

                while let Some(Token::Newline) = self.peek() {
                    Self::debug_print(debug, "Skipping newline after opening brace");
                    self.advance();
                }

                let mut statements = Vec::new();
                while let Some(token) = self.peek() {
                    if token == &Token::CloseBrace {
                        break;
                    }

                    let stmt = self.parse_statement(debug)?;
                    match stmt {
                        AstNode::Block(v) if v.is_empty() => {}
                        _ => statements.push(stmt),
                    }

                    while let Some(Token::Newline) = self.peek() {
                        Self::debug_print(debug, "Skipping newline between statements");
                        self.advance();
                    }
                }

                if !self.match_token(&Token::CloseBrace) {
                    return Err("Expected '}' at end of block".to_string());
                }

                Self::debug_print(debug, "Block parsing complete");
                Ok(AstNode::Block(statements))
            }
            _ => Err("Expected '{' to start block".to_string()),
        }
    }

    fn parse_procedure(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        let name = match self.advance() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected procedure name".to_string()),
        };
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after procedure name".to_string());
        }
        let mut params = Vec::new();
        while let Some(token) = self.peek() {
            if token == &Token::CloseParen {
                break;
            }
            if !params.is_empty() {
                if !self.match_token(&Token::Comma) {
                    return Err("Expected comma between parameters".to_string());
                }
            }
            match self.advance() {
                Some(Token::Identifier(param)) => params.push(param),
                _ => return Err("Expected parameter name".to_string()),
            }
        }
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')' after parameters".to_string());
        }
        let body = self.parse_block(debug)?;
        Ok(AstNode::ProcedureDecl(name, params, Box::new(body)))
    }

    fn parse_display_inline(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after DISPLAYINLINE".to_string());
        }
        let expr = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')' after expression".to_string());
        }
        Ok(AstNode::DisplayInline(Box::new(expr)))
    }

    fn parse_comment(&mut self, _debug: bool) -> Result<AstNode, String> {
        self.advance();
        match self.advance() {
            Some(Token::String(text)) => Ok(AstNode::Comment(text)),
            _ => Err("Expected string after COMMENT".to_string()),
        }
    }

    fn parse_import(&mut self, _debug: bool) -> Result<AstNode, String> {
        self.advance();
        match self.advance() {
            Some(Token::String(path)) => Ok(AstNode::Import(path)),
            _ => Err("Expected string after IMPORT".to_string()),
        }
    }

    #[allow(dead_code)]
    fn synchronize(&mut self) {
        while let Some(token) = self.peek() {
            match token {
                Token::Newline => {
                    self.advance();
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn parse_list(&mut self, debug: bool) -> Result<AstNode, String> {
        let mut elements = Vec::new();
        loop {
            while let Some(Token::Newline) = self.peek() {
                self.advance();
            }

            if let Some(Token::CloseBracket) = self.peek() {
                self.advance();
                break;
            }

            if !elements.is_empty() {
                if !self.match_token(&Token::Comma) {
                    return Err("Expected comma between list elements".to_string());
                }
                while let Some(Token::Newline) = self.peek() {
                    self.advance();
                }
            }

            elements.push(self.parse_expression(debug)?);

            while let Some(Token::Newline) = self.peek() {
                self.advance();
            }
        }
        Ok(AstNode::List(elements))
    }

    fn parse_list_length(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after LENGTH".to_string());
        }
        let list = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')'".to_string());
        }
        Ok(AstNode::Length(Box::new(list)))
    }

    fn parse_list_remove(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after REMOVE".to_string());
        }
        let list = self.parse_expression(debug)?;
        if !self.match_token(&Token::Comma) {
            return Err("Expected comma after list".to_string());
        }
        let index = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')' after index".to_string());
        }
        Ok(AstNode::Remove(Box::new(list), Box::new(index)))
    }

    fn parse_list_append(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after APPEND".to_string());
        }
        let list = self.parse_expression(debug)?;
        if !self.match_token(&Token::Comma) {
            return Err("Expected comma after list".to_string());
        }
        let value = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')'".to_string());
        }
        Ok(AstNode::Append(Box::new(list), Box::new(value)))
    }

    fn parse_if(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        let condition = if self.match_token(&Token::OpenParen) {
            let expr = self.parse_expression(debug)?;
            if !self.match_token(&Token::CloseParen) {
                return Err("Expected ')' after condition".to_string());
            }
            expr
        } else {
            self.parse_expression(debug)?
        };

        let then_branch = self.parse_block(debug)?;

        while let Some(Token::Newline) = self.peek() {
            self.advance();
        }

        let else_branch = if self.peek() == Some(&Token::Else) {
            self.advance();

            while let Some(Token::Newline) = self.peek() {
                self.advance();
            }

            if self.peek() == Some(&Token::If) {
                Some(Box::new(self.parse_if(debug)?))
            } else {
                Some(Box::new(self.parse_block(debug)?))
            }
        } else {
            None
        };

        Ok(AstNode::If(
            Box::new(condition),
            Box::new(then_branch),
            else_branch,
        ))
    }

    fn parse_repeat(&mut self, debug: bool) -> Result<AstNode, String> {
        Self::debug_print(debug, "Starting repeat parse");
        self.advance();

        if self.peek() == Some(&Token::Until) {
            self.advance();
            let condition = if self.match_token(&Token::OpenParen) {
                let expr = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after condition".to_string());
                }
                expr
            } else {
                self.parse_expression(debug)?
            };

            while let Some(Token::Newline) = self.peek() {
                self.advance();
            }

            let body = self.parse_block(debug)?;
            Ok(AstNode::RepeatUntil(Box::new(body), Box::new(condition)))
        } else {
            let times = self.parse_expression(debug)?;
            if !self.match_token(&Token::Times) {
                return Err("Expected TIMES after repeat count".to_string());
            }
            let body = self.parse_block(debug)?;
            Ok(AstNode::RepeatTimes(Box::new(times), Box::new(body)))
        }
    }

    fn parse_list_insert(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after INSERT".to_string());
        }
        let list = self.parse_expression(debug)?;
        if !self.match_token(&Token::Comma) {
            return Err("Expected comma after list".to_string());
        }
        let index = self.parse_expression(debug)?;
        if !self.match_token(&Token::Comma) {
            return Err("Expected comma after index".to_string());
        }
        let value = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')'".to_string());
        }
        Ok(AstNode::Insert(
            Box::new(list),
            Box::new(index),
            Box::new(value),
        ))
    }

    fn parse_random(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after RANDOM".to_string());
        }
        let min = self.parse_expression(debug)?;
        if !self.match_token(&Token::Comma) {
            return Err("Expected comma after min value".to_string());
        }
        let max = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')'".to_string());
        }
        Ok(AstNode::Random(Box::new(min), Box::new(max)))
    }

    fn parse_substring(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after SUBSTRING".to_string());
        }
        let string = self.parse_expression(debug)?;
        if !self.match_token(&Token::Comma) {
            return Err("Expected comma after string".to_string());
        }
        let start = self.parse_expression(debug)?;
        if !self.match_token(&Token::Comma) {
            return Err("Expected comma after start index".to_string());
        }
        let end = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')'".to_string());
        }
        Ok(AstNode::Substring(
            Box::new(string),
            Box::new(start),
            Box::new(end),
        ))
    }

    fn parse_concat(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after CONCAT".to_string());
        }
        let str1 = self.parse_expression(debug)?;
        if !self.match_token(&Token::Comma) {
            return Err("Expected comma after first string".to_string());
        }
        let str2 = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')'".to_string());
        }
        Ok(AstNode::Concat(Box::new(str1), Box::new(str2)))
    }

    fn parse_to_string(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after TOSTRING".to_string());
        }
        let expr = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')'".to_string());
        }
        Ok(AstNode::ToString(Box::new(expr)))
    }

    fn parse_to_num(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance();
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after TONUM".to_string());
        }
        let expr = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')'".to_string());
        }
        Ok(AstNode::ToNum(Box::new(expr)))
    }
}

pub fn parse(tokens: Vec<Token>, debug: bool) -> Result<AstNode, String> {
    let mut parser = Parser::new(tokens);
    parser.parse_program(debug)
}
