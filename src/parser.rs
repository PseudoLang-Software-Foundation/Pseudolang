use crate::lexer::Token;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AstNode {
    // Literals
    Integer(i32),
    Float(f32),
    String(String),
    Boolean(bool),
    List(Vec<AstNode>),

    // Variables and assignments
    Identifier(String),
    Assignment(Box<AstNode>, Box<AstNode>),
    ListAccess(Box<AstNode>, Box<AstNode>),
    ListAssignment(Box<AstNode>, Box<AstNode>, Box<AstNode>),

    // Operations
    BinaryOp(Box<AstNode>, BinaryOperator, Box<AstNode>),
    UnaryOp(UnaryOperator, Box<AstNode>),

    // Control flow
    If(Box<AstNode>, Box<AstNode>, Option<Box<AstNode>>),
    RepeatTimes(Box<AstNode>, Box<AstNode>),
    RepeatUntil(Box<AstNode>, Box<AstNode>),
    ForEach(String, Box<AstNode>, Box<AstNode>),

    // Functions and procedures
    ProcedureDecl(String, Vec<String>, Box<AstNode>),
    ProcedureCall(String, Vec<AstNode>),
    Return(Box<AstNode>),

    // Built-in functions
    Display(Option<Box<AstNode>>),
    DisplayInline(Box<AstNode>),
    Input,
    Random(Box<AstNode>, Box<AstNode>),
    Insert(Box<AstNode>, Box<AstNode>, Box<AstNode>),
    Append(Box<AstNode>, Box<AstNode>),
    Remove(Box<AstNode>, Box<AstNode>),
    Length(Box<AstNode>),
    Substring(Box<AstNode>, Box<AstNode>, Box<AstNode>),
    Concat(Box<AstNode>, Box<AstNode>),
    ToString(Box<AstNode>),
    ToNum(Box<AstNode>),

    // Classes
    ClassDecl(String, Box<AstNode>),

    // Program structure
    Block(Vec<AstNode>),
    Program(Vec<AstNode>),
    Comment(String),
    Import(String),

    RawString(String),
    FormattedString(String, Vec<String>),
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
#[allow(dead_code)] // Added this for UnaryOperator too
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
            eprintln!("\x1b[33m[PARSER DEBUG]\x1b[0m {}", message); // Changed format and color
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
        ); // Added count
        Ok(AstNode::Program(statements))
    }

    fn parse_statement(&mut self, debug: bool) -> Result<AstNode, String> {
        Self::debug_print(
            debug,
            &format!("Parsing statement at position {}", self.current),
        );

        match self.peek() {
            None => {
                Self::debug_print(debug, "End of input reached");
                Ok(AstNode::Block(Vec::new()))
            }
            Some(Token::Newline) => {
                Self::debug_print(debug, "Found newline, skipping");
                self.advance();
                self.parse_statement(debug)
            }
            Some(Token::CloseBrace) => {
                // We've reached the end of a block, don't consume the brace
                Ok(AstNode::Block(Vec::new()))
            }
            Some(Token::Identifier(_)) => {
                // First parse the identifier
                let identifier = match self.advance() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected identifier".to_string()),
                };

                // Check what follows the identifier
                match self.peek() {
                    Some(Token::Assign) => {
                        self.advance(); // consume the Assign token
                        let value = self.parse_expression(debug)?;
                        Ok(AstNode::Assignment(
                            Box::new(AstNode::Identifier(identifier)),
                            Box::new(value),
                        ))
                    }
                    Some(Token::OpenBracket) => {
                        // Handle list access/assignment
                        self.advance(); // consume '['
                        let index = self.parse_expression(debug)?;
                        if !self.match_token(&Token::CloseBracket) {
                            return Err("Expected ']'".to_string());
                        }

                        if self.match_token(&Token::Assign) {
                            // List assignment
                            let value = self.parse_expression(debug)?;
                            Ok(AstNode::ListAssignment(
                                Box::new(AstNode::Identifier(identifier)),
                                Box::new(index),
                                Box::new(value),
                            ))
                        } else {
                            // List access
                            Ok(AstNode::ListAccess(
                                Box::new(AstNode::Identifier(identifier)),
                                Box::new(index),
                            ))
                        }
                    }
                    Some(Token::OpenParen) => {
                        // Handle procedure call
                        self.advance(); // consume '('
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
                    _ => Ok(AstNode::Identifier(identifier)),
                }
            }
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
            // ...rest of existing match cases...
            Some(Token::ListInsert) => self.parse_list_insert(debug),
            Some(Token::ListAppend) => self.parse_list_append(debug),
            Some(Token::ListRemove) => self.parse_list_remove(debug),
            Some(Token::ListLength) => self.parse_list_length(debug),
            Some(Token::Random) => self.parse_random(debug),
            Some(Token::Substring) => self.parse_substring(debug),
            Some(Token::Concat) => self.parse_concat(debug),
            Some(Token::ToString) => self.parse_to_string(debug),
            Some(Token::ToNum) => self.parse_to_num(debug),
            Some(Token::If) => self.parse_if(debug),
            Some(Token::Repeat) => {
                Self::debug_print(debug, "Starting repeat parse");
                self.parse_repeat(debug)
            }
            Some(Token::For) => self.parse_foreach(debug),
            Some(Token::Class) => self.parse_class(debug),
            Some(Token::Procedure) => self.parse_procedure(debug),
            Some(Token::Display(_)) => self.parse_display(debug),
            Some(Token::DisplayInline) => self.parse_display_inline(debug),
            Some(Token::Comment) => self.parse_comment(debug),
            Some(Token::Import) => self.parse_import(debug),
            Some(Token::Return) => {
                self.advance(); // consume RETURN
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after RETURN".to_string());
                }
                let expr = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after return expression".to_string());
                }
                Ok(AstNode::Return(Box::new(expr)))
            }
            Some(Token::Input) => {
                self.advance(); // consume INPUT
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after INPUT".to_string());
                }
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after INPUT(".to_string());
                }
                Ok(AstNode::Input)
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

    // Add this helper method to check if a token can start an expression
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
            | Some(Token::ListLength)
            | Some(Token::GreaterThan)
            | Some(Token::GreaterThanOrEqual)
            | Some(Token::LessThan)
            | Some(Token::LessThanOrEqual)
            | Some(Token::Equal)
            | Some(Token::NotEqual) => true,
            _ => false,
        }
    }

    fn parse_expression(&mut self, debug: bool) -> Result<AstNode, String> {
        self.parse_logical(debug)
    }

    fn parse_logical(&mut self, debug: bool) -> Result<AstNode, String> {
        let mut expr = self.parse_equality(debug)?;

        while let Some(token) = self.peek() {
            match token {
                Token::And => {
                    self.advance();
                    let right = self.parse_equality(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::And, Box::new(right));
                }
                Token::Or => {
                    self.advance();
                    let right = self.parse_equality(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Or, Box::new(right));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_equality(&mut self, debug: bool) -> Result<AstNode, String> {
        let mut expr = self.parse_comparison(debug)?;

        while let Some(token) = self.peek() {
            match token {
                Token::Equal => {
                    self.advance();
                    let right = self.parse_comparison(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Eq, Box::new(right));
                }
                Token::NotEqual => {
                    self.advance();
                    let right = self.parse_comparison(debug)?;
                    expr =
                        AstNode::BinaryOp(Box::new(expr), BinaryOperator::NotEq, Box::new(right));
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
                Token::GreaterThan => {
                    self.advance();
                    let right = self.parse_term(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Gt, Box::new(right));
                }
                Token::GreaterThanOrEqual => {
                    self.advance();
                    let right = self.parse_term(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::GtEq, Box::new(right));
                }
                Token::LessThan => {
                    self.advance();
                    let right = self.parse_term(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Lt, Box::new(right));
                }
                Token::LessThanOrEqual => {
                    self.advance();
                    let right = self.parse_term(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::LtEq, Box::new(right));
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
                Token::Multiply => {
                    self.advance();
                    let right = self.parse_factor(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Mul, Box::new(right));
                }
                Token::Divide => {
                    self.advance();
                    let right = self.parse_factor(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Div, Box::new(right));
                }
                Token::Modulo => {
                    self.advance();
                    let right = self.parse_factor(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Mod, Box::new(right));
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
                Token::Plus => {
                    self.advance();
                    let right = self.parse_unary(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Add, Box::new(right));
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_unary(debug)?;
                    expr = AstNode::BinaryOp(Box::new(expr), BinaryOperator::Sub, Box::new(right));
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
            Some(Token::ListLength) => {
                self.advance(); // consume LENGTH
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after LENGTH".to_string());
                }
                let list = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')'".to_string());
                }
                Ok(AstNode::Length(Box::new(list)))
            }
            Some(Token::Identifier(_)) => {
                let name = match self.advance() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected identifier".to_string()),
                };

                // Check for list access after identifier
                if self.match_token(&Token::OpenBracket) {
                    let index = self.parse_expression(debug)?;
                    if !self.match_token(&Token::CloseBracket) {
                        return Err("Expected ']' after index".to_string());
                    }
                    Ok(AstNode::ListAccess(
                        Box::new(AstNode::Identifier(name)),
                        Box::new(index),
                    ))
                } else {
                    Ok(AstNode::Identifier(name))
                }
            }
            _ => match self.advance() {
                Some(Token::Integer(n)) => Ok(AstNode::Integer(n)),
                Some(Token::Float(f)) => Ok(AstNode::Float(f)),
                Some(Token::String(s)) => Ok(AstNode::String(s)),
                Some(Token::RawString(s)) => Ok(AstNode::RawString(s)),
                Some(Token::FormattedString(s, vars)) => Ok(AstNode::FormattedString(s, vars)),
                Some(Token::Boolean(b)) => Ok(AstNode::Boolean(b)),
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
        self.advance(); // consume CLASS
        let name = match self.advance() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected class name".to_string()),
        };
        let body = self.parse_block(debug)?;
        Ok(AstNode::ClassDecl(name, Box::new(body)))
    }

    fn parse_foreach(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance(); // consume FOR
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

        // Skip any newlines before the opening brace
        while let Some(Token::Newline) = self.peek() {
            Self::debug_print(debug, "Skipping newline before block");
            self.advance();
        }

        match self.peek() {
            Some(Token::OpenBrace) => {
                Self::debug_print(debug, "Found opening brace");
                self.advance();

                // Skip any newlines after opening brace
                while let Some(Token::Newline) = self.peek() {
                    Self::debug_print(debug, "Skipping newline after opening brace");
                    self.advance();
                }

                let mut statements = Vec::new();
                while let Some(token) = self.peek() {
                    if token == &Token::CloseBrace {
                        Self::debug_print(debug, "Found closing brace");
                        break;
                    }

                    // Parse the next statement
                    let stmt = self.parse_statement(debug)?;

                    // Only add non-empty blocks to statements
                    match stmt {
                        AstNode::Block(v) if v.is_empty() => {}
                        _ => statements.push(stmt),
                    }

                    // Skip any newlines between statements
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
        self.advance(); // consume PROCEDURE
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

    fn parse_display(&mut self, debug: bool) -> Result<AstNode, String> {
        match self.advance() {
            // consume DISPLAY
            Some(Token::Display(Some(boxed_token))) => {
                // Handle case where lexer already captured the string
                match *boxed_token {
                    Token::String(s) => Ok(AstNode::Display(Some(Box::new(AstNode::String(s))))),
                    _ => Err("Expected string literal after DISPLAY".to_string()),
                }
            }
            Some(Token::Display(None)) => {
                // Handle optional parentheses case
                if self.match_token(&Token::OpenParen) {
                    let expr = if self.peek() == Some(&Token::CloseParen) {
                        None
                    } else {
                        Some(Box::new(self.parse_expression(debug)?))
                    };
                    if !self.match_token(&Token::CloseParen) {
                        return Err("Expected ')' after expression".to_string());
                    }
                    Ok(AstNode::Display(expr))
                } else {
                    // Handle case without parentheses
                    if self.is_expression_start() {
                        Ok(AstNode::Display(Some(Box::new(
                            self.parse_expression(debug)?,
                        ))))
                    } else {
                        Ok(AstNode::Display(None))
                    }
                }
            }
            _ => Err("Expected DISPLAY token".to_string()),
        }
    }

    fn parse_display_inline(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance(); // consume DISPLAYINLINE
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after DISPLAYINLINE".to_string());
        }
        let expr = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')' after expression".to_string());
        }
        Ok(AstNode::DisplayInline(Box::new(expr)))
    }

    fn parse_comment(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance(); // consume COMMENT
        match self.advance() {
            Some(Token::String(text)) => Ok(AstNode::Comment(text)),
            _ => Err("Expected string after COMMENT".to_string()),
        }
    }

    fn parse_import(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance(); // consume IMPORT
        match self.advance() {
            Some(Token::String(path)) => Ok(AstNode::Import(path)),
            _ => Err("Expected string after IMPORT".to_string()),
        }
    }

    fn parse_assignment_or_call(&mut self, debug: bool) -> Result<AstNode, String> {
        let identifier = match self.advance() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected identifier".to_string()),
        };

        match self.peek() {
            Some(Token::OpenBracket) => {
                self.advance();
                let index = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseBracket) {
                    return Err("Expected ']'".to_string());
                }
                if self.match_token(&Token::Assign) {
                    let value = self.parse_expression(debug)?;
                    Ok(AstNode::ListAssignment(
                        Box::new(AstNode::Identifier(identifier)),
                        Box::new(index),
                        Box::new(value),
                    ))
                } else {
                    Ok(AstNode::ListAccess(
                        Box::new(AstNode::Identifier(identifier)),
                        Box::new(index),
                    ))
                }
            }
            Some(Token::Assign) => {
                self.advance();
                let value = self.parse_expression(debug)?;
                Ok(AstNode::Assignment(
                    Box::new(AstNode::Identifier(identifier)),
                    Box::new(value),
                ))
            }
            Some(Token::OpenParen) => {
                self.advance();
                let mut args = Vec::new();
                while let Some(token) = self.peek() {
                    if token == &Token::CloseParen {
                        break;
                    }
                    if !args.is_empty() {
                        if !self.match_token(&Token::Comma) {
                            return Err("Expected comma between arguments".to_string());
                        }
                    }
                    args.push(self.parse_expression(debug)?);
                }
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')'".to_string());
                }
                Ok(AstNode::ProcedureCall(identifier, args))
            }
            _ => Ok(AstNode::Identifier(identifier)),
        }
    }

    // Fix: Add error recovery
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
        while let Some(token) = self.peek() {
            if token == &Token::CloseBracket {
                break;
            }
            if !elements.is_empty() {
                if !self.match_token(&Token::Comma) {
                    return Err("Expected comma between list elements".to_string());
                }
            }
            elements.push(self.parse_expression(debug)?);
        }
        if !self.match_token(&Token::CloseBracket) {
            return Err("Expected ']'".to_string());
        }
        Ok(AstNode::List(elements))
    }

    fn parse_list_length(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance(); // consume LENGTH
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
        self.advance(); // consume REMOVE
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after REMOVE".to_string());
        }
        let list = self.parse_expression(debug)?;
        if !self.match_token(&Token::Comma) {
            return Err("Expected comma after list".to_string());
        }
        let index = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')'".to_string());
        }
        Ok(AstNode::Remove(Box::new(list), Box::new(index)))
    }

    fn parse_list_append(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance(); // consume APPEND
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
        self.advance(); // consume IF
        if !self.match_token(&Token::OpenParen) {
            return Err("Expected '(' after IF".to_string());
        }
        let condition = self.parse_expression(debug)?;
        if !self.match_token(&Token::CloseParen) {
            return Err("Expected ')' after condition".to_string());
        }
        let then_branch = self.parse_block(debug)?;

        // Skip any newlines after the 'then' block
        while let Some(Token::Newline) = self.peek() {
            self.advance();
        }

        // Handle ELSE and ELSE IF
        let else_branch = if self.match_token(&Token::Else) {
            // Check if this is an ELSE IF
            if self.peek() == Some(&Token::If) {
                // Parse it as another IF statement
                Some(Box::new(self.parse_if(debug)?))
            } else {
                // Regular ELSE block
                let else_block = self.parse_block(debug)?;
                Some(Box::new(else_block))
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
        self.advance(); // consume REPEAT

        match self.peek() {
            Some(Token::Until) => {
                self.advance(); // consume UNTIL
                if !self.match_token(&Token::OpenParen) {
                    return Err("Expected '(' after REPEAT UNTIL".to_string());
                }
                let condition = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err("Expected ')' after condition".to_string());
                }

                // Skip any newlines before the block
                while let Some(Token::Newline) = self.peek() {
                    self.advance();
                }

                let body = self.parse_block(debug)?;
                Ok(AstNode::RepeatUntil(Box::new(body), Box::new(condition)))
            }
            _ => {
                let times = self.parse_expression(debug)?;
                if !self.match_token(&Token::Times) {
                    return Err("Expected TIMES after repeat count".to_string());
                }
                let body = self.parse_block(debug)?;
                Ok(AstNode::RepeatTimes(Box::new(times), Box::new(body)))
            }
        }
    }

    fn parse_list_insert(&mut self, debug: bool) -> Result<AstNode, String> {
        self.advance(); // consume INSERT
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
        self.advance(); // consume RANDOM
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
        self.advance(); // consume SUBSTRING
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
        self.advance(); // consume CONCAT
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
        self.advance(); // consume TOSTRING
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
        self.advance(); // consume TONUM
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
