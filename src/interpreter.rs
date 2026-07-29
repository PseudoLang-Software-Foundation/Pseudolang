use crate::error::{PSLError, Span, StackFrame};
use crate::parser::{AstNode, BinaryOperator, Spanned, UnaryOperator};
use num_bigint::BigInt;
use num_traits::{FromPrimitive, One, Signed, ToPrimitive, Zero};
use rand::RngExt;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasi"))]
use std::thread;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasi"))]
use std::time::Duration;

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum Value {
    Integer(BigInt),
    Float(f64),
    String(String),
    Boolean(bool),
    List(Vec<Value>),
    Unit,
    Null,
    NaN,
}

enum Interruption {
    Return(Value),
    Error(PSLError),
}

type EvalResult = Result<Value, Interruption>;

fn runtime_err(msg: impl Into<String>, span: Span, env: &Rc<RefCell<Environment>>) -> Interruption {
    Interruption::Error(PSLError {
        message: msg.into(),
        span: Some(span),
        stack_trace: env.borrow().get_call_stack(),
    })
}

const MAX_STACK_DEPTH: usize = 1000;
const MAX_LOOP_ITERATIONS: usize = 1_000_000;

#[derive(Clone)]
struct Environment {
    variables: HashMap<String, Value>,
    procedures: HashMap<String, (Vec<String>, Spanned)>,
    output: String,
    parent: Option<Rc<RefCell<Environment>>>,
    call_stack: Rc<RefCell<Vec<StackFrame>>>,
    parsed_flags: Rc<HashMap<String, String>>,
}

impl Environment {
    fn new() -> Self {
        Environment {
            variables: HashMap::new(),  // skipcq: RS-W1079
            procedures: HashMap::new(), // skipcq: RS-W1079
            output: String::new(),      // skipcq: RS-W1079
            parent: None,
            call_stack: Rc::new(RefCell::new(Vec::new())), // skipcq: RS-W1079
            parsed_flags: Rc::new(HashMap::new()),         // skipcq: RS-W1079
        }
    }

    fn new_with_parent(parent: Rc<RefCell<Environment>>) -> Self {
        let procedures = parent.borrow().procedures.clone();
        let call_stack = Rc::clone(&parent.borrow().call_stack);
        let parsed_flags = Rc::clone(&parent.borrow().parsed_flags);
        Environment {
            variables: HashMap::new(), // skipcq: RS-W1079
            procedures,
            output: String::new(), // skipcq: RS-W1079
            parent: Some(Rc::clone(&parent)),
            call_stack,
            parsed_flags,
        }
    }

    fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.variables.get(name) {
            return Some(value.clone());
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow().get(name);
        }
        None
    }

    fn set(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    fn get_procedure(&self, name: &str) -> Option<(Vec<String>, Spanned)> {
        self.procedures.get(name).cloned()
    }

    fn push_frame(&self, frame: StackFrame) {
        self.call_stack.borrow_mut().push(frame);
    }

    fn pop_frame(&self) {
        self.call_stack.borrow_mut().pop();
    }

    fn get_call_stack(&self) -> Vec<StackFrame> {
        self.call_stack.borrow().clone()
    }

    fn stack_depth(&self) -> usize {
        self.call_stack.borrow().len()
    }
}

fn parse_program_args(raw: &[String]) -> (HashMap<String, String>, Vec<String>) {
    let mut flags = HashMap::new();
    let mut positionals = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        let arg = &raw[i];
        if let Some(key) = arg.strip_prefix("--") {
            if i + 1 < raw.len() && !raw[i + 1].starts_with('-') {
                flags.insert(key.to_string(), raw[i + 1].clone());
                i += 2;
            } else {
                flags.insert(key.to_string(), "true".to_string());
                i += 1;
            }
        } else if let Some(key) = arg.strip_prefix('-') {
            if i + 1 < raw.len() && !raw[i + 1].starts_with('-') {
                flags.insert(key.to_string(), raw[i + 1].clone());
                i += 2;
            } else {
                flags.insert(key.to_string(), "true".to_string());
                i += 1;
            }
        } else {
            positionals.push(arg.clone());
            i += 1;
        }
    }
    (flags, positionals)
}

fn init_env_with_args(env: &Rc<RefCell<Environment>>, args: &[String]) {
    let (flags, positionals) = parse_program_args(args);

    let args_list = args.iter().map(|a| Value::String(a.clone())).collect();
    let positionals_list = positionals
        .iter()
        .map(|p| Value::String(p.clone()))
        .collect();

    let mut env_mut = env.borrow_mut();
    env_mut.set("ARGS".to_string(), Value::List(args_list));
    env_mut.set(
        "ARGCOUNT".to_string(),
        Value::Integer(BigInt::from(args.len())),
    );
    env_mut.set("POSITIONALS".to_string(), Value::List(positionals_list));
    env_mut.parsed_flags = Rc::new(flags);
}

#[allow(dead_code)]
pub fn run(ast: Spanned) -> Result<String, String> {
    let env = Rc::new(RefCell::new(Environment::new()));
    init_env_with_args(&env, &[]);
    match evaluate_node(&ast, Rc::clone(&env), false) {
        Ok(_) => Ok(env.borrow().output.clone()),
        Err(Interruption::Return(_)) => Ok(env.borrow().output.clone()),
        Err(Interruption::Error(e)) => Err(e.message),
    }
}

pub fn run_with_source(ast: Spanned, _source: &str, args: &[String]) -> Result<String, PSLError> {
    let env = Rc::new(RefCell::new(Environment::new()));
    init_env_with_args(&env, args);
    match evaluate_node(&ast, Rc::clone(&env), false) {
        Ok(_) => Ok(env.borrow().output.clone()),
        Err(Interruption::Return(_)) => Ok(env.borrow().output.clone()),
        Err(Interruption::Error(e)) => Err(e),
    }
}

fn evaluate_node(node: &Spanned, env: Rc<RefCell<Environment>>, debug: bool) -> EvalResult {
    #[cfg(not(target_arch = "wasm32"))]
    return stacker::maybe_grow(64 * 1024, 2 * 1024 * 1024, || {
        evaluate_node_impl(node, env, debug)
    });

    #[cfg(target_arch = "wasm32")]
    evaluate_node_impl(node, env, debug)
}

// skipcq: RS-R1000
fn evaluate_node_impl(node: &Spanned, env: Rc<RefCell<Environment>>, debug: bool) -> EvalResult {
    let Spanned {
        node: ref ast_node,
        span,
    } = *node;

    if debug {
        println!("Evaluating node: {:?}", ast_node);
    }

    match ast_node {
        AstNode::Program(statements) | AstNode::Block(statements) => {
            let mut last_value = Value::Unit;
            for stmt in statements {
                last_value = evaluate_node(stmt, Rc::clone(&env), debug)?;
            }
            Ok(last_value)
        }

        AstNode::Integer(n) => Ok(Value::Integer(n.clone())),
        AstNode::Float(f) => Ok(Value::Float(*f)),
        AstNode::String(s) => Ok(Value::String(s.clone())),
        AstNode::Boolean(b) => Ok(Value::Boolean(*b)),
        AstNode::Null => Ok(Value::Null),
        AstNode::NaN => Ok(Value::NaN),
        AstNode::RawString(s) => Ok(Value::String(s.clone())),

        AstNode::List(elements) => {
            let mut values = Vec::new();
            for elem in elements {
                values.push(evaluate_node(elem, Rc::clone(&env), debug)?);
            }
            Ok(Value::List(values))
        }

        AstNode::Identifier(name) => match env.borrow().get(name) {
            Some(val) => Ok(val),
            None => Err(runtime_err(
                format!("Undefined variable: {}", name),
                span,
                &env,
            )),
        },

        AstNode::Assignment(target, value) => {
            let val = evaluate_node(value, Rc::clone(&env), debug)?;
            if let AstNode::Identifier(name) = &target.node {
                if debug {
                    println!("Assigning {} = {:?}", name, val);
                }
                if matches!(&value.node, AstNode::FormattedString(_, _)) {
                    let output = value_to_string(&val);
                    env.borrow_mut().output.push_str(&output);
                    env.borrow_mut().output.push('\n');
                }
                env.borrow_mut().set(name.clone(), val.clone());
                Ok(val)
            } else {
                Err(runtime_err("Invalid assignment target", span, &env))
            }
        }

        AstNode::BinaryOp(left_expr, op, right_expr) => match op {
            BinaryOperator::And => {
                let left_val = evaluate_node(left_expr, Rc::clone(&env), debug)?;
                if matches!(left_val, Value::Boolean(false)) {
                    Ok(Value::Boolean(false))
                } else {
                    let right_val = evaluate_node(right_expr, Rc::clone(&env), debug)?;
                    if let Value::Boolean(right_bool) = right_val {
                        Ok(Value::Boolean(right_bool))
                    } else {
                        Err(runtime_err(
                            "Right operand of AND must be boolean",
                            span,
                            &env,
                        ))
                    }
                }
            }
            BinaryOperator::Or => {
                let left_val = evaluate_node(left_expr, Rc::clone(&env), debug)?;
                if matches!(left_val, Value::Boolean(true)) {
                    Ok(Value::Boolean(true))
                } else {
                    let right_val = evaluate_node(right_expr, Rc::clone(&env), debug)?;
                    if let Value::Boolean(right_bool) = right_val {
                        Ok(Value::Boolean(right_bool))
                    } else {
                        Err(runtime_err(
                            "Right operand of OR must be boolean",
                            span,
                            &env,
                        ))
                    }
                }
            }
            _ => {
                let left_val = evaluate_node(left_expr, Rc::clone(&env), debug)?;
                let right_val = evaluate_node(right_expr, Rc::clone(&env), debug)?;
                evaluate_binary_op(&left_val, op, &right_val)
                    .map_err(|msg| runtime_err(msg, span, &env))
            }
        },

        AstNode::UnaryOp(op, expr) => {
            let val = evaluate_node(expr, Rc::clone(&env), debug)?;
            evaluate_unary_op(op, &val).map_err(|msg| runtime_err(msg, span, &env))
        }

        AstNode::If(condition, then_branch, else_branch) => {
            let cond_val = evaluate_node(condition, Rc::clone(&env), debug)?;
            match cond_val {
                Value::Boolean(true) => evaluate_node(then_branch, Rc::clone(&env), debug),
                Value::Boolean(false) => match else_branch {
                    Some(else_branch) => evaluate_node(else_branch, Rc::clone(&env), debug),
                    None => Ok(Value::Unit),
                },
                _ => Err(runtime_err("Condition must be a boolean", span, &env)),
            }
        }

        AstNode::RepeatTimes(count, body) => {
            let count_val = evaluate_node(count, Rc::clone(&env), debug)?;
            if let Value::Integer(n) = count_val {
                let iterations = n
                    .to_i64()
                    .ok_or_else(|| runtime_err("REPEAT count too large", span, &env))?;
                for _ in 0..iterations {
                    evaluate_node(body, Rc::clone(&env), debug)?;
                }
                Ok(Value::Unit)
            } else {
                Err(runtime_err("REPEAT count must be an integer", span, &env))
            }
        }

        AstNode::Display(expr) => match expr {
            Some(expr) => {
                let result = evaluate_node(expr, Rc::clone(&env), debug)?;
                let output = value_to_string(&result);
                if !std::thread::current()
                    .name()
                    .is_some_and(|name| name.starts_with("test"))
                {
                    println!("{}", output);
                    io::stdout().flush().unwrap();
                }
                env.borrow_mut().output.push_str(&output);
                env.borrow_mut().output.push('\n');
                Ok(result)
            }
            None => {
                if !std::thread::current()
                    .name()
                    .is_some_and(|name| name.starts_with("test"))
                {
                    println!();
                }
                env.borrow_mut().output.push('\n');
                Ok(Value::Unit)
            }
        },

        AstNode::DisplayInline(expr) => {
            let value = evaluate_node(expr, Rc::clone(&env), debug)?;
            let output = value_to_string(&value);
            if !std::thread::current()
                .name()
                .is_some_and(|name| name.starts_with("test"))
            {
                print!("{}", output);
                io::stdout().flush().unwrap();
            }
            env.borrow_mut().output.push_str(&output);
            Ok(Value::Unit)
        }

        AstNode::Input(prompt) => {
            #[cfg(any(not(target_arch = "wasm32"), feature = "wasi"))]
            {
                let mut input_str = String::default();

                if let Some(prompt_expr) = prompt {
                    let prompt_val = evaluate_node(prompt_expr, Rc::clone(&env), debug)?;
                    let prompt_str = value_to_string(&prompt_val);
                    print!("{}", prompt_str);
                    io::stdout().flush().unwrap();
                }

                io::stdin()
                    .read_line(&mut input_str)
                    .map_err(|e| runtime_err(e.to_string(), span, &env))?;
                let input = input_str.trim().to_string();

                if prompt.is_none() {
                    env.borrow_mut().output.push_str(&input);
                    env.borrow_mut().output.push('\n');
                }

                Ok(Value::String(input))
            }

            #[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
            {
                let prompt_text = if let Some(prompt_expr) = prompt {
                    let prompt_val = evaluate_node(prompt_expr, Rc::clone(&env), debug)?;
                    value_to_string(&prompt_val)
                } else {
                    "Input:".to_string()
                };

                let input = crate::interpreter::prompt(&prompt_text);

                if prompt.is_none() {
                    env.borrow_mut().output.push_str(&input);
                    env.borrow_mut().output.push('\n');
                }

                Ok(Value::String(input))
            }
        }

        AstNode::ProcedureDecl(name, params, body) => {
            env.borrow_mut()
                .procedures
                .insert(name.clone(), (params.clone(), (**body).clone()));
            Ok(Value::Unit)
        }

        AstNode::ProcedureCall(name, args) => {
            if let Some(result) = eval_builtin(name, args, &env, span, debug) {
                return result;
            }
            if env.borrow().stack_depth() >= MAX_STACK_DEPTH {
                return Err(runtime_err(
                    "Stack overflow: maximum recursion depth exceeded",
                    span,
                    &env,
                ));
            }
            let procedure = env.borrow().get_procedure(name).ok_or_else(|| {
                runtime_err(format!("Procedure '{}' not found", name), span, &env)
            })?;
            let local_env = Rc::new(RefCell::new(Environment::new_with_parent(Rc::clone(&env))));
            let (params, ref body) = procedure;
            for (param, arg) in params.iter().zip(args) {
                let arg_value = evaluate_node(arg, Rc::clone(&env), debug)?;
                local_env.borrow_mut().set(param.clone(), arg_value);
            }
            env.borrow().push_frame(StackFrame {
                name: name.clone(),
                span,
            });
            let body_result = evaluate_node(body, Rc::clone(&local_env), debug);
            env.borrow().pop_frame();
            env.borrow_mut().output.push_str(&local_env.borrow().output);
            match body_result {
                Err(Interruption::Return(val)) => Ok(val),
                other => other,
            }
        }

        AstNode::ListAccess(list, index) => {
            let current_value = evaluate_node(list, Rc::clone(&env), debug)?;
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;

            match (current_value, index_val) {
                (Value::List(elements), Value::Integer(i)) => {
                    let idx = &i - BigInt::one();
                    if idx.is_negative() {
                        Err(runtime_err(
                            "List index out of bounds: index cannot be less than 1",
                            span,
                            &env,
                        ))
                    } else {
                        let uidx = idx
                            .to_usize()
                            .ok_or_else(|| runtime_err("List index too large", span, &env))?;
                        if uidx >= elements.len() {
                            Err(runtime_err(
                                format!(
                                    "List index out of bounds: {} (size: {})",
                                    i,
                                    elements.len()
                                ),
                                span,
                                &env,
                            ))
                        } else {
                            Ok(elements[uidx].clone())
                        }
                    }
                }
                (Value::String(s), Value::Integer(i)) => {
                    let idx = &i - BigInt::one();
                    if idx.is_negative() {
                        Err(runtime_err(
                            "String index out of bounds: index cannot be less than 1",
                            span,
                            &env,
                        ))
                    } else {
                        let uidx = idx
                            .to_usize()
                            .ok_or_else(|| runtime_err("String index too large", span, &env))?;
                        if uidx >= s.len() {
                            Err(runtime_err(
                                format!("String index out of bounds: {} (size: {})", i, s.len()),
                                span,
                                &env,
                            ))
                        } else {
                            let ch = s
                                .chars()
                                .nth(uidx)
                                .ok_or_else(|| runtime_err("Invalid string index", span, &env))?;
                            Ok(Value::String(ch.to_string()))
                        }
                    }
                }
                _ => Err(runtime_err(
                    "Invalid index access - expected list or string and integer index",
                    span,
                    &env,
                )),
            }
        }

        AstNode::ListAssignment(list, index, value) => {
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;
            let new_val = evaluate_node(value, Rc::clone(&env), debug)?;

            if let AstNode::Identifier(name) = &list.node {
                let mut elements = if let Some(Value::List(elements)) = env.borrow().get(name) {
                    elements
                } else {
                    return Err(runtime_err(
                        format!("Variable {} is not a list", name),
                        span,
                        &env,
                    ));
                };

                if let Value::Integer(i) = index_val {
                    let idx = &i - BigInt::one();
                    match idx.to_usize() {
                        Some(uidx) if uidx < elements.len() => {
                            let ret = new_val.clone();
                            elements[uidx] = new_val;
                            env.borrow_mut().set(name.clone(), Value::List(elements));
                            Ok(ret)
                        }
                        _ => Err(runtime_err("List index out of bounds", span, &env)),
                    }
                } else {
                    Err(runtime_err("Invalid list index", span, &env))
                }
            } else if let AstNode::ListAccess(inner_list, inner_index) = &list.node {
                let list_val = evaluate_node(inner_list, Rc::clone(&env), debug)?;
                let index_inner = evaluate_node(inner_index, Rc::clone(&env), debug)?;

                if let (Value::List(mut elements), Value::Integer(i)) = (list_val, index_inner) {
                    let idx = &i - BigInt::one();
                    if let Some(uidx) = idx.to_usize()
                        && uidx < elements.len()
                        && let Value::Integer(j) = index_val
                    {
                        let jdx = &j - BigInt::one();
                        if let Some(ujdx) = jdx.to_usize()
                            && let Value::List(mut inner_elements) =
                                std::mem::replace(&mut elements[uidx], Value::Unit)
                            && ujdx < inner_elements.len()
                        {
                            let ret = new_val.clone();
                            inner_elements[ujdx] = new_val;
                            elements[uidx] = Value::List(inner_elements);

                            if let AstNode::Identifier(name) = &inner_list.node {
                                env.borrow_mut().set(name.clone(), Value::List(elements));
                                return Ok(ret);
                            }
                        }
                    }
                }
                Err(runtime_err("Invalid nested list assignment", span, &env))
            } else {
                Err(runtime_err("Invalid list assignment target", span, &env))
            }
        }

        AstNode::Substring(string, start, end) => {
            let str_val = evaluate_node(string, Rc::clone(&env), debug)?;
            let start_val = evaluate_node(start, Rc::clone(&env), debug)?;
            let end_val = evaluate_node(end, Rc::clone(&env), debug)?;

            if let (Value::String(s), Value::Integer(start), Value::Integer(end)) =
                (str_val, start_val, end_val)
            {
                let start_idx = &start - BigInt::one();
                let end_idx = &end - BigInt::one();
                match (start_idx.to_usize(), end_idx.to_usize()) {
                    (Some(si), Some(ei))
                        if !start_idx.is_negative() && end_idx >= start_idx && ei <= s.len() =>
                    {
                        Ok(Value::String(s[si..=ei].to_string()))
                    }
                    _ => Err(runtime_err("Invalid substring indices", span, &env)),
                }
            } else {
                Err(runtime_err("Invalid substring arguments", span, &env))
            }
        }

        AstNode::Concat(str1, str2) => {
            let s1 = evaluate_node(str1, Rc::clone(&env), debug)?;
            let s2 = evaluate_node(str2, Rc::clone(&env), debug)?;
            if let (Value::String(s1), Value::String(s2)) = (s1, s2) {
                Ok(Value::String(format!("{}{}", s1, s2)))
            } else {
                Err(runtime_err("CONCAT requires string arguments", span, &env))
            }
        }

        AstNode::ToString(expr) => {
            let val = evaluate_node(expr, Rc::clone(&env), debug)?;
            Ok(Value::String(value_to_string(&val)))
        }

        AstNode::ToNum(expr) => {
            let val = evaluate_node(expr, Rc::clone(&env), debug)?;
            if let Value::String(s) = val {
                if let Ok(n) = s.parse::<BigInt>() {
                    Ok(Value::Integer(n))
                } else if let Ok(f) = s.parse::<f64>() {
                    Ok(Value::Float(f))
                } else {
                    Err(runtime_err("Cannot convert string to number", span, &env))
                }
            } else {
                Err(runtime_err("TONUM requires string argument", span, &env))
            }
        }

        AstNode::RepeatUntil(body, condition) => {
            let mut iterations = 0;

            loop {
                iterations += 1;
                if iterations > MAX_LOOP_ITERATIONS {
                    return Err(runtime_err("Maximum loop iterations exceeded", span, &env));
                }

                evaluate_node(body, Rc::clone(&env), debug)?;

                let cond_val = evaluate_node(condition, Rc::clone(&env), debug)?;
                match cond_val {
                    Value::Boolean(true) => break,
                    Value::Boolean(false) => continue,
                    _ => {
                        return Err(runtime_err(
                            "REPEAT UNTIL condition must evaluate to boolean",
                            span,
                            &env,
                        ));
                    }
                }
            }
            Ok(Value::Unit)
        }

        AstNode::ForEach(var_name, list, body) => {
            let list_val = evaluate_node(list, Rc::clone(&env), debug)?;
            match list_val {
                Value::List(elements) => {
                    let mut result = Value::Unit;
                    for element in elements {
                        env.borrow_mut().set(var_name.clone(), element);
                        result = evaluate_node(body, Rc::clone(&env), debug)?;
                    }
                    Ok(result)
                }
                Value::String(s) => {
                    let mut result = Value::Unit;
                    for c in s.chars() {
                        env.borrow_mut()
                            .set(var_name.clone(), Value::String(c.to_string()));
                        result = evaluate_node(body, Rc::clone(&env), debug)?;
                    }
                    Ok(result)
                }
                _ => Err(runtime_err("FOR EACH requires list or string", span, &env)),
            }
        }

        AstNode::FormattedString(s, expressions) => {
            let mut result = String::with_capacity(s.len());
            let mut placeholders = s.split("{}");
            let mut expr_iter = expressions.iter();

            if let Some(first_part) = placeholders.next() {
                result.push_str(first_part);
            }

            for part in placeholders {
                if let Some(expr) = expr_iter.next() {
                    let value = evaluate_node(expr, Rc::clone(&env), debug)?;
                    result.push_str(&value_to_string(&value));
                }
                result.push_str(part);
            }

            Ok(Value::String(result))
        }

        AstNode::Length(list) => {
            let list_val = evaluate_node(list, Rc::clone(&env), debug)?;
            match list_val {
                Value::List(elements) => Ok(Value::Integer(BigInt::from(elements.len()))),
                Value::String(s) => Ok(Value::Integer(BigInt::from(s.len()))),
                _ => Err(runtime_err(
                    "LENGTH requires a list or string argument",
                    span,
                    &env,
                )),
            }
        }

        AstNode::ListInsert(list, index, value) | AstNode::Insert(list, index, value) => {
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;
            let insert_val = evaluate_node(value, Rc::clone(&env), debug)?;

            if let AstNode::Identifier(name) = &list.node {
                let elements = if let Some(Value::List(elements)) = env.borrow().get(name) {
                    elements
                } else {
                    return Err(runtime_err(
                        format!("Variable {} is not a list", name),
                        span,
                        &env,
                    ));
                };

                if let Value::Integer(i) = index_val {
                    let idx = &i - BigInt::one();
                    match idx.to_usize() {
                        Some(uidx) if !idx.is_negative() && uidx <= elements.len() => {
                            let mut new_elements = elements.clone();
                            new_elements.insert(uidx, insert_val.clone());
                            env.borrow_mut()
                                .set(name.clone(), Value::List(new_elements));
                            Ok(insert_val)
                        }
                        _ => Err(runtime_err("List index out of bounds", span, &env)),
                    }
                } else {
                    Err(runtime_err("Invalid list index", span, &env))
                }
            } else {
                Err(runtime_err("INSERT requires a list variable", span, &env))
            }
        }

        AstNode::ListAppend(list, value) | AstNode::Append(list, value) => {
            let append_val = evaluate_node(value, Rc::clone(&env), debug)?;

            if let AstNode::Identifier(name) = &list.node {
                let elements = if let Some(Value::List(elements)) = env.borrow().get(name) {
                    elements
                } else {
                    return Err(runtime_err(
                        format!("Variable {} is not a list", name),
                        span,
                        &env,
                    ));
                };

                let mut new_elements = elements.clone();
                new_elements.push(append_val.clone());
                env.borrow_mut()
                    .set(name.clone(), Value::List(new_elements));
                Ok(append_val)
            } else {
                Err(runtime_err("APPEND requires a list variable", span, &env))
            }
        }

        AstNode::ListRemove(list, index) | AstNode::Remove(list, index) => {
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;

            if let AstNode::Identifier(name) = &list.node {
                let elements = if let Some(Value::List(elements)) = env.borrow().get(name) {
                    elements
                } else {
                    return Err(runtime_err(
                        format!("Variable {} is not a list", name),
                        span,
                        &env,
                    ));
                };

                if let Value::Integer(i) = index_val {
                    let idx = &i - BigInt::one();
                    match idx.to_usize() {
                        Some(uidx) if !idx.is_negative() && uidx < elements.len() => {
                            let mut new_elements = elements.clone();
                            let removed_value = new_elements.remove(uidx);
                            env.borrow_mut()
                                .set(name.clone(), Value::List(new_elements));
                            Ok(removed_value)
                        }
                        _ => Err(runtime_err("List index out of bounds", span, &env)),
                    }
                } else {
                    Err(runtime_err("REMOVE requires an integer index", span, &env))
                }
            } else {
                Err(runtime_err("REMOVE requires a list variable", span, &env))
            }
        }

        AstNode::Random(min, max) => {
            let min_val = evaluate_node(min, Rc::clone(&env), debug)?;
            let max_val = evaluate_node(max, Rc::clone(&env), debug)?;

            match (min_val, max_val) {
                (Value::Integer(min_int), Value::Integer(max_int)) => {
                    if min_int > max_int {
                        return Err(runtime_err(
                            "Min value must be less than or equal to max value",
                            span,
                            &env,
                        ));
                    }
                    let min_i64 = min_int
                        .to_i64()
                        .ok_or_else(|| runtime_err("RANDOM bounds too large", span, &env))?;
                    let max_i64 = max_int
                        .to_i64()
                        .ok_or_else(|| runtime_err("RANDOM bounds too large", span, &env))?;
                    let mut rng = rand::rng();
                    Ok(Value::Integer(BigInt::from(
                        rng.random_range(min_i64..=max_i64),
                    )))
                }
                _ => Err(runtime_err("RANDOM requires integer arguments", span, &env)),
            }
        }

        AstNode::ClassDecl(name, _body) => Err(runtime_err(
            format!(
                "CLASS '{}' is not yet implemented. Class declarations are parsed but instantiation, method dispatch, and field access are not supported.",
                name
            ),
            span,
            &env,
        )),

        AstNode::Import(path) => {
            let content = std::fs::read_to_string(path).map_err(|e| {
                runtime_err(
                    format!("Failed to read import file {}: {}", path, e),
                    span,
                    &env,
                )
            })?;

            let mut lexer = crate::lexer::Lexer::new(&content);
            let tokens = lexer.tokenize();

            let imported_ast = crate::parser::parse(tokens, false).map_err(|e| {
                runtime_err(
                    format!("Failed to parse import file {}: {}", path, e),
                    span,
                    &env,
                )
            })?;

            evaluate_node(&imported_ast, Rc::clone(&env), debug)?;

            Ok(Value::Unit)
        }

        AstNode::Return(expr) => {
            let value = evaluate_node(expr, Rc::clone(&env), debug)?;
            Err(Interruption::Return(value))
        }

        AstNode::Sort(list_expr) => {
            let list_val = evaluate_node(list_expr, Rc::clone(&env), debug)?;
            if let Value::List(mut elements) = list_val {
                elements.sort_by(|a, b| match (a, b) {
                    (Value::Integer(a_int), Value::Integer(b_int)) => a_int.cmp(b_int),
                    (Value::Float(a_float), Value::Float(b_float)) => a_float
                        .partial_cmp(b_float)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    (Value::Integer(a_int), Value::Float(b_float)) => bigint_to_f64(a_int)
                        .partial_cmp(b_float)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    (Value::Float(a_float), Value::Integer(b_int)) => a_float
                        .partial_cmp(&bigint_to_f64(b_int))
                        .unwrap_or(std::cmp::Ordering::Equal),
                    (Value::String(a_str), Value::String(b_str)) => a_str.cmp(b_str),
                    _ => std::cmp::Ordering::Equal,
                });
                Ok(Value::List(elements))
            } else {
                Err(runtime_err(
                    "SORT requires a list as an argument",
                    span,
                    &env,
                ))
            }
        }

        AstNode::TryCatch {
            try_block,
            error_var,
            catch_block,
        } => match evaluate_node(try_block, Rc::clone(&env), debug) {
            Ok(result) => Ok(result),
            Err(Interruption::Return(val)) => Err(Interruption::Return(val)),
            Err(Interruption::Error(error)) => {
                let catch_env =
                    Rc::new(RefCell::new(Environment::new_with_parent(Rc::clone(&env))));
                if let Some(var_name) = error_var {
                    catch_env
                        .borrow_mut()
                        .set(var_name.clone(), Value::String(error.message));
                }
                let result = evaluate_node(catch_block, Rc::clone(&catch_env), debug)?;
                env.borrow_mut().output.push_str(&catch_env.borrow().output);
                Ok(result)
            }
        },

        AstNode::Eval(expr) => {
            let expr_val = evaluate_node(expr, Rc::clone(&env), debug)?;
            if let Value::String(s) = expr_val {
                let mut lexer = crate::lexer::Lexer::new(&s);
                let tokens = lexer.tokenize();
                let mut parser = crate::parser::Parser::new(tokens);
                let ast = parser.parse_expression(debug).map_err(|mut e| {
                    e.span = Some(span);
                    Interruption::Error(e)
                })?;

                evaluate_node(&ast, Rc::clone(&env), debug)
            } else {
                Err(runtime_err("EVAL requires a string argument", span, &env))
            }
        }

        AstNode::Comment(_) => Ok(Value::Unit),
    }
}

// skipcq: RS-R1000
fn eval_builtin(
    name: &str,
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> Option<EvalResult> {
    match name {
        "SLEEP" => Some(eval_builtin_sleep(args, env, span, debug)),
        "CONCAT" => Some(eval_builtin_concat(args, env, span, debug)),
        "SUBSTRING" => Some(eval_builtin_substring(args, env, span, debug)),
        "LENGTH" => Some(eval_builtin_length(args, env, span, debug)),
        "REMOVE" => Some(eval_builtin_remove(args, env, span, debug)),
        "APPEND" => Some(eval_builtin_append(args, env, span, debug)),
        "INSERT" => Some(eval_builtin_insert(args, env, span, debug)),
        "ABS" => Some(eval_builtin_abs(args, env, span, debug)),
        "CEIL" => Some(eval_builtin_ceil(args, env, span, debug)),
        "FLOOR" => Some(eval_builtin_floor(args, env, span, debug)),
        "POW" => Some(eval_builtin_pow(args, env, span, debug)),
        "SQRT" => Some(eval_builtin_sqrt(args, env, span, debug)),
        "SIN" => Some(eval_single_num_fn(args, env, span, debug, "SIN", f64::sin)),
        "COS" => Some(eval_single_num_fn(args, env, span, debug, "COS", f64::cos)),
        "TAN" => Some(eval_single_num_fn(args, env, span, debug, "TAN", f64::tan)),
        "ASIN" => Some(eval_single_num_fn(
            args,
            env,
            span,
            debug,
            "ASIN",
            f64::asin,
        )),
        "ACOS" => Some(eval_single_num_fn(
            args,
            env,
            span,
            debug,
            "ACOS",
            f64::acos,
        )),
        "ATAN" => Some(eval_single_num_fn(
            args,
            env,
            span,
            debug,
            "ATAN",
            f64::atan,
        )),
        "EXP" => Some(eval_single_num_fn(args, env, span, debug, "EXP", f64::exp)),
        "LOG" | "NLOG" => Some(eval_single_num_fn(args, env, span, debug, "LOG", f64::ln)),
        "LOGTEN" => Some(eval_single_num_fn(
            args,
            env,
            span,
            debug,
            "LOGTEN",
            f64::log10,
        )),
        "LOGTWO" => Some(eval_single_num_fn(
            args,
            env,
            span,
            debug,
            "LOGTWO",
            f64::log2,
        )),
        "DEGREES" => Some(eval_single_num_fn(
            args,
            env,
            span,
            debug,
            "DEGREES",
            f64::to_degrees,
        )),
        "RADIANS" => Some(eval_single_num_fn(
            args,
            env,
            span,
            debug,
            "RADIANS",
            f64::to_radians,
        )),
        "GCD" => Some(eval_builtin_gcd(args, env, span, debug)),
        "FACTORIAL" => Some(eval_builtin_factorial(args, env, span, debug)),
        "HYPOT" => Some(eval_builtin_hypot(args, env, span, debug)),
        "MIN" => Some(eval_builtin_min(args, env, span, debug)),
        "MAX" => Some(eval_builtin_max(args, env, span, debug)),
        "EXIT" => std::process::exit(0),
        "ROUND" => Some(eval_builtin_round(args, env, span, debug)),
        "SPLIT" => Some(eval_builtin_split(args, env, span, debug)),
        "TRIM" => Some(eval_builtin_trim(args, env, span, debug)),
        "REPLACE" => Some(eval_builtin_replace(args, env, span, debug)),
        "UPPERCASE" => Some(eval_builtin_uppercase(args, env, span, debug)),
        "LOWERCASE" => Some(eval_builtin_lowercase(args, env, span, debug)),
        "TIMESTAMP" => Some(eval_builtin_timestamp(args, env, span, debug)),
        "TIME" => Some(eval_builtin_time(args, env, span, debug)),
        "TIMEZONE" => Some(eval_builtin_timezone(args, env, span, debug)),
        "TIMEZONES" => Some(eval_builtin_timezones(args, env, span)),
        "MILLITIME" => Some(eval_builtin_millitime(args, env, span)),
        "CONTAINS" => Some(eval_builtin_contains(args, env, span, debug)),
        "FIND" => Some(eval_builtin_find(args, env, span, debug)),
        "RANGE" => Some(eval_builtin_range(args, env, span, debug)),
        "STARTSWITH" => Some(eval_builtin_startswith(args, env, span, debug)),
        "ENDSWITH" => Some(eval_builtin_endswith(args, env, span, debug)),
        "HASARG" => Some(eval_builtin_hasarg(args, env, span, debug)),
        "GETARG" => Some(eval_builtin_getarg(args, env, span, debug)),
        _ => None,
    }
}

fn eval_builtin_sleep(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("SLEEP requires one argument", span, env));
    }
    io::stdout().flush().unwrap();
    #[cfg(any(not(target_arch = "wasm32"), feature = "wasi"))]
    {
        let seconds = evaluate_node(&args[0], Rc::clone(env), debug)?;
        match seconds {
            Value::Integer(n) => {
                let secs = n.to_u64().unwrap_or(0);
                thread::sleep(Duration::from_secs(secs));
                Ok(Value::Unit)
            }
            Value::Float(f) => {
                thread::sleep(Duration::from_secs_f64(f));
                Ok(Value::Unit)
            }
            _ => Err(runtime_err("SLEEP requires a numeric argument", span, env)),
        }
    }
    #[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
    {
        let _seconds = evaluate_node(&args[0], Rc::clone(env), debug)?;
        log(
            "SLEEP function is not fully supported in WebAssembly. The program will continue without pausing.",
        );
        return Ok(Value::Unit);
    }
}

fn eval_builtin_concat(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("CONCAT requires two arguments", span, env));
    }
    let s1 = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let s2 = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match (s1, s2) {
        (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
        _ => Err(runtime_err("CONCAT requires string arguments", span, env)),
    }
}

fn eval_builtin_substring(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 3 {
        return Err(runtime_err("SUBSTRING requires three arguments", span, env));
    }
    let str_val = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let start_val = evaluate_node(&args[1], Rc::clone(env), debug)?;
    let end_val = evaluate_node(&args[2], Rc::clone(env), debug)?;
    if let (Value::String(s), Value::Integer(start), Value::Integer(end)) =
        (str_val, start_val, end_val)
    {
        let start_idx = &start - BigInt::one();
        let end_idx = &end - BigInt::one();
        match (start_idx.to_usize(), end_idx.to_usize()) {
            (Some(si), Some(ei))
                if !start_idx.is_negative() && end_idx >= start_idx && ei < s.len() =>
            {
                Ok(Value::String(s[si..=ei].to_string()))
            }
            _ => Err(runtime_err("Invalid substring indices", span, env)),
        }
    } else {
        Err(runtime_err("Invalid substring arguments", span, env))
    }
}

fn eval_builtin_length(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("LENGTH requires one argument", span, env));
    }
    let arg = evaluate_node(&args[0], Rc::clone(env), debug)?;
    match arg {
        Value::List(elements) => Ok(Value::Integer(BigInt::from(elements.len()))),
        Value::String(s) => Ok(Value::Integer(BigInt::from(s.len()))),
        _ => Err(runtime_err(
            "LENGTH requires a list or string argument",
            span,
            env,
        )),
    }
}

fn eval_builtin_remove(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("REMOVE requires two arguments", span, env));
    }
    let synth = Spanned::new(
        AstNode::Remove(Box::new(args[0].clone()), Box::new(args[1].clone())),
        span,
    );
    evaluate_node(&synth, Rc::clone(env), debug)
}

fn eval_builtin_append(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("APPEND requires two arguments", span, env));
    }
    let synth = Spanned::new(
        AstNode::Append(Box::new(args[0].clone()), Box::new(args[1].clone())),
        span,
    );
    evaluate_node(&synth, Rc::clone(env), debug)
}

fn eval_builtin_insert(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 3 {
        return Err(runtime_err("INSERT requires three arguments", span, env));
    }
    let synth = Spanned::new(
        AstNode::Insert(
            Box::new(args[0].clone()),
            Box::new(args[1].clone()),
            Box::new(args[2].clone()),
        ),
        span,
    );
    evaluate_node(&synth, Rc::clone(env), debug)
}

fn eval_builtin_abs(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("ABS requires one argument", span, env));
    }
    let x = evaluate_node(&args[0], Rc::clone(env), debug)?;
    match x {
        Value::Integer(n) => Ok(Value::Integer(n.abs())),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        _ => Err(runtime_err("ABS requires a numeric argument", span, env)),
    }
}

fn eval_builtin_ceil(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("CEIL requires one argument", span, env));
    }
    let x = evaluate_node(&args[0], Rc::clone(env), debug)?;
    match x {
        Value::Float(f) => BigInt::from_f64(f.ceil())
            .map(Value::Integer)
            .ok_or_else(|| runtime_err("CEIL: result is not a finite number", span, env)),
        Value::Integer(n) => Ok(Value::Integer(n)),
        _ => Err(runtime_err("CEIL requires a numeric argument", span, env)),
    }
}

fn eval_builtin_floor(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("FLOOR requires one argument", span, env));
    }
    let x = evaluate_node(&args[0], Rc::clone(env), debug)?;
    match x {
        Value::Float(f) => BigInt::from_f64(f.floor())
            .map(Value::Integer)
            .ok_or_else(|| runtime_err("FLOOR: result is not a finite number", span, env)),
        Value::Integer(n) => Ok(Value::Integer(n)),
        _ => Err(runtime_err("FLOOR requires a numeric argument", span, env)),
    }
}

fn eval_builtin_pow(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("POW requires two arguments", span, env));
    }
    let base = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let exponent = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match (base, exponent) {
        (Value::Integer(a), Value::Integer(b)) => match b.to_u32() {
            Some(exp) => Ok(Value::Integer(a.pow(exp))),
            None => Ok(Value::Float(bigint_to_f64(&a).powf(bigint_to_f64(&b)))),
        },
        (Value::Float(a), Value::Integer(b)) => match b.to_i32() {
            Some(exp) => Ok(Value::Float(a.powi(exp))),
            None => Ok(Value::Float(a.powf(bigint_to_f64(&b)))),
        },
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(b))),
        (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(bigint_to_f64(&a).powf(b))),
        _ => Err(runtime_err("POW requires numeric arguments", span, env)),
    }
}

fn eval_builtin_sqrt(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("SQRT requires one argument", span, env));
    }
    let x = evaluate_node(&args[0], Rc::clone(env), debug)?;
    match x {
        Value::Integer(n) => Ok(Value::Float(bigint_to_f64(&n).sqrt())),
        Value::Float(f) => Ok(Value::Float(f.sqrt())),
        _ => Err(runtime_err("SQRT requires a numeric argument", span, env)),
    }
}

fn eval_builtin_gcd(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("GCD requires two arguments", span, env));
    }
    let a = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let b = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match (a, b) {
        (Value::Integer(m), Value::Integer(n)) => Ok(Value::Integer(bigint_gcd(&m, &n))),
        _ => Err(runtime_err("GCD requires integer arguments", span, env)),
    }
}

fn eval_builtin_factorial(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("FACTORIAL requires one argument", span, env));
    }
    let x = evaluate_node(&args[0], Rc::clone(env), debug)?;
    match x {
        Value::Integer(n) => Ok(Value::Integer(bigint_factorial(&n))),
        _ => Err(runtime_err(
            "FACTORIAL requires an integer argument",
            span,
            env,
        )),
    }
}

fn eval_builtin_hypot(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("HYPOT requires two arguments", span, env));
    }
    let a = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let b = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.hypot(y))),
        (Value::Integer(x), Value::Float(y)) => Ok(Value::Float(bigint_to_f64(&x).hypot(y))),
        (Value::Float(x), Value::Integer(y)) => Ok(Value::Float(x.hypot(bigint_to_f64(&y)))),
        (Value::Integer(x), Value::Integer(y)) => {
            Ok(Value::Float(bigint_to_f64(&x).hypot(bigint_to_f64(&y))))
        }
        _ => Err(runtime_err("HYPOT requires numeric arguments", span, env)),
    }
}

fn eval_builtin_min(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("MIN requires two arguments", span, env));
    }
    let a = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let b = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(if x <= y { x } else { y })),
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.min(y))),
        (Value::Integer(x), Value::Float(y)) => Ok(Value::Float(bigint_to_f64(&x).min(y))),
        (Value::Float(x), Value::Integer(y)) => Ok(Value::Float(x.min(bigint_to_f64(&y)))),
        _ => Err(runtime_err("MIN requires two numeric arguments", span, env)),
    }
}

fn eval_builtin_max(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("MAX requires two arguments", span, env));
    }
    let a = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let b = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(if x >= y { x } else { y })),
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.max(y))),
        (Value::Integer(x), Value::Float(y)) => Ok(Value::Float(bigint_to_f64(&x).max(y))),
        (Value::Float(x), Value::Integer(y)) => Ok(Value::Float(x.max(bigint_to_f64(&y)))),
        _ => Err(runtime_err("MAX requires two numeric arguments", span, env)),
    }
}

fn eval_builtin_round(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("ROUND requires one argument", span, env));
    }
    let x = evaluate_node(&args[0], Rc::clone(env), debug)?;
    match x {
        Value::Float(f) => BigInt::from_f64(f.round())
            .map(Value::Integer)
            .ok_or_else(|| runtime_err("ROUND: result is not a finite number", span, env)),
        Value::Integer(n) => Ok(Value::Integer(n)),
        _ => Err(runtime_err("ROUND requires a numeric argument", span, env)),
    }
}

fn eval_builtin_split(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("SPLIT requires two arguments", span, env));
    }
    let string_val = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let delimiter_val = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match (string_val, delimiter_val) {
        (Value::String(s), Value::String(d)) => {
            let parts: Vec<Value> = s
                .split(&d)
                .map(|part| Value::String(part.to_string()))
                .collect();
            Ok(Value::List(parts))
        }
        _ => Err(runtime_err(
            "SPLIT requires two string arguments",
            span,
            env,
        )),
    }
}

fn eval_builtin_trim(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("TRIM requires one argument", span, env));
    }
    let str_val = evaluate_node(&args[0], Rc::clone(env), debug)?;
    match str_val {
        Value::String(s) => Ok(Value::String(s.trim().to_string())),
        _ => Err(runtime_err("TRIM requires a string argument", span, env)),
    }
}

fn eval_builtin_replace(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 3 {
        return Err(runtime_err("REPLACE requires three arguments", span, env));
    }
    let str_val = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let from_val = evaluate_node(&args[1], Rc::clone(env), debug)?;
    let to_val = evaluate_node(&args[2], Rc::clone(env), debug)?;
    match (str_val, from_val, to_val) {
        (Value::String(s), Value::String(from), Value::String(to)) => {
            Ok(Value::String(s.replace(&from, &to)))
        }
        _ => Err(runtime_err(
            "REPLACE requires three string arguments",
            span,
            env,
        )),
    }
}

fn eval_builtin_uppercase(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("UPPERCASE requires one argument", span, env));
    }
    let str_val = evaluate_node(&args[0], Rc::clone(env), debug)?;
    match str_val {
        Value::String(s) => Ok(Value::String(s.to_uppercase())),
        _ => Err(runtime_err(
            "UPPERCASE requires a string argument",
            span,
            env,
        )),
    }
}

fn eval_builtin_lowercase(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("LOWERCASE requires one argument", span, env));
    }
    let str_val = evaluate_node(&args[0], Rc::clone(env), debug)?;
    match str_val {
        Value::String(s) => Ok(Value::String(s.to_lowercase())),
        _ => Err(runtime_err(
            "LOWERCASE requires a string argument",
            span,
            env,
        )),
    }
}

fn eval_builtin_timestamp(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    match args.len() {
        0 => {
            #[cfg(any(not(target_arch = "wasm32"), feature = "wasi"))]
            {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| runtime_err(e.to_string(), span, env))?;
                let secs = now.as_secs() as f64;
                let nanos = now.subsec_nanos() as f64 / 1_000_000_000.0;
                Ok(Value::Float(secs + nanos))
            }
            #[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
            {
                let unix_ms = date_now();
                let perf_time = get_high_precision_time();
                let fract_ms = perf_time % 1.0;
                let nanos = fract_ms * 1_000_000.0;
                let seconds = unix_ms / 1000.0;
                let seconds_int = seconds.floor();
                let millis_part = seconds - seconds_int;
                let timestamp = seconds_int + millis_part + (nanos / 1_000_000_000.0);
                return Ok(Value::Float(timestamp));
            }
        }
        1 => {
            #[cfg(any(not(target_arch = "wasm32"), feature = "wasi"))]
            {
                let datetime = evaluate_node(&args[0], Rc::clone(env), debug)?;
                if let Value::String(dt) = datetime {
                    use chrono::NaiveDateTime;
                    match NaiveDateTime::parse_from_str(&dt, "%Y-%m-%d %H:%M:%S%.f") {
                        Ok(dt) => {
                            let timestamp = dt.and_utc().timestamp() as f64;
                            let nanos =
                                dt.and_utc().timestamp_subsec_nanos() as f64 / 1_000_000_000.0;
                            Ok(Value::Float(timestamp + nanos))
                        }
                        Err(e) => Err(runtime_err(
                            format!("Invalid datetime format: {}", e),
                            span,
                            env,
                        )),
                    }
                } else {
                    Err(runtime_err(
                        "TIMESTAMP requires a datetime string",
                        span,
                        env,
                    ))
                }
            }
            #[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
            {
                let timestamp = evaluate_node(&args[0], Rc::clone(env), debug)?;
                return match timestamp {
                    Value::Integer(ts) => {
                        let js_timestamp = JsValue::from_f64(bigint_to_f64(&ts) * 1000.0);
                        let date = js_sys::Date::new(&js_timestamp);
                        Ok(Value::String(format!(
                            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                            date.get_utc_full_year(),
                            date.get_utc_month() + 1,
                            date.get_utc_date(),
                            date.get_utc_hours(),
                            date.get_utc_minutes(),
                            date.get_utc_seconds()
                        )))
                    }
                    Value::Float(ts) => {
                        let js_timestamp = JsValue::from_f64(ts * 1000.0);
                        let date = js_sys::Date::new(&js_timestamp);
                        Ok(Value::String(format!(
                            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                            date.get_utc_full_year(),
                            date.get_utc_month() + 1,
                            date.get_utc_date(),
                            date.get_utc_hours(),
                            date.get_utc_minutes(),
                            date.get_utc_seconds()
                        )))
                    }
                    _ => Err(runtime_err("TIME requires a numeric timestamp", span, env)),
                };
            }
        }
        _ => Err(runtime_err(
            "TIMESTAMP requires 0 or 1 arguments",
            span,
            env,
        )),
    }
}

fn eval_builtin_time(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("TIME requires one argument", span, env));
    }
    #[cfg(any(not(target_arch = "wasm32"), feature = "wasi"))]
    {
        let timestamp = evaluate_node(&args[0], Rc::clone(env), debug)?;
        match timestamp {
            Value::Integer(ts) => {
                use chrono::{TimeZone, Utc};
                let ts_i64 = ts
                    .to_i64()
                    .ok_or_else(|| runtime_err("Timestamp value too large", span, env))?;
                let dt = Utc
                    .timestamp_opt(ts_i64, 0)
                    .single()
                    .ok_or_else(|| runtime_err("Invalid timestamp", span, env))?;
                Ok(Value::String(dt.naive_local().to_string()))
            }
            Value::Float(ts) => {
                use chrono::{TimeZone, Utc};
                let secs = ts.floor() as i64;
                let nanos = ((ts - ts.floor()) * 1_000_000_000.0) as u32;
                let dt = Utc
                    .timestamp_opt(secs, nanos)
                    .single()
                    .ok_or_else(|| runtime_err("Invalid timestamp", span, env))?;
                Ok(Value::String(dt.naive_local().to_string()))
            }
            _ => Err(runtime_err("TIME requires a numeric timestamp", span, env)),
        }
    }
    #[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
    {
        let timestamp = evaluate_node(&args[0], Rc::clone(env), debug)?;
        return match timestamp {
            Value::Integer(ts) => {
                let js_timestamp = JsValue::from_f64(bigint_to_f64(&ts) * 1000.0);
                let date = js_sys::Date::new(&js_timestamp);
                Ok(Value::String(format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    date.get_utc_full_year(),
                    date.get_utc_month() + 1,
                    date.get_utc_date(),
                    date.get_utc_hours(),
                    date.get_utc_minutes(),
                    date.get_utc_seconds()
                )))
            }
            Value::Float(ts) => {
                let js_timestamp = JsValue::from_f64(ts * 1000.0);
                let date = js_sys::Date::new(&js_timestamp);
                Ok(Value::String(format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    date.get_utc_full_year(),
                    date.get_utc_month() + 1,
                    date.get_utc_date(),
                    date.get_utc_hours(),
                    date.get_utc_minutes(),
                    date.get_utc_seconds()
                )))
            }
            _ => Err(runtime_err("TIME requires a numeric timestamp", span, env)),
        };
    }
}

fn eval_builtin_timezone(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err(
            "TIMEZONE requires two arguments: timestamp and timezone",
            span,
            env,
        ));
    }
    let timestamp = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let tz_name = evaluate_node(&args[1], Rc::clone(env), debug)?;
    if let Value::String(tz) = tz_name {
        use chrono::{TimeZone, Utc};
        use chrono_tz::Tz;
        let dt_utc = match timestamp {
            Value::Integer(ts) => {
                let ts_i64 = ts
                    .to_i64()
                    .ok_or_else(|| runtime_err("Timestamp value too large", span, env))?;
                Utc.timestamp_opt(ts_i64, 0)
                    .single()
                    .ok_or_else(|| runtime_err("Invalid timestamp", span, env))?
            }
            Value::Float(ts) => {
                let secs = ts.floor() as i64;
                let nanos = ((ts - ts.floor()) * 1_000_000_000.0) as u32;
                Utc.timestamp_opt(secs, nanos)
                    .single()
                    .ok_or_else(|| runtime_err("Invalid timestamp", span, env))?
            }
            _ => {
                return Err(runtime_err(
                    "TIMEZONE requires a numeric timestamp",
                    span,
                    env,
                ));
            }
        };
        let tz: Tz = tz
            .parse()
            .map_err(|_| runtime_err(format!("Invalid timezone: {}", tz), span, env))?;
        let dt_tz = dt_utc.with_timezone(&tz);
        Ok(Value::String(dt_tz.naive_local().to_string()))
    } else {
        Err(runtime_err(
            "TIMEZONE requires a timezone name (string)",
            span,
            env,
        ))
    }
}

fn eval_builtin_timezones(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> EvalResult {
    if !args.is_empty() {
        return Err(runtime_err("TIMEZONES takes no arguments", span, env));
    }
    use chrono_tz::TZ_VARIANTS;
    let tzs: Vec<Value> = TZ_VARIANTS
        .iter()
        .map(|tz| Value::String(tz.name().to_string()))
        .collect();
    Ok(Value::List(tzs))
}

fn eval_builtin_millitime(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> EvalResult {
    if !args.is_empty() {
        return Err(runtime_err("MILLITIME takes no arguments", span, env));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| runtime_err(e.to_string(), span, env))?;
    Ok(Value::Integer(BigInt::from(now.as_millis())))
}

fn eval_builtin_contains(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("CONTAINS requires two arguments", span, env));
    }
    let str_val = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let text_val = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match (str_val, text_val) {
        (Value::String(s), Value::String(t)) => Ok(Value::Boolean(s.contains(&t))),
        _ => Err(runtime_err(
            "CONTAINS requires two string arguments",
            span,
            env,
        )),
    }
}

fn eval_builtin_find(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("FIND requires two arguments", span, env));
    }
    let str_val = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let text_val = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match (str_val, text_val) {
        (Value::String(s), Value::String(t)) => match s.find(&t) {
            Some(index) => Ok(Value::Integer(BigInt::from(index + 1))),
            None => Ok(Value::Integer(BigInt::from(-1))),
        },
        _ => Err(runtime_err("FIND requires two string arguments", span, env)),
    }
}

fn eval_builtin_range(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    match args.len() {
        1 => {
            let end = evaluate_node(&args[0], Rc::clone(env), debug)?;
            if let Value::Integer(end_val) = end {
                if end_val < BigInt::one() {
                    return Err(runtime_err(
                        "RANGE end value must be greater than 0",
                        span,
                        env,
                    ));
                }
                Ok(Value::List(bigint_range_inclusive(BigInt::one(), end_val)))
            } else {
                Err(runtime_err("RANGE requires integer arguments", span, env))
            }
        }
        2 => {
            let start = evaluate_node(&args[0], Rc::clone(env), debug)?;
            let end = evaluate_node(&args[1], Rc::clone(env), debug)?;
            if let (Value::Integer(start_val), Value::Integer(end_val)) = (start, end) {
                if end_val < start_val {
                    return Err(runtime_err(
                        "RANGE end value must be greater than or equal to start value",
                        span,
                        env,
                    ));
                }
                Ok(Value::List(bigint_range_inclusive(start_val, end_val)))
            } else {
                Err(runtime_err("RANGE requires integer arguments", span, env))
            }
        }
        _ => Err(runtime_err(
            "RANGE requires one or two arguments",
            span,
            env,
        )),
    }
}

fn eval_builtin_startswith(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("STARTSWITH requires two arguments", span, env));
    }
    let fullstring = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let substring = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match (fullstring, substring) {
        (Value::String(s), Value::String(sub)) => Ok(Value::Boolean(s.starts_with(&sub))),
        _ => Err(runtime_err(
            "STARTSWITH requires two string arguments",
            span,
            env,
        )),
    }
}

fn eval_builtin_endswith(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("ENDSWITH requires two arguments", span, env));
    }
    let fullstring = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let substring = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match (fullstring, substring) {
        (Value::String(s), Value::String(sub)) => Ok(Value::Boolean(s.ends_with(&sub))),
        _ => Err(runtime_err(
            "ENDSWITH requires two string arguments",
            span,
            env,
        )),
    }
}

fn eval_builtin_hasarg(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("HASARG requires one argument", span, env));
    }
    let name_val = evaluate_node(&args[0], Rc::clone(env), debug)?;
    match name_val {
        Value::String(name) => {
            let key = name.trim_start_matches('-');
            let found = env.borrow().parsed_flags.contains_key(key);
            Ok(Value::Boolean(found))
        }
        _ => Err(runtime_err("HASARG requires a string argument", span, env)),
    }
}

fn eval_builtin_getarg(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.is_empty() || args.len() > 2 {
        return Err(runtime_err("GETARG requires 1 or 2 arguments", span, env));
    }
    let name_val = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let key = match &name_val {
        Value::String(name) => name.trim_start_matches('-').to_string(),
        _ => {
            return Err(runtime_err(
                "GETARG requires a string as the first argument",
                span,
                env,
            ));
        }
    };
    let flags = Rc::clone(&env.borrow().parsed_flags);
    match flags.get(&key) {
        Some(val) => Ok(Value::String(val.clone())),
        None if args.len() == 2 => evaluate_node(&args[1], Rc::clone(env), debug),
        None => Err(runtime_err(
            format!("Argument '{}' not found", key),
            span,
            env,
        )),
    }
}

fn eval_single_num_fn(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
    name: &str,
    f: fn(f64) -> f64,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err(
            format!("{} requires one argument", name),
            span,
            env,
        ));
    }
    let x = evaluate_node(&args[0], Rc::clone(env), debug)?;
    match x {
        Value::Float(v) => Ok(Value::Float(f(v))),
        Value::Integer(n) => Ok(Value::Float(f(bigint_to_f64(&n)))),
        _ => Err(runtime_err(
            format!("{} requires a numeric argument", name),
            span,
            env,
        )),
    }
}

fn mixed_arithmetic(a: f64, op: &BinaryOperator, b: f64) -> Result<Value, String> {
    match op {
        BinaryOperator::Add => Ok(Value::Float(a + b)),
        BinaryOperator::Sub => Ok(Value::Float(a - b)),
        BinaryOperator::Mul => Ok(Value::Float(a * b)),
        BinaryOperator::Div => {
            if b == 0.0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Float(a / b))
            }
        }
        BinaryOperator::Mod => {
            if b == 0.0 {
                Err("Modulo by zero".to_string())
            } else {
                Ok(Value::Float(a % b))
            }
        }
        _ => unreachable!("mixed_arithmetic called with non-arithmetic operator"),
    }
}

fn mixed_compare(a: f64, op: &BinaryOperator, b: f64) -> Result<Value, String> {
    let result = match op {
        BinaryOperator::Eq => a == b,
        BinaryOperator::NotEq => a != b,
        BinaryOperator::Lt => a < b,
        BinaryOperator::LtEq => a <= b,
        BinaryOperator::Gt => a > b,
        BinaryOperator::GtEq => a >= b,
        _ => unreachable!("mixed_compare called with non-comparison operator"),
    };
    Ok(Value::Boolean(result))
}

// skipcq: RS-R1000
fn evaluate_binary_op(left: &Value, op: &BinaryOperator, right: &Value) -> Result<Value, String> {
    match (left, op, right) {
        (Value::NaN, BinaryOperator::Eq, _) => Ok(Value::Boolean(false)),
        (_, BinaryOperator::Eq, Value::NaN) => Ok(Value::Boolean(false)),
        (Value::NaN, BinaryOperator::NotEq, _) => Ok(Value::Boolean(true)),
        (_, BinaryOperator::NotEq, Value::NaN) => Ok(Value::Boolean(true)),
        (Value::NaN, _, _) | (_, _, Value::NaN) => Ok(Value::NaN),

        (Value::Null, BinaryOperator::Eq, Value::Null) => Ok(Value::Boolean(true)),
        (Value::Null, BinaryOperator::NotEq, Value::Null) => Ok(Value::Boolean(false)),
        (Value::Null, _, _) | (_, _, Value::Null) => Ok(Value::Boolean(false)),

        // BigInt arithmetic
        (Value::Integer(a), BinaryOperator::Add, Value::Integer(b)) => Ok(Value::Integer(a + b)),
        (Value::Integer(a), BinaryOperator::Sub, Value::Integer(b)) => Ok(Value::Integer(a - b)),
        (Value::Integer(a), BinaryOperator::Mul, Value::Integer(b)) => Ok(Value::Integer(a * b)),
        (Value::Integer(a), BinaryOperator::Div, Value::Integer(b)) => {
            if b.is_zero() {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Integer(a / b))
            }
        }
        (Value::Integer(a), BinaryOperator::Mod, Value::Integer(b)) => {
            if b.is_zero() {
                Err("Modulo by zero".to_string())
            } else {
                Ok(Value::Integer(a % b))
            }
        }

        // BigInt comparisons
        (Value::Integer(a), BinaryOperator::Eq, Value::Integer(b)) => Ok(Value::Boolean(a == b)),
        (Value::Integer(a), BinaryOperator::NotEq, Value::Integer(b)) => Ok(Value::Boolean(a != b)),
        (Value::Integer(a), BinaryOperator::Lt, Value::Integer(b)) => Ok(Value::Boolean(a < b)),
        (Value::Integer(a), BinaryOperator::LtEq, Value::Integer(b)) => Ok(Value::Boolean(a <= b)),
        (Value::Integer(a), BinaryOperator::Gt, Value::Integer(b)) => Ok(Value::Boolean(a > b)),
        (Value::Integer(a), BinaryOperator::GtEq, Value::Integer(b)) => Ok(Value::Boolean(a >= b)),

        // Boolean
        (Value::Boolean(a), BinaryOperator::And, Value::Boolean(b)) => Ok(Value::Boolean(*a && *b)),
        (Value::Boolean(a), BinaryOperator::Or, Value::Boolean(b)) => Ok(Value::Boolean(*a || *b)),

        // String
        (Value::String(a), BinaryOperator::Eq, Value::String(b)) => Ok(Value::Boolean(a == b)),
        (Value::String(a), BinaryOperator::NotEq, Value::String(b)) => Ok(Value::Boolean(a != b)),
        (Value::String(a), BinaryOperator::Lt, Value::String(b)) => Ok(Value::Boolean(a < b)),
        (Value::String(a), BinaryOperator::LtEq, Value::String(b)) => Ok(Value::Boolean(a <= b)),
        (Value::String(a), BinaryOperator::Gt, Value::String(b)) => Ok(Value::Boolean(a > b)),
        (Value::String(a), BinaryOperator::GtEq, Value::String(b)) => Ok(Value::Boolean(a >= b)),
        (Value::String(a), BinaryOperator::Add, Value::String(b)) => {
            Ok(Value::String(format!("{}{}", a, b)))
        }

        // Float arithmetic
        (Value::Float(a), BinaryOperator::Add, Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::Float(a), BinaryOperator::Sub, Value::Float(b)) => Ok(Value::Float(a - b)),
        (Value::Float(a), BinaryOperator::Mul, Value::Float(b)) => Ok(Value::Float(a * b)),
        (Value::Float(a), BinaryOperator::Div, Value::Float(b)) => {
            if *b == 0.0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Float(a / b))
            }
        }

        // Mixed Integer/Float arithmetic
        (Value::Integer(a), op, Value::Float(b)) if op.is_arithmetic() => {
            mixed_arithmetic(bigint_to_f64(a), op, *b)
        }
        (Value::Float(a), op, Value::Integer(b)) if op.is_arithmetic() => {
            mixed_arithmetic(*a, op, bigint_to_f64(b))
        }

        // Boolean equality
        (Value::Boolean(a), BinaryOperator::Eq, Value::Boolean(b)) => Ok(Value::Boolean(a == b)),
        (Value::Boolean(a), BinaryOperator::NotEq, Value::Boolean(b)) => Ok(Value::Boolean(a != b)),

        // List concatenation
        (Value::List(a), BinaryOperator::Add, Value::List(b)) => {
            let mut result = a.clone();
            result.extend(b.iter().cloned());
            Ok(Value::List(result))
        }

        // Float comparisons
        (Value::Float(a), BinaryOperator::Eq, Value::Float(b)) => Ok(Value::Boolean(a == b)),
        (Value::Float(a), BinaryOperator::NotEq, Value::Float(b)) => Ok(Value::Boolean(a != b)),
        (Value::Float(a), BinaryOperator::Lt, Value::Float(b)) => Ok(Value::Boolean(a < b)),
        (Value::Float(a), BinaryOperator::LtEq, Value::Float(b)) => Ok(Value::Boolean(a <= b)),
        (Value::Float(a), BinaryOperator::Gt, Value::Float(b)) => Ok(Value::Boolean(a > b)),
        (Value::Float(a), BinaryOperator::GtEq, Value::Float(b)) => Ok(Value::Boolean(a >= b)),

        // Mixed Integer/Float comparisons
        (Value::Integer(a), op, Value::Float(b)) if op.is_comparison() => {
            mixed_compare(bigint_to_f64(a), op, *b)
        }
        (Value::Float(a), op, Value::Integer(b)) if op.is_comparison() => {
            mixed_compare(*a, op, bigint_to_f64(b))
        }

        _ => Err(format!(
            "Invalid operation: {:?} {:?} {:?}",
            left, op, right
        )),
    }
}

fn evaluate_unary_op(op: &UnaryOperator, val: &Value) -> Result<Value, String> {
    match (op, val) {
        (UnaryOperator::Neg, Value::Integer(n)) => Ok(Value::Integer(-n)),
        (UnaryOperator::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
        (UnaryOperator::Not, Value::Boolean(b)) => Ok(Value::Boolean(!b)),
        _ => Err(format!("Invalid unary operation: {:?} {:?}", op, val)),
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Integer(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.clone(),
        Value::Boolean(b) => b.to_string(),
        Value::List(elements) => {
            let elements_str: Vec<String> = elements.iter().map(value_to_string).collect();
            format!("[{}]", elements_str.join(", "))
        }
        Value::Unit => "".to_string(),
        Value::Null => "NULL".to_string(),
        Value::NaN => "NAN".to_string(),
    }
}

fn bigint_to_f64(n: &BigInt) -> f64 {
    n.to_f64().unwrap_or_else(|| {
        if n.is_negative() {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }
    })
}

fn bigint_gcd(m: &BigInt, n: &BigInt) -> BigInt {
    num_integer::gcd(m.clone(), n.clone())
}

fn bigint_range_inclusive(start: BigInt, end: BigInt) -> Vec<Value> {
    let mut list = Vec::new();
    let mut i = start;
    while i <= end {
        list.push(Value::Integer(i.clone()));
        i += BigInt::one();
    }
    list
}

fn bigint_factorial(n: &BigInt) -> BigInt {
    if n.is_negative() {
        BigInt::zero()
    } else {
        let mut result = BigInt::one();
        let mut i = BigInt::one();
        while i <= *n {
            result *= &i;
            i += BigInt::one();
        }
        result
    }
}

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
use wasm_bindgen::prelude::*;

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
use js_sys;

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = window)]
    fn prompt(s: &str) -> String;

    #[wasm_bindgen(js_namespace = Date, js_name = now)]
    fn date_now() -> f64;

    #[wasm_bindgen(js_namespace = performance, js_name = now, catch)]
    fn performance_now() -> Result<f64, JsValue>;
}

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
fn get_high_precision_time() -> f64 {
    match performance_now() {
        Ok(time) => time,
        Err(_) => {
            log("Warning: performance.now() not available, falling back to Date.now()");
            date_now() % 1000.0
        }
    }
}
