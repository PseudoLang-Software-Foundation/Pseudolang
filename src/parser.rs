use crate::error::{PSLError, Span};
use crate::lexer::{Lexer, Token};
use num_bigint::BigInt;

#[derive(Debug, Clone)]
pub struct Spanned {
    pub node: AstNode,
    pub span: Span,
}

impl Spanned {
    pub fn new(node: AstNode, span: Span) -> Self {
        Self { node, span }
    }
}

/// Tear the AST down iteratively instead of recursively.
///
/// The derived drop glue for `Spanned` recurses once per level of the tree, so a
/// deep AST aborts the process (SIGABRT, "stack overflow") when it is released —
/// *after* the program has already run correctly. The parser's
/// [`MAX_NESTING_DEPTH`] guard cannot prevent this, because a left-associative
/// chain like `1+1+...+1` is built by a *loop* in `parse_term`, so parse
/// recursion stays flat while the tree it produces gets arbitrarily deep.
///
/// So `drop` moves the children into an explicit worklist and releases the nodes
/// one at a time. Every node is pushed and popped at most once, so this is
/// linear, and a node is only ever released once its own children have been
/// taken (its `node` is [`AstNode::Null`]) or it never had any — either way the
/// inner `drop` bottoms out immediately instead of recursing.
impl Drop for Spanned {
    fn drop(&mut self) {
        // Fast path for leaves, which are the vast majority of any AST: nothing
        // to move, so let the ordinary drop glue release `self.node`.
        if self.node.is_childless() {
            return;
        }
        // `Vec::new` does not allocate, and only nodes that actually have
        // children are ever pushed, so tearing down one tree costs one Vec.
        let mut worklist: Vec<Spanned> = Vec::new();
        take_children(&mut self.node, &mut worklist);
        while let Some(mut child) = worklist.pop() {
            take_children(&mut child.node, &mut worklist);
            // `child` is released here holding an empty `AstNode::Null`.
        }
    }
}

/// Move every direct `Spanned` child of `node` into `out`, leaving `node` empty.
///
/// Children that are themselves leaves are released on the spot rather than
/// queued: that keeps the worklist proportional to the number of *interior*
/// nodes and keeps a wide literal such as `[1, 2, ..., 100000]` as cheap to drop
/// as it was before.
///
/// The match is deliberately exhaustive: a new [`AstNode`] variant that owns a
/// `Spanned` must be listed here, otherwise it would silently go back to being
/// dropped recursively.
fn take_children(node: &mut AstNode, out: &mut Vec<Spanned>) {
    fn push(out: &mut Vec<Spanned>, child: Spanned) {
        if !child.node.is_childless() {
            out.push(child);
        }
        // A leaf child is dropped right here, hitting the fast path above.
    }

    fn push_all(out: &mut Vec<Spanned>, children: impl IntoIterator<Item = Spanned>) {
        for child in children {
            push(out, child);
        }
    }

    match std::mem::replace(node, AstNode::Null) {
        AstNode::List(items)
        | AstNode::Block(items)
        | AstNode::Program(items)
        | AstNode::ProcedureCall(_, items)
        | AstNode::FormattedString(_, items) => push_all(out, items),

        AstNode::Dictionary(entries) => {
            for (key, value) in entries {
                push(out, key);
                push(out, value);
            }
        }

        AstNode::UnaryOp(_, a)
        | AstNode::Return(a)
        | AstNode::DisplayInline(a)
        | AstNode::Length(a)
        | AstNode::ToString(a)
        | AstNode::ToNum(a)
        | AstNode::Sort(a)
        | AstNode::ClassDecl(_, a)
        | AstNode::ProcedureDecl(_, _, a)
        | AstNode::Eval(a) => push(out, *a),

        AstNode::Display(a) | AstNode::Input(a) => push_all(out, a.map(|b| *b)),

        AstNode::Assignment(a, b)
        | AstNode::ListAccess(a, b)
        | AstNode::BinaryOp(a, _, b)
        | AstNode::RepeatTimes(a, b)
        | AstNode::RepeatUntil(a, b)
        | AstNode::ForEach(_, a, b)
        | AstNode::Random(a, b)
        | AstNode::Append(a, b)
        | AstNode::Remove(a, b)
        | AstNode::Concat(a, b) => {
            push(out, *a);
            push(out, *b);
        }

        AstNode::TryCatch {
            try_block: a,
            catch_block: b,
            ..
        } => {
            push(out, *a);
            push(out, *b);
        }

        AstNode::ListAssignment(a, b, c)
        | AstNode::Insert(a, b, c)
        | AstNode::Substring(a, b, c) => {
            push(out, *a);
            push(out, *b);
            push(out, *c);
        }

        AstNode::If(a, b, c) => {
            push(out, *a);
            push(out, *b);
            push_all(out, c.map(|d| *d));
        }

        // Leaves: nothing recursive to hand over.
        AstNode::Integer(_)
        | AstNode::Float(_)
        | AstNode::String(_)
        | AstNode::Boolean(_)
        | AstNode::Null
        | AstNode::NaN
        | AstNode::Identifier(_)
        | AstNode::Comment
        | AstNode::Import(_)
        | AstNode::RawString(_) => {}
    }
}

#[derive(Debug, Clone)]
pub enum AstNode {
    Integer(BigInt),
    Float(f64),
    String(String),
    Boolean(bool),
    List(Vec<Spanned>),
    Dictionary(Vec<(Spanned, Spanned)>),
    Null,
    NaN,

    Identifier(String),
    Assignment(Box<Spanned>, Box<Spanned>),
    ListAccess(Box<Spanned>, Box<Spanned>),
    ListAssignment(Box<Spanned>, Box<Spanned>, Box<Spanned>),

    BinaryOp(Box<Spanned>, BinaryOperator, Box<Spanned>),
    UnaryOp(UnaryOperator, Box<Spanned>),

    If(Box<Spanned>, Box<Spanned>, Option<Box<Spanned>>),
    RepeatTimes(Box<Spanned>, Box<Spanned>),
    RepeatUntil(Box<Spanned>, Box<Spanned>),
    ForEach(String, Box<Spanned>, Box<Spanned>),

    ProcedureDecl(String, Vec<String>, Box<Spanned>),
    ProcedureCall(String, Vec<Spanned>),
    Return(Box<Spanned>),

    Display(Option<Box<Spanned>>),
    DisplayInline(Box<Spanned>),
    Input(Option<Box<Spanned>>),
    Random(Box<Spanned>, Box<Spanned>),
    Insert(Box<Spanned>, Box<Spanned>, Box<Spanned>),
    Append(Box<Spanned>, Box<Spanned>),
    Remove(Box<Spanned>, Box<Spanned>),
    Length(Box<Spanned>),
    Substring(Box<Spanned>, Box<Spanned>, Box<Spanned>),
    Concat(Box<Spanned>, Box<Spanned>),
    ToString(Box<Spanned>),
    ToNum(Box<Spanned>),
    Sort(Box<Spanned>),

    ClassDecl(String, Box<Spanned>),

    Block(Vec<Spanned>),
    Program(Vec<Spanned>),
    Comment,
    Import(String),

    RawString(String),
    FormattedString(String, Vec<Spanned>),
    TryCatch {
        try_block: Box<Spanned>,
        error_var: Option<String>,
        catch_block: Box<Spanned>,
    },
    Eval(Box<Spanned>),
}

impl AstNode {
    /// True for the variants that own no [`Spanned`] at all.
    ///
    /// Used by [`Drop for Spanned`](Spanned) to skip the iterative teardown
    /// entirely for leaf nodes. Kept in sync with the leaf arm of
    /// `take_children`.
    fn is_childless(&self) -> bool {
        matches!(
            self,
            AstNode::Integer(_)
                | AstNode::Float(_)
                | AstNode::String(_)
                | AstNode::Boolean(_)
                | AstNode::Null
                | AstNode::NaN
                | AstNode::Identifier(_)
                | AstNode::Comment
                | AstNode::Import(_)
                | AstNode::RawString(_)
        )
    }
}

#[derive(Debug, Clone)]
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

impl BinaryOperator {
    pub fn is_comparison(&self) -> bool {
        matches!(
            self,
            BinaryOperator::Eq
                | BinaryOperator::NotEq
                | BinaryOperator::Lt
                | BinaryOperator::LtEq
                | BinaryOperator::Gt
                | BinaryOperator::GtEq
        )
    }

    pub fn is_arithmetic(&self) -> bool {
        matches!(
            self,
            BinaryOperator::Add
                | BinaryOperator::Sub
                | BinaryOperator::Mul
                | BinaryOperator::Div
                | BinaryOperator::Mod
        )
    }
}

#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Not,
    Neg,
}

/// Hard cap on how deeply expressions and blocks may nest.
///
/// The parser is recursive descent, so every nested `(`, `[`, call argument or
/// `{` block costs a stack frame. Without a limit a deeply nested source file
/// exhausts the stack and the process dies with SIGABRT instead of reporting an
/// error. The real ceiling is build- and host-dependent (~2400 levels for a
/// release build on an 8 MiB main thread, but only ~375 for a debug build and
/// far less inside a 1 MiB WebAssembly stack), so the limit is set well below
/// the smallest of those while still being far more nesting than any real
/// PseudoLang program needs.
pub const MAX_NESTING_DEPTH: usize = 128;

pub struct Parser {
    tokens: Vec<(Token, Span)>,
    current: usize,
    /// Current recursion depth of `parse_expression` / `parse_block`.
    depth: usize,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, Span)>) -> Self {
        Parser {
            tokens,
            current: 0,
            depth: 0,
        }
    }

    /// Run `body` one nesting level deeper, refusing to recurse past
    /// [`MAX_NESTING_DEPTH`] so that runaway nesting produces a `PSLError`
    /// rather than a stack overflow.
    fn nested<T>(
        &mut self,
        body: impl FnOnce(&mut Self) -> Result<T, PSLError>,
    ) -> Result<T, PSLError> {
        if self.depth >= MAX_NESTING_DEPTH {
            return Err(self.create_error(&format!(
                "Maximum nesting depth exceeded (limit: {})",
                MAX_NESTING_DEPTH
            )));
        }
        self.depth += 1;
        let result = body(self);
        self.depth -= 1;
        result
    }

    fn debug_print(debug: bool, message: &str) {
        if debug {
            eprintln!("\x1b[33m[PARSER DEBUG]\x1b[0m {}", message);
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current).map(|(t, _)| t)
    }

    fn peek_span(&self) -> Span {
        self.tokens
            .get(self.current)
            .map(|(_, s)| *s)
            .unwrap_or_default()
    }

    fn prev_span(&self) -> Span {
        if self.current > 0 {
            self.tokens[self.current - 1].1
        } else {
            Span::default()
        }
    }

    fn advance(&mut self) -> Option<Token> {
        if self.current < self.tokens.len() {
            let token = self.tokens[self.current].0.clone();
            self.current += 1;
            Some(token)
        } else {
            None
        }
    }

    fn match_token(&mut self, expected: &Token) -> bool {
        if let Some(token) = self.peek()
            && token == expected
        {
            self.advance();
            return true;
        }
        false
    }

    fn spanned_from(&self, node: AstNode, start: usize) -> Spanned {
        let end = self.prev_span().end;
        Spanned {
            node,
            span: Span::new(start, end),
        }
    }

    fn parse_program(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        Self::debug_print(debug, "Starting program parse");
        let start = self.peek_span().start;
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
        Ok(self.spanned_from(AstNode::Program(statements), start))
    }

    // skipcq: RS-R1000
    fn parse_statement(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        // Leading newlines are skipped in a loop. This used to be a tail call
        // back into `parse_statement`, which rustc is not obliged to turn into a
        // jump, so a file with a long enough run of blank lines overflowed the
        // stack. A blank line is not "nesting", so this must not cost depth.
        while matches!(self.peek(), Some(Token::Newline)) {
            Self::debug_print(debug, "Found newline, skipping");
            self.advance();
        }

        Self::debug_print(
            debug,
            &format!("Parsing statement at position {}", self.current),
        );
        let start = self.peek_span().start;

        match self.peek() {
            Some(Token::Try) => {
                self.advance();
                let try_block = self.parse_block(debug)?;

                while matches!(self.peek(), Some(Token::Newline)) {
                    self.advance();
                }

                if !self.match_token(&Token::Catch) {
                    return Err(self.create_error("Expected 'catch' after try block"));
                }

                let mut error_var = None;
                if self.match_token(&Token::OpenParen) {
                    if let Some(Token::Identifier(name)) = self.advance() {
                        error_var = Some(name);
                    } else {
                        return Err(self.create_error("Expected identifier after 'catch('"));
                    }

                    if !self.match_token(&Token::CloseParen) {
                        return Err(self.create_error("Expected ')' after catch variable"));
                    }
                }

                let catch_block = self.parse_block(debug)?;

                Ok(self.spanned_from(
                    AstNode::TryCatch {
                        try_block: Box::new(try_block),
                        error_var,
                        catch_block: Box::new(catch_block),
                    },
                    start,
                ))
            }
            Some(Token::ListAppend) => self.parse_builtin(debug, "APPEND", 2, |mut a| {
                AstNode::Append(Box::new(a.remove(0)), Box::new(a.remove(0)))
            }),
            Some(Token::ListRemove) => self.parse_builtin(debug, "REMOVE", 2, |mut a| {
                AstNode::Remove(Box::new(a.remove(0)), Box::new(a.remove(0)))
            }),
            Some(Token::ListInsert) => self.parse_builtin(debug, "INSERT", 3, |mut a| {
                AstNode::Insert(
                    Box::new(a.remove(0)),
                    Box::new(a.remove(0)),
                    Box::new(a.remove(0)),
                )
            }),
            Some(Token::Random) => self.parse_builtin(debug, "RANDOM", 2, |mut a| {
                AstNode::Random(Box::new(a.remove(0)), Box::new(a.remove(0)))
            }),
            Some(Token::Substring) => self.parse_builtin(debug, "SUBSTRING", 3, |mut a| {
                AstNode::Substring(
                    Box::new(a.remove(0)),
                    Box::new(a.remove(0)),
                    Box::new(a.remove(0)),
                )
            }),
            Some(Token::Concat) => self.parse_builtin(debug, "CONCAT", 2, |mut a| {
                AstNode::Concat(Box::new(a.remove(0)), Box::new(a.remove(0)))
            }),
            Some(Token::ToString) => self.parse_builtin(debug, "TOSTRING", 1, |mut a| {
                AstNode::ToString(Box::new(a.remove(0)))
            }),
            Some(Token::ToNum) => self.parse_builtin(debug, "TONUM", 1, |mut a| {
                AstNode::ToNum(Box::new(a.remove(0)))
            }),
            Some(Token::ListLength) => self.parse_builtin(debug, "LENGTH", 1, |mut a| {
                AstNode::Length(Box::new(a.remove(0)))
            }),
            Some(Token::Sort) => self.parse_builtin(debug, "SORT", 1, |mut a| {
                AstNode::Sort(Box::new(a.remove(0)))
            }),
            Some(Token::Identifier(_)) => {
                let identifier = match self.advance() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err(self.create_error("Expected identifier")),
                };

                let mut list_accesses = Vec::new();
                while matches!(self.peek(), Some(Token::OpenBracket)) {
                    self.advance();
                    let index = self.parse_expression(debug)?;
                    if !self.match_token(&Token::CloseBracket) {
                        return Err(self.create_error("Expected ']'"));
                    }
                    list_accesses.push(index);
                }

                match self.peek() {
                    Some(Token::Assign) => {
                        self.advance();
                        let value = self.parse_expression(debug)?;

                        if list_accesses.is_empty() {
                            let ident_span = Span::new(start, start + identifier.len());
                            let target = Spanned::new(AstNode::Identifier(identifier), ident_span);
                            Ok(self.spanned_from(
                                AstNode::Assignment(Box::new(target), Box::new(value)),
                                start,
                            ))
                        } else {
                            let ident_span = Span::new(start, start + identifier.len());
                            let mut current =
                                Spanned::new(AstNode::Identifier(identifier), ident_span);
                            for (i, index) in list_accesses.iter().enumerate() {
                                if i == list_accesses.len() - 1 {
                                    let node = AstNode::ListAssignment(
                                        Box::new(current),
                                        Box::new(index.clone()),
                                        Box::new(value.clone()),
                                    );
                                    return Ok(self.spanned_from(node, start));
                                } else {
                                    let access_span = Span::new(start, index.span.end);
                                    current = Spanned::new(
                                        AstNode::ListAccess(
                                            Box::new(current),
                                            Box::new(index.clone()),
                                        ),
                                        access_span,
                                    );
                                }
                            }
                            unreachable!()
                        }
                    }
                    Some(Token::OpenParen) => {
                        self.advance();
                        let mut args = Vec::new();
                        while !self.match_token(&Token::CloseParen) {
                            if !args.is_empty() && !self.match_token(&Token::Comma) {
                                return Err(self.create_error("Expected comma between arguments"));
                            }
                            args.push(self.parse_expression(debug)?);
                        }
                        Ok(self.spanned_from(AstNode::ProcedureCall(identifier, args), start))
                    }
                    _ => {
                        if list_accesses.is_empty() {
                            Ok(self.spanned_from(AstNode::Identifier(identifier), start))
                        } else {
                            let ident_span = Span::new(start, start + identifier.len());
                            let mut current =
                                Spanned::new(AstNode::Identifier(identifier), ident_span);
                            for index in list_accesses {
                                let access_span = Span::new(start, index.span.end);
                                current = Spanned::new(
                                    AstNode::ListAccess(Box::new(current), Box::new(index)),
                                    access_span,
                                );
                            }
                            Ok(current)
                        }
                    }
                }
            }
            None => {
                Self::debug_print(debug, "End of input reached");
                Ok(Spanned::new(AstNode::Block(Vec::new()), Span::default()))
            }
            Some(Token::CloseBrace) => {
                Ok(Spanned::new(AstNode::Block(Vec::new()), self.peek_span()))
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
            Some(Token::Repeat) => {
                Self::debug_print(debug, "Starting repeat parse");
                self.parse_repeat(debug)
            }
            Some(Token::For) => self.parse_foreach(debug),
            Some(Token::Class) => self.parse_class(debug),
            Some(Token::Display(_)) => {
                self.advance();

                if !self.match_token(&Token::OpenParen) {
                    return Err(self.create_error("Expected '(' after DISPLAY"));
                }
                if self.match_token(&Token::CloseParen) {
                    Ok(self.spanned_from(AstNode::Display(None), start))
                } else {
                    let expr = self.parse_expression(debug)?;
                    if !self.match_token(&Token::CloseParen) {
                        return Err(self.create_error("Expected ')' after expression"));
                    }
                    Ok(self.spanned_from(AstNode::Display(Some(Box::new(expr))), start))
                }
            }
            Some(Token::DisplayInline) => self.parse_builtin(debug, "DISPLAYINLINE", 1, |mut a| {
                AstNode::DisplayInline(Box::new(a.remove(0)))
            }),
            Some(Token::Comment) => self.parse_comment(),
            Some(Token::Import) => self.parse_import(),
            Some(Token::Return) => {
                self.advance();
                if matches!(self.peek(), Some(Token::OpenParen)) {
                    self.advance();
                    if matches!(self.peek(), Some(Token::CloseParen)) {
                        self.advance();
                        let empty = Spanned::new(AstNode::Block(vec![]), self.prev_span());
                        Ok(self.spanned_from(AstNode::Return(Box::new(empty)), start))
                    } else {
                        let expr = self.parse_expression(debug)?;
                        if !self.match_token(&Token::CloseParen) {
                            return Err(self.create_error("Expected ')' after return expression"));
                        }
                        Ok(self.spanned_from(AstNode::Return(Box::new(expr)), start))
                    }
                } else if self.is_expression_start()
                    || matches!(self.peek(), Some(Token::OpenBrace))
                {
                    let expr = self.parse_expression(debug)?;
                    Ok(self.spanned_from(AstNode::Return(Box::new(expr)), start))
                } else {
                    let empty = Spanned::new(AstNode::Block(vec![]), self.prev_span());
                    Ok(self.spanned_from(AstNode::Return(Box::new(empty)), start))
                }
            }
            Some(Token::Input) => {
                self.advance();
                if !self.match_token(&Token::OpenParen) {
                    return Err(self.create_error("Expected '(' after INPUT"));
                }
                let prompt = if self.peek() != Some(&Token::CloseParen) {
                    Some(Box::new(self.parse_expression(debug)?))
                } else {
                    None
                };
                if !self.match_token(&Token::CloseParen) {
                    return Err(self.create_error("Expected ')' after INPUT"));
                }
                Ok(self.spanned_from(AstNode::Input(prompt), start))
            }
            Some(Token::Eval) => self.parse_builtin(debug, "EVAL", 1, |mut a| {
                AstNode::Eval(Box::new(a.remove(0)))
            }),
            _ => {
                Self::debug_print(
                    debug,
                    &format!("Unexpected token in statement: {:?}", self.peek()),
                );
                Err(self.create_error("Unexpected token in statement"))
            }
        }
    }

    fn is_expression_start(&self) -> bool {
        matches!(
            self.peek(),
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
                | Some(Token::Input)
        )
    }

    /// Depth-guarded entry point for expression parsing; every nested
    /// sub-expression goes through here so runaway nesting is caught.
    pub fn parse_expression(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        self.nested(|p| p.parse_expression_inner(debug))
    }

    fn parse_expression_inner(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        if self.match_token(&Token::Sort) {
            if !self.match_token(&Token::OpenParen) {
                return Err(self.create_error("Expected '(' after SORT"));
            }
            let list_expr = self.parse_expression(debug)?;
            if !self.match_token(&Token::CloseParen) {
                return Err(self.create_error("Expected ')' after list expression"));
            }
            Ok(self.spanned_from(AstNode::Sort(Box::new(list_expr)), start))
        } else {
            self.parse_logical_or(debug)
        }
    }

    fn parse_logical_or(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        let mut expr = self.parse_logical_and(debug)?;

        while self.match_token(&Token::Or) {
            let right = self.parse_logical_and(debug)?;
            expr = self.spanned_from(
                AstNode::BinaryOp(Box::new(expr), BinaryOperator::Or, Box::new(right)),
                start,
            );
        }
        Ok(expr)
    }

    fn parse_logical_and(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        let mut expr = self.parse_equality(debug)?;

        while self.match_token(&Token::And) {
            let right = self.parse_equality(debug)?;
            expr = self.spanned_from(
                AstNode::BinaryOp(Box::new(expr), BinaryOperator::And, Box::new(right)),
                start,
            );
        }
        Ok(expr)
    }

    fn parse_equality(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
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
                    expr = self.spanned_from(
                        AstNode::BinaryOp(Box::new(expr), op, Box::new(right)),
                        start,
                    );
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
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
                    expr = self.spanned_from(
                        AstNode::BinaryOp(Box::new(expr), op, Box::new(right)),
                        start,
                    );
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_term(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        let mut expr = self.parse_factor(debug)?;

        while let Some(token) = self.peek() {
            match token {
                Token::Plus => {
                    self.advance();
                    let right = self.parse_factor(debug)?;
                    expr = self.spanned_from(
                        AstNode::BinaryOp(Box::new(expr), BinaryOperator::Add, Box::new(right)),
                        start,
                    );
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_factor(debug)?;
                    expr = self.spanned_from(
                        AstNode::BinaryOp(Box::new(expr), BinaryOperator::Sub, Box::new(right)),
                        start,
                    );
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_factor(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        let mut expr = self.parse_unary(debug)?;

        while let Some(token) = self.peek() {
            match token {
                Token::Multiply => {
                    self.advance();
                    let right = self.parse_unary(debug)?;
                    expr = self.spanned_from(
                        AstNode::BinaryOp(Box::new(expr), BinaryOperator::Mul, Box::new(right)),
                        start,
                    );
                }
                Token::Divide => {
                    self.advance();
                    let right = self.parse_unary(debug)?;
                    expr = self.spanned_from(
                        AstNode::BinaryOp(Box::new(expr), BinaryOperator::Div, Box::new(right)),
                        start,
                    );
                }
                Token::Modulo => {
                    self.advance();
                    let right = self.parse_unary(debug)?;
                    expr = self.spanned_from(
                        AstNode::BinaryOp(Box::new(expr), BinaryOperator::Mod, Box::new(right)),
                        start,
                    );
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_unary(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        if let Some(token) = self.peek() {
            match token {
                // Both prefix operators recurse into `parse_unary`, so they must
                // go through `nested`: without it `NOT NOT NOT ...` (or `- - -
                // ...`) grows the stack without ever moving the depth counter.
                Token::Not => {
                    self.advance();
                    let expr = self.nested(|p| p.parse_unary(debug))?;
                    Ok(self
                        .spanned_from(AstNode::UnaryOp(UnaryOperator::Not, Box::new(expr)), start))
                }
                Token::Minus => {
                    self.advance();
                    let expr = self.nested(|p| p.parse_unary(debug))?;
                    Ok(self
                        .spanned_from(AstNode::UnaryOp(UnaryOperator::Neg, Box::new(expr)), start))
                }
                _ => self.parse_primary(debug),
            }
        } else {
            Err(self.create_error("Unexpected end of input"))
        }
    }

    // skipcq: RS-R1000
    fn parse_primary(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        match self.peek() {
            Some(Token::ListAppend) => self.parse_builtin(debug, "APPEND", 2, |mut a| {
                AstNode::Append(Box::new(a.remove(0)), Box::new(a.remove(0)))
            }),
            Some(Token::ListRemove) => self.parse_builtin(debug, "REMOVE", 2, |mut a| {
                AstNode::Remove(Box::new(a.remove(0)), Box::new(a.remove(0)))
            }),
            Some(Token::ListInsert) => self.parse_builtin(debug, "INSERT", 3, |mut a| {
                AstNode::Insert(
                    Box::new(a.remove(0)),
                    Box::new(a.remove(0)),
                    Box::new(a.remove(0)),
                )
            }),
            Some(Token::ListLength) => self.parse_builtin(debug, "LENGTH", 1, |mut a| {
                AstNode::Length(Box::new(a.remove(0)))
            }),
            Some(Token::Random) => self.parse_builtin(debug, "RANDOM", 2, |mut a| {
                AstNode::Random(Box::new(a.remove(0)), Box::new(a.remove(0)))
            }),
            Some(Token::Substring) => self.parse_builtin(debug, "SUBSTRING", 3, |mut a| {
                AstNode::Substring(
                    Box::new(a.remove(0)),
                    Box::new(a.remove(0)),
                    Box::new(a.remove(0)),
                )
            }),
            Some(Token::Concat) => self.parse_builtin(debug, "CONCAT", 2, |mut a| {
                AstNode::Concat(Box::new(a.remove(0)), Box::new(a.remove(0)))
            }),
            Some(Token::ToString) => self.parse_builtin(debug, "TOSTRING", 1, |mut a| {
                AstNode::ToString(Box::new(a.remove(0)))
            }),
            Some(Token::ToNum) => self.parse_builtin(debug, "TONUM", 1, |mut a| {
                AstNode::ToNum(Box::new(a.remove(0)))
            }),
            Some(Token::Sort) => self.parse_builtin(debug, "SORT", 1, |mut a| {
                AstNode::Sort(Box::new(a.remove(0)))
            }),
            Some(Token::Identifier(_)) => {
                let name = match self.advance() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err(self.create_error("Expected identifier")),
                };

                let ident_span = Span::new(start, self.prev_span().end);
                let mut node = Spanned::new(AstNode::Identifier(name.clone()), ident_span);

                while matches!(self.peek(), Some(Token::OpenBracket)) {
                    self.advance();
                    let index = self.parse_expression(debug)?;
                    if !self.match_token(&Token::CloseBracket) {
                        return Err(self.create_error("Expected ']' after list index"));
                    }
                    let access_end = self.prev_span().end;
                    node = Spanned::new(
                        AstNode::ListAccess(Box::new(node), Box::new(index)),
                        Span::new(start, access_end),
                    );
                }

                if self.match_token(&Token::OpenParen) {
                    let mut args = Vec::new();
                    while !self.match_token(&Token::CloseParen) {
                        if !args.is_empty() && !self.match_token(&Token::Comma) {
                            return Err(self.create_error("Expected comma between arguments"));
                        }
                        args.push(self.parse_expression(debug)?);
                    }
                    return Ok(self.spanned_from(AstNode::ProcedureCall(name, args), start));
                }

                Ok(node)
            }
            Some(Token::FormattedString(_, _)) => {
                let (template, vars) = match self.peek() {
                    Some(Token::FormattedString(t, v)) => (t.clone(), v.clone()),
                    _ => unreachable!(),
                };
                let fs_span = self.peek_span();
                self.advance();
                let mut expressions = Vec::new();
                for var in vars {
                    let mut var_lexer = Lexer::new(&var);
                    let var_tokens = var_lexer.tokenize();
                    // Each interpolation slot gets its own Parser, so it must
                    // inherit the current depth: otherwise nesting restarts at
                    // zero every level and `f"{f"{...}"}"` escapes the guard.
                    let mut var_parser = Parser::new(var_tokens);
                    var_parser.depth = self.depth + 1;
                    let mut expr = var_parser.parse_expression(debug)?;
                    expr.span = fs_span;
                    expressions.push(expr);
                }
                Ok(self.spanned_from(AstNode::FormattedString(template, expressions), start))
            }
            Some(Token::Input) => {
                self.advance();
                if !self.match_token(&Token::OpenParen) {
                    return Err(self.create_error("Expected '(' after INPUT"));
                }
                let prompt = if self.peek() != Some(&Token::CloseParen) {
                    Some(Box::new(self.parse_expression(debug)?))
                } else {
                    None
                };
                if !self.match_token(&Token::CloseParen) {
                    return Err(self.create_error("Expected ')' after INPUT"));
                }
                Ok(self.spanned_from(AstNode::Input(prompt), start))
            }
            Some(Token::Eval) => {
                self.advance();
                if !self.match_token(&Token::OpenParen) {
                    return Err(self.create_error("Expected '(' after EVAL"));
                }
                let expr = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err(self.create_error("Expected ')' after EVAL expression"));
                }
                Ok(self.spanned_from(AstNode::Eval(Box::new(expr)), start))
            }
            _ => match self.advance() {
                Some(Token::Integer(n)) => Ok(Spanned::new(
                    AstNode::Integer(n),
                    Span::new(start, self.prev_span().end),
                )),
                Some(Token::Float(f)) => Ok(Spanned::new(
                    AstNode::Float(f),
                    Span::new(start, self.prev_span().end),
                )),
                Some(Token::String(s)) => Ok(Spanned::new(
                    AstNode::String(s),
                    Span::new(start, self.prev_span().end),
                )),
                Some(Token::RawString(s)) => Ok(Spanned::new(
                    AstNode::RawString(s),
                    Span::new(start, self.prev_span().end),
                )),
                Some(Token::Boolean(b)) => Ok(Spanned::new(
                    AstNode::Boolean(b),
                    Span::new(start, self.prev_span().end),
                )),
                Some(Token::Null) => Ok(Spanned::new(
                    AstNode::Null,
                    Span::new(start, self.prev_span().end),
                )),
                Some(Token::NaN) => Ok(Spanned::new(
                    AstNode::NaN,
                    Span::new(start, self.prev_span().end),
                )),
                Some(Token::Identifier(name)) => Ok(Spanned::new(
                    AstNode::Identifier(name),
                    Span::new(start, self.prev_span().end),
                )),
                Some(Token::OpenParen) => {
                    let expr = self.parse_expression(debug)?;
                    if !self.match_token(&Token::CloseParen) {
                        return Err(self.create_error("Expected ')' after expression"));
                    }
                    Ok(expr)
                }
                Some(Token::OpenBracket) => self.parse_list(debug, start),
                Some(Token::OpenBrace) => self.parse_dict(debug, start),
                _ => Err(self.create_error("Unexpected token in expression")),
            },
        }
    }

    fn parse_class(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        self.advance();
        let name = match self.advance() {
            Some(Token::Identifier(name)) => name,
            _ => return Err(self.create_error("Expected class name")),
        };
        let body = self.parse_block(debug)?;
        Ok(self.spanned_from(AstNode::ClassDecl(name, Box::new(body)), start))
    }

    fn parse_foreach(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        self.advance();
        if !self.match_token(&Token::Each) {
            return Err(self.create_error("Expected EACH after FOR"));
        }
        let var_name = match self.advance() {
            Some(Token::Identifier(name)) => name,
            _ => return Err(self.create_error("Expected identifier after EACH")),
        };
        if !self.match_token(&Token::In) {
            return Err(self.create_error("Expected IN after identifier"));
        }

        let list = self.parse_expression(debug)?;
        let body = self.parse_block(debug)?;
        Ok(self.spanned_from(
            AstNode::ForEach(var_name, Box::new(list), Box::new(body)),
            start,
        ))
    }

    fn parse_block(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        self.nested(|p| p.parse_block_inner(debug))
    }

    fn parse_block_inner(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        Self::debug_print(
            debug,
            &format!("Parsing block, current token: {:?}", self.peek()),
        );

        while matches!(self.peek(), Some(Token::Newline)) {
            Self::debug_print(debug, "Skipping newline before block");
            self.advance();
        }

        let start = self.peek_span().start;

        match self.peek() {
            Some(Token::OpenBrace) => {
                Self::debug_print(debug, "Found opening brace");
                self.advance();

                while matches!(self.peek(), Some(Token::Newline)) {
                    Self::debug_print(debug, "Skipping newline after opening brace");
                    self.advance();
                }

                let mut statements = Vec::new();
                while let Some(token) = self.peek() {
                    if token == &Token::CloseBrace {
                        break;
                    }

                    let stmt = self.parse_statement(debug)?;
                    match &stmt.node {
                        AstNode::Block(v) if v.is_empty() => {}
                        _ => statements.push(stmt),
                    }

                    while matches!(self.peek(), Some(Token::Newline)) {
                        Self::debug_print(debug, "Skipping newline between statements");
                        self.advance();
                    }
                }

                if !self.match_token(&Token::CloseBrace) {
                    return Err(self.create_error("Expected '}' at end of block"));
                }

                Self::debug_print(debug, "Block parsing complete");
                Ok(self.spanned_from(AstNode::Block(statements), start))
            }
            _ => Err(self.create_error("Expected '{' to start block")),
        }
    }

    fn parse_procedure(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        self.advance();
        let name = match self.advance() {
            Some(Token::Identifier(name)) => name,
            _ => return Err(self.create_error("Expected procedure name")),
        };
        if !self.match_token(&Token::OpenParen) {
            return Err(self.create_error("Expected '(' after procedure name"));
        }
        let mut params = Vec::new();
        while let Some(token) = self.peek() {
            if token == &Token::CloseParen {
                break;
            }
            if !params.is_empty() && !self.match_token(&Token::Comma) {
                return Err(self.create_error("Expected comma between parameters"));
            }
            match self.advance() {
                Some(Token::Identifier(param)) => params.push(param),
                _ => return Err(self.create_error("Expected parameter name")),
            }
        }
        if !self.match_token(&Token::CloseParen) {
            return Err(self.create_error("Expected ')' after parameters"));
        }
        let body = self.parse_block(debug)?;
        Ok(self.spanned_from(AstNode::ProcedureDecl(name, params, Box::new(body)), start))
    }

    fn parse_builtin(
        &mut self,
        debug: bool,
        name: &str,
        n_args: usize,
        make: impl FnOnce(Vec<Spanned>) -> AstNode,
    ) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        self.advance();
        if !self.match_token(&Token::OpenParen) {
            return Err(self.create_error(&format!("Expected '(' after {name}")));
        }
        let mut args = Vec::with_capacity(n_args);
        for i in 0..n_args {
            if i > 0 && !self.match_token(&Token::Comma) {
                return Err(self.create_error("Expected comma between arguments"));
            }
            args.push(self.parse_expression(debug)?);
        }
        if !self.match_token(&Token::CloseParen) {
            return Err(self.create_error("Expected ')'"));
        }
        Ok(self.spanned_from(make(args), start))
    }

    fn parse_comment(&mut self) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        self.advance();
        match self.advance() {
            Some(Token::String(_)) => Ok(self.spanned_from(AstNode::Comment, start)),
            _ => Err(self.create_error("Expected string after COMMENT")),
        }
    }

    fn parse_import(&mut self) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        self.advance();
        match self.advance() {
            Some(Token::String(path)) => Ok(self.spanned_from(AstNode::Import(path), start)),
            _ => Err(self.create_error("Expected string after IMPORT")),
        }
    }

    fn parse_list(&mut self, debug: bool, start: usize) -> Result<Spanned, PSLError> {
        let mut elements = Vec::new();
        loop {
            while matches!(self.peek(), Some(Token::Newline)) {
                self.advance();
            }

            if matches!(self.peek(), Some(Token::CloseBracket)) {
                self.advance();
                break;
            }

            if !elements.is_empty() {
                if !self.match_token(&Token::Comma) {
                    return Err(self.create_error("Expected comma between list elements"));
                }
                while matches!(self.peek(), Some(Token::Newline)) {
                    self.advance();
                }
            }

            elements.push(self.parse_expression(debug)?);

            while matches!(self.peek(), Some(Token::Newline)) {
                self.advance();
            }
        }
        Ok(self.spanned_from(AstNode::List(elements), start))
    }

    fn parse_dict(&mut self, debug: bool, start: usize) -> Result<Spanned, PSLError> {
        let mut entries: Vec<(Spanned, Spanned)> = Vec::new();
        loop {
            while matches!(self.peek(), Some(Token::Newline)) {
                self.advance();
            }

            if matches!(self.peek(), Some(Token::CloseBrace)) {
                self.advance();
                break;
            }

            if !entries.is_empty() {
                if !self.match_token(&Token::Comma) {
                    return Err(self.create_error("Expected comma between dictionary entries"));
                }
                while matches!(self.peek(), Some(Token::Newline)) {
                    self.advance();
                }
            }

            let key = self.parse_expression(debug)?;

            while matches!(self.peek(), Some(Token::Newline)) {
                self.advance();
            }

            if !self.match_token(&Token::Colon) {
                return Err(self.create_error("Expected ':' after dictionary key"));
            }

            while matches!(self.peek(), Some(Token::Newline)) {
                self.advance();
            }

            let value = self.parse_expression(debug)?;
            entries.push((key, value));

            while matches!(self.peek(), Some(Token::Newline)) {
                self.advance();
            }
        }
        Ok(self.spanned_from(AstNode::Dictionary(entries), start))
    }

    fn parse_if(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        self.advance();
        let condition = if self.match_token(&Token::OpenParen) {
            let expr = self.parse_expression(debug)?;
            if !self.match_token(&Token::CloseParen) {
                return Err(self.create_error("Expected ')' after condition"));
            }
            expr
        } else {
            self.parse_expression(debug)?
        };

        let then_branch = self.parse_block(debug)?;

        while matches!(self.peek(), Some(Token::Newline)) {
            self.advance();
        }

        let else_branch = if self.peek() == Some(&Token::Else) {
            self.advance();

            while matches!(self.peek(), Some(Token::Newline)) {
                self.advance();
            }

            if self.peek() == Some(&Token::If) {
                // `ELSE IF` recurses into `parse_if` *after* `parse_block` has
                // already returned and given its depth back, so an `ELSE IF`
                // chain would otherwise accumulate stack frames at depth 0.
                Some(Box::new(self.nested(|p| p.parse_if(debug))?))
            } else {
                Some(Box::new(self.parse_block(debug)?))
            }
        } else {
            None
        };

        Ok(self.spanned_from(
            AstNode::If(Box::new(condition), Box::new(then_branch), else_branch),
            start,
        ))
    }

    fn parse_repeat(&mut self, debug: bool) -> Result<Spanned, PSLError> {
        let start = self.peek_span().start;
        Self::debug_print(debug, "Starting repeat parse");
        self.advance();

        if self.peek() == Some(&Token::Until) {
            self.advance();
            let condition = if self.match_token(&Token::OpenParen) {
                let expr = self.parse_expression(debug)?;
                if !self.match_token(&Token::CloseParen) {
                    return Err(self.create_error("Expected ')' after condition"));
                }
                expr
            } else {
                self.parse_expression(debug)?
            };

            while matches!(self.peek(), Some(Token::Newline)) {
                self.advance();
            }

            let body = self.parse_block(debug)?;
            Ok(self.spanned_from(
                AstNode::RepeatUntil(Box::new(body), Box::new(condition)),
                start,
            ))
        } else {
            let times = self.parse_expression(debug)?;
            if !self.match_token(&Token::Times) {
                return Err(self.create_error("Expected TIMES after repeat count"));
            }
            let body = self.parse_block(debug)?;
            Ok(self.spanned_from(AstNode::RepeatTimes(Box::new(times), Box::new(body)), start))
        }
    }

    fn create_error(&self, message: &str) -> PSLError {
        PSLError::with_span(message, self.peek_span())
    }
}

pub fn parse(tokens: Vec<(Token, Span)>, debug: bool) -> Result<Spanned, PSLError> {
    let mut parser = Parser::new(tokens);
    parser.parse_program(debug)
}

pub fn parse_with_source(
    tokens: Vec<(Token, Span)>,
    _source: &str,
    debug: bool,
) -> Result<Spanned, PSLError> {
    let mut parser = Parser::new(tokens);
    parser.parse_program(debug)
}
