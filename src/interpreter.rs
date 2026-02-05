use crate::error::{PSLError, Span, StackFrame};
use crate::parser::{AstNode, BinaryOperator, Spanned, UnaryOperator};
use rand::Rng;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum Value {
    Integer(i64),
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
}

impl Environment {
    fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            procedures: HashMap::new(),
            output: String::new(),
            parent: None,
            call_stack: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn new_with_parent(parent: Rc<RefCell<Environment>>) -> Self {
        let procedures = parent.borrow().procedures.clone();
        let call_stack = Rc::clone(&parent.borrow().call_stack);
        Environment {
            variables: HashMap::new(),
            procedures,
            output: String::new(),
            parent: Some(Rc::clone(&parent)),
            call_stack,
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

#[allow(dead_code)]
pub fn run(ast: Spanned) -> Result<String, String> {
    let env = Rc::new(RefCell::new(Environment::new()));
    match evaluate_node(&ast, Rc::clone(&env), false) {
        Ok(_) => Ok(env.borrow().output.clone()),
        Err(Interruption::Return(_)) => Ok(env.borrow().output.clone()),
        Err(Interruption::Error(e)) => Err(e.message),
    }
}

pub fn run_with_source(ast: Spanned, _source: &str) -> Result<String, PSLError> {
    let env = Rc::new(RefCell::new(Environment::new()));
    match evaluate_node(&ast, Rc::clone(&env), false) {
        Ok(_) => Ok(env.borrow().output.clone()),
        Err(Interruption::Return(_)) => Ok(env.borrow().output.clone()),
        Err(Interruption::Error(e)) => Err(e),
    }
}

fn evaluate_node(node: &Spanned, env: Rc<RefCell<Environment>>, debug: bool) -> EvalResult {
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

        AstNode::Integer(n) => Ok(Value::Integer(*n)),
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
                if let Value::Boolean(false) = left_val {
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
                if let Value::Boolean(true) = left_val {
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
                for _ in 0..n {
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
            #[cfg(not(target_arch = "wasm32"))]
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

            #[cfg(all(target_arch = "wasm32", feature = "wasi"))]
            {
                Err(runtime_err(
                    "INPUT is not supported in this environment",
                    span,
                    &env,
                ))
            }
        }

        AstNode::ProcedureDecl(name, params, body) => {
            env.borrow_mut()
                .procedures
                .insert(name.clone(), (params.clone(), (**body).clone()));
            Ok(Value::Unit)
        }

        AstNode::ProcedureCall(name, args) => match name.as_str() {
            "SLEEP" => {
                if args.len() != 1 {
                    return Err(runtime_err("SLEEP requires one argument", span, &env));
                }
                io::stdout().flush().unwrap();

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let seconds = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match seconds {
                        Value::Integer(n) => {
                            thread::sleep(Duration::from_secs(n as u64));
                            Ok(Value::Unit)
                        }
                        Value::Float(f) => {
                            thread::sleep(Duration::from_secs_f64(f));
                            Ok(Value::Unit)
                        }
                        _ => Err(runtime_err("SLEEP requires a numeric argument", span, &env)),
                    }
                }

                #[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
                {
                    let _seconds = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    log(
                        "SLEEP function is not fully supported in WebAssembly. The program will continue without pausing.",
                    );
                    Ok(Value::Unit)
                }

                #[cfg(all(target_arch = "wasm32", feature = "wasi"))]
                {
                    Err(runtime_err(
                        "SLEEP is not supported in this environment",
                        span,
                        &env,
                    ))
                }
            }
            "CONCAT" => {
                if args.len() != 2 {
                    return Err(runtime_err("CONCAT requires two arguments", span, &env));
                }
                let s1 = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let s2 = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                if let (Value::String(a), Value::String(b)) = (s1, s2) {
                    Ok(Value::String(format!("{}{}", a, b)))
                } else {
                    Err(runtime_err("CONCAT requires string arguments", span, &env))
                }
            }
            "SUBSTRING" => {
                if args.len() != 3 {
                    return Err(runtime_err(
                        "SUBSTRING requires three arguments",
                        span,
                        &env,
                    ));
                }
                let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let start_val = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                let end_val = evaluate_node(&args[2], Rc::clone(&env), debug)?;
                if let (Value::String(s), Value::Integer(start), Value::Integer(end)) =
                    (str_val, start_val, end_val)
                {
                    let start_idx = start - 1;
                    let end_idx = end - 1;
                    if start_idx >= 0 && end_idx >= start_idx && (end_idx as usize) < s.len() {
                        Ok(Value::String(
                            s[start_idx as usize..=end_idx as usize].to_string(),
                        ))
                    } else {
                        Err(runtime_err("Invalid substring indices", span, &env))
                    }
                } else {
                    Err(runtime_err("Invalid substring arguments", span, &env))
                }
            }
            "LENGTH" => {
                if args.len() != 1 {
                    return Err(runtime_err("LENGTH requires one argument", span, &env));
                }
                let arg = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                match arg {
                    Value::List(elements) => Ok(Value::Integer(elements.len() as i64)),
                    Value::String(s) => Ok(Value::Integer(s.len() as i64)),
                    _ => Err(runtime_err(
                        "LENGTH requires a list or string argument",
                        span,
                        &env,
                    )),
                }
            }
            "REMOVE" => {
                if args.len() != 2 {
                    return Err(runtime_err("REMOVE requires two arguments", span, &env));
                }
                let synth = Spanned::new(
                    AstNode::Remove(Box::new(args[0].clone()), Box::new(args[1].clone())),
                    span,
                );
                evaluate_node(&synth, Rc::clone(&env), debug)
            }
            "APPEND" => {
                if args.len() != 2 {
                    return Err(runtime_err("APPEND requires two arguments", span, &env));
                }
                let synth = Spanned::new(
                    AstNode::Append(Box::new(args[0].clone()), Box::new(args[1].clone())),
                    span,
                );
                evaluate_node(&synth, Rc::clone(&env), debug)
            }
            "INSERT" => {
                if args.len() != 3 {
                    return Err(runtime_err("INSERT requires three arguments", span, &env));
                }
                let synth = Spanned::new(
                    AstNode::Insert(
                        Box::new(args[0].clone()),
                        Box::new(args[1].clone()),
                        Box::new(args[2].clone()),
                    ),
                    span,
                );
                evaluate_node(&synth, Rc::clone(&env), debug)
            }
            "ABS" => {
                if args.len() != 1 {
                    return Err(runtime_err("ABS requires one argument", span, &env));
                }
                let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                match x {
                    Value::Integer(n) => Ok(Value::Integer(n.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err(runtime_err("ABS requires a numeric argument", span, &env)),
                }
            }
            "CEIL" => {
                if args.len() != 1 {
                    return Err(runtime_err("CEIL requires one argument", span, &env));
                }
                let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                match x {
                    Value::Float(f) => Ok(Value::Integer(f.ceil() as i64)),
                    Value::Integer(n) => Ok(Value::Integer(n)),
                    _ => Err(runtime_err("CEIL requires a numeric argument", span, &env)),
                }
            }
            "FLOOR" => {
                if args.len() != 1 {
                    return Err(runtime_err("FLOOR requires one argument", span, &env));
                }
                let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                match x {
                    Value::Float(f) => Ok(Value::Integer(f.floor() as i64)),
                    Value::Integer(n) => Ok(Value::Integer(n)),
                    _ => Err(runtime_err("FLOOR requires a numeric argument", span, &env)),
                }
            }
            "POW" => {
                if args.len() != 2 {
                    return Err(runtime_err("POW requires two arguments", span, &env));
                }
                let base = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let exponent = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                match (base, exponent) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        Ok(Value::Float((a as f64).powi(b as i32)))
                    }
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a.powi(b as i32))),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(b))),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Float((a as f64).powf(b))),
                    _ => Err(runtime_err("POW requires numeric arguments", span, &env)),
                }
            }
            "SQRT" => {
                if args.len() != 1 {
                    return Err(runtime_err("SQRT requires one argument", span, &env));
                }
                let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                match x {
                    Value::Integer(n) => Ok(Value::Float((n as f64).sqrt())),
                    Value::Float(f) => Ok(Value::Float(f.sqrt())),
                    _ => Err(runtime_err("SQRT requires a numeric argument", span, &env)),
                }
            }
            "SIN" => eval_single_num_fn(args, &env, span, debug, "SIN", |v| v.sin()),
            "COS" => eval_single_num_fn(args, &env, span, debug, "COS", |v| v.cos()),
            "TAN" => eval_single_num_fn(args, &env, span, debug, "TAN", |v| v.tan()),
            "ASIN" => eval_single_num_fn(args, &env, span, debug, "ASIN", |v| v.asin()),
            "ACOS" => eval_single_num_fn(args, &env, span, debug, "ACOS", |v| v.acos()),
            "ATAN" => eval_single_num_fn(args, &env, span, debug, "ATAN", |v| v.atan()),
            "EXP" => eval_single_num_fn(args, &env, span, debug, "EXP", |v| v.exp()),
            "LOG" | "NLOG" => eval_single_num_fn(args, &env, span, debug, "LOG", |v| v.ln()),
            "LOGTEN" => eval_single_num_fn(args, &env, span, debug, "LOGTEN", |v| v.log10()),
            "LOGTWO" => eval_single_num_fn(args, &env, span, debug, "LOGTWO", |v| v.log2()),
            "DEGREES" => eval_single_num_fn(args, &env, span, debug, "DEGREES", |v| v.to_degrees()),
            "RADIANS" => eval_single_num_fn(args, &env, span, debug, "RADIANS", |v| v.to_radians()),
            "GCD" => {
                if args.len() != 2 {
                    return Err(runtime_err("GCD requires two arguments", span, &env));
                }
                let a = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let b = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                match (a, b) {
                    (Value::Integer(m), Value::Integer(n)) => Ok(Value::Integer(gcd(m, n))),
                    _ => Err(runtime_err("GCD requires integer arguments", span, &env)),
                }
            }
            "FACTORIAL" => {
                if args.len() != 1 {
                    return Err(runtime_err("FACTORIAL requires one argument", span, &env));
                }
                let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                if let Value::Integer(n) = x {
                    Ok(Value::Integer(factorial(n)))
                } else {
                    Err(runtime_err(
                        "FACTORIAL requires an integer argument",
                        span,
                        &env,
                    ))
                }
            }
            "HYPOT" => {
                if args.len() != 2 {
                    return Err(runtime_err("HYPOT requires two arguments", span, &env));
                }
                let a = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let b = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                match (a, b) {
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.hypot(y))),
                    (Value::Integer(x), Value::Float(y)) => Ok(Value::Float((x as f64).hypot(y))),
                    (Value::Float(x), Value::Integer(y)) => Ok(Value::Float(x.hypot(y as f64))),
                    (Value::Integer(x), Value::Integer(y)) => {
                        Ok(Value::Float((x as f64).hypot(y as f64)))
                    }
                    _ => Err(runtime_err("HYPOT requires numeric arguments", span, &env)),
                }
            }
            "MIN" => {
                if args.len() != 2 {
                    return Err(runtime_err("MIN requires two arguments", span, &env));
                }
                let a = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let b = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                match (a, b) {
                    (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(x.min(y))),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.min(y))),
                    (Value::Integer(x), Value::Float(y)) => Ok(Value::Float((x as f64).min(y))),
                    (Value::Float(x), Value::Integer(y)) => Ok(Value::Float(x.min(y as f64))),
                    _ => Err(runtime_err(
                        "MIN requires two numeric arguments",
                        span,
                        &env,
                    )),
                }
            }
            "MAX" => {
                if args.len() != 2 {
                    return Err(runtime_err("MAX requires two arguments", span, &env));
                }
                let a = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let b = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                match (a, b) {
                    (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(x.max(y))),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.max(y))),
                    (Value::Integer(x), Value::Float(y)) => Ok(Value::Float((x as f64).max(y))),
                    (Value::Float(x), Value::Integer(y)) => Ok(Value::Float(x.max(y as f64))),
                    _ => Err(runtime_err(
                        "MAX requires two numeric arguments",
                        span,
                        &env,
                    )),
                }
            }
            "EXIT" => {
                std::process::exit(0);
            }
            "ROUND" => {
                if args.len() != 1 {
                    return Err(runtime_err("ROUND requires one argument", span, &env));
                }
                let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                match x {
                    Value::Float(f) => Ok(Value::Integer(f.round() as i64)),
                    Value::Integer(n) => Ok(Value::Integer(n)),
                    _ => Err(runtime_err("ROUND requires a numeric argument", span, &env)),
                }
            }
            "SPLIT" => {
                if args.len() != 2 {
                    return Err(runtime_err("SPLIT requires two arguments", span, &env));
                }
                let string_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let delimiter_val = evaluate_node(&args[1], Rc::clone(&env), debug)?;
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
                        &env,
                    )),
                }
            }
            "TRIM" => {
                if args.len() != 1 {
                    return Err(runtime_err("TRIM requires one argument", span, &env));
                }
                let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                if let Value::String(s) = str_val {
                    Ok(Value::String(s.trim().to_string()))
                } else {
                    Err(runtime_err("TRIM requires a string argument", span, &env))
                }
            }
            "REPLACE" => {
                if args.len() != 3 {
                    return Err(runtime_err("REPLACE requires three arguments", span, &env));
                }
                let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let from_val = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                let to_val = evaluate_node(&args[2], Rc::clone(&env), debug)?;
                match (str_val, from_val, to_val) {
                    (Value::String(s), Value::String(from), Value::String(to)) => {
                        Ok(Value::String(s.replace(&from, &to)))
                    }
                    _ => Err(runtime_err(
                        "REPLACE requires three string arguments",
                        span,
                        &env,
                    )),
                }
            }
            "UPPERCASE" => {
                if args.len() != 1 {
                    return Err(runtime_err("UPPERCASE requires one argument", span, &env));
                }
                let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                if let Value::String(s) = str_val {
                    Ok(Value::String(s.to_uppercase()))
                } else {
                    Err(runtime_err(
                        "UPPERCASE requires a string argument",
                        span,
                        &env,
                    ))
                }
            }
            "LOWERCASE" => {
                if args.len() != 1 {
                    return Err(runtime_err("LOWERCASE requires one argument", span, &env));
                }
                let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                if let Value::String(s) = str_val {
                    Ok(Value::String(s.to_lowercase()))
                } else {
                    Err(runtime_err(
                        "LOWERCASE requires a string argument",
                        span,
                        &env,
                    ))
                }
            }
            "TIMESTAMP" => match args.len() {
                0 => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(|e| runtime_err(e.to_string(), span, &env))?;
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
                        Ok(Value::Float(timestamp))
                    }

                    #[cfg(all(target_arch = "wasm32", feature = "wasi"))]
                    {
                        Err(runtime_err(
                            "TIMESTAMP is not supported in this environment",
                            span,
                            &env,
                        ))
                    }
                }
                1 => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let datetime = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                        if let Value::String(dt) = datetime {
                            use chrono::NaiveDateTime;
                            match NaiveDateTime::parse_from_str(&dt, "%Y-%m-%d %H:%M:%S%.f") {
                                Ok(dt) => {
                                    let timestamp = dt.and_utc().timestamp() as f64;
                                    let nanos = dt.and_utc().timestamp_subsec_nanos() as f64
                                        / 1_000_000_000.0;
                                    Ok(Value::Float(timestamp + nanos))
                                }
                                Err(e) => Err(runtime_err(
                                    format!("Invalid datetime format: {}", e),
                                    span,
                                    &env,
                                )),
                            }
                        } else {
                            Err(runtime_err(
                                "TIMESTAMP requires a datetime string",
                                span,
                                &env,
                            ))
                        }
                    }

                    #[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
                    {
                        let timestamp = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                        match timestamp {
                            Value::Integer(ts) => {
                                let js_timestamp = JsValue::from_f64((ts as f64) * 1000.0);
                                let date = js_sys::Date::new(&js_timestamp);
                                let year = date.get_utc_full_year();
                                let month = date.get_utc_month() + 1;
                                let day = date.get_utc_date();
                                let hours = date.get_utc_hours();
                                let minutes = date.get_utc_minutes();
                                let seconds = date.get_utc_seconds();
                                let formatted = format!(
                                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                                    year, month, day, hours, minutes, seconds
                                );
                                Ok(Value::String(formatted))
                            }
                            Value::Float(ts) => {
                                let js_timestamp = JsValue::from_f64(ts * 1000.0);
                                let date = js_sys::Date::new(&js_timestamp);
                                let year = date.get_utc_full_year();
                                let month = date.get_utc_month() + 1;
                                let day = date.get_utc_date();
                                let hours = date.get_utc_hours();
                                let minutes = date.get_utc_minutes();
                                let seconds = date.get_utc_seconds();
                                let formatted = format!(
                                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                                    year, month, day, hours, minutes, seconds
                                );
                                Ok(Value::String(formatted))
                            }
                            _ => Err(runtime_err("TIME requires a numeric timestamp", span, &env)),
                        }
                    }

                    #[cfg(all(target_arch = "wasm32", feature = "wasi"))]
                    {
                        Err(runtime_err(
                            "TIMESTAMP is not supported in this environment",
                            span,
                            &env,
                        ))
                    }
                }
                _ => Err(runtime_err(
                    "TIMESTAMP requires 0 or 1 arguments",
                    span,
                    &env,
                )),
            },
            "TIME" => {
                if args.len() != 1 {
                    return Err(runtime_err("TIME requires one argument", span, &env));
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let timestamp = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match timestamp {
                        Value::Integer(ts) => {
                            use chrono::{TimeZone, Utc};
                            let dt = Utc
                                .timestamp_opt(ts, 0)
                                .single()
                                .ok_or_else(|| runtime_err("Invalid timestamp", span, &env))?;
                            Ok(Value::String(dt.naive_local().to_string()))
                        }
                        Value::Float(ts) => {
                            use chrono::{TimeZone, Utc};
                            let secs = ts.floor() as i64;
                            let nanos = ((ts - ts.floor()) * 1_000_000_000.0) as u32;
                            let dt = Utc
                                .timestamp_opt(secs, nanos)
                                .single()
                                .ok_or_else(|| runtime_err("Invalid timestamp", span, &env))?;
                            Ok(Value::String(dt.naive_local().to_string()))
                        }
                        _ => Err(runtime_err("TIME requires a numeric timestamp", span, &env)),
                    }
                }

                #[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
                {
                    let timestamp = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match timestamp {
                        Value::Integer(ts) => {
                            let js_timestamp = JsValue::from_f64((ts as f64) * 1000.0);
                            let date = js_sys::Date::new(&js_timestamp);
                            let year = date.get_utc_full_year();
                            let month = date.get_utc_month() + 1;
                            let day = date.get_utc_date();
                            let hours = date.get_utc_hours();
                            let minutes = date.get_utc_minutes();
                            let seconds = date.get_utc_seconds();
                            let formatted = format!(
                                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                                year, month, day, hours, minutes, seconds
                            );
                            Ok(Value::String(formatted))
                        }
                        Value::Float(ts) => {
                            let js_timestamp = JsValue::from_f64(ts * 1000.0);
                            let date = js_sys::Date::new(&js_timestamp);
                            let year = date.get_utc_full_year();
                            let month = date.get_utc_month() + 1;
                            let day = date.get_utc_date();
                            let hours = date.get_utc_hours();
                            let minutes = date.get_utc_minutes();
                            let seconds = date.get_utc_seconds();
                            let formatted = format!(
                                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                                year, month, day, hours, minutes, seconds
                            );
                            Ok(Value::String(formatted))
                        }
                        _ => Err(runtime_err("TIME requires a numeric timestamp", span, &env)),
                    }
                }

                #[cfg(all(target_arch = "wasm32", feature = "wasi"))]
                {
                    Err(runtime_err(
                        "TIME is not supported in this environment",
                        span,
                        &env,
                    ))
                }
            }
            "TIMEZONE" => {
                if args.len() != 2 {
                    return Err(runtime_err(
                        "TIMEZONE requires two arguments: timestamp and timezone",
                        span,
                        &env,
                    ));
                }
                let timestamp = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let tz_name = evaluate_node(&args[1], Rc::clone(&env), debug)?;

                if let Value::String(tz) = tz_name {
                    use chrono::{TimeZone, Utc};
                    use chrono_tz::Tz;

                    let dt_utc = match timestamp {
                        Value::Integer(ts) => Utc
                            .timestamp_opt(ts, 0)
                            .single()
                            .ok_or_else(|| runtime_err("Invalid timestamp", span, &env))?,
                        Value::Float(ts) => {
                            let secs = ts.floor() as i64;
                            let nanos = ((ts - ts.floor()) * 1_000_000_000.0) as u32;
                            Utc.timestamp_opt(secs, nanos)
                                .single()
                                .ok_or_else(|| runtime_err("Invalid timestamp", span, &env))?
                        }
                        _ => {
                            return Err(runtime_err(
                                "TIMEZONE requires a numeric timestamp",
                                span,
                                &env,
                            ));
                        }
                    };

                    let tz: Tz = tz.parse().map_err(|_| {
                        runtime_err(format!("Invalid timezone: {}", tz), span, &env)
                    })?;

                    let dt_tz = dt_utc.with_timezone(&tz);
                    Ok(Value::String(dt_tz.naive_local().to_string()))
                } else {
                    Err(runtime_err(
                        "TIMEZONE requires a timezone name (string)",
                        span,
                        &env,
                    ))
                }
            }
            "TIMEZONES" => {
                if !args.is_empty() {
                    return Err(runtime_err("TIMEZONES takes no arguments", span, &env));
                }
                use chrono_tz::TZ_VARIANTS;
                let tzs: Vec<Value> = TZ_VARIANTS
                    .iter()
                    .map(|tz| Value::String(tz.name().to_string()))
                    .collect();
                Ok(Value::List(tzs))
            }
            "MILLITIME" => {
                if !args.is_empty() {
                    return Err(runtime_err("MILLITIME takes no arguments", span, &env));
                }
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| runtime_err(e.to_string(), span, &env))?;
                let millis = now.as_millis();
                let millis = std::cmp::min(millis, i64::MAX as u128) as i64;
                Ok(Value::Integer(millis))
            }
            "CONTAINS" => {
                if args.len() != 2 {
                    return Err(runtime_err("CONTAINS requires two arguments", span, &env));
                }
                let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let text_val = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                match (str_val, text_val) {
                    (Value::String(s), Value::String(t)) => Ok(Value::Boolean(s.contains(&t))),
                    _ => Err(runtime_err(
                        "CONTAINS requires two string arguments",
                        span,
                        &env,
                    )),
                }
            }
            "FIND" => {
                if args.len() != 2 {
                    return Err(runtime_err("FIND requires two arguments", span, &env));
                }
                let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let text_val = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                match (str_val, text_val) {
                    (Value::String(s), Value::String(t)) => match s.find(&t) {
                        Some(index) => Ok(Value::Integer((index + 1) as i64)),
                        None => Ok(Value::Integer(-1)),
                    },
                    _ => Err(runtime_err(
                        "FIND requires two string arguments",
                        span,
                        &env,
                    )),
                }
            }
            "RANGE" => match args.len() {
                1 => {
                    let end = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    if let Value::Integer(end_val) = end {
                        if end_val < 1 {
                            return Err(runtime_err(
                                "RANGE end value must be greater than 0",
                                span,
                                &env,
                            ));
                        }
                        let list: Vec<Value> = (1..=end_val).map(Value::Integer).collect();
                        Ok(Value::List(list))
                    } else {
                        Err(runtime_err("RANGE requires integer arguments", span, &env))
                    }
                }
                2 => {
                    let start = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    let end = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                    if let (Value::Integer(start_val), Value::Integer(end_val)) = (start, end) {
                        if end_val < start_val {
                            return Err(runtime_err(
                                "RANGE end value must be greater than or equal to start value",
                                span,
                                &env,
                            ));
                        }
                        let list: Vec<Value> = (start_val..=end_val).map(Value::Integer).collect();
                        Ok(Value::List(list))
                    } else {
                        Err(runtime_err("RANGE requires integer arguments", span, &env))
                    }
                }
                _ => Err(runtime_err(
                    "RANGE requires one or two arguments",
                    span,
                    &env,
                )),
            },
            "STARTSWITH" => {
                if args.len() != 2 {
                    return Err(runtime_err("STARTSWITH requires two arguments", span, &env));
                }
                let fullstring = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let substring = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                match (fullstring, substring) {
                    (Value::String(s), Value::String(sub)) => {
                        Ok(Value::Boolean(s.starts_with(&sub)))
                    }
                    _ => Err(runtime_err(
                        "STARTSWITH requires two string arguments",
                        span,
                        &env,
                    )),
                }
            }
            "ENDSWITH" => {
                if args.len() != 2 {
                    return Err(runtime_err("ENDSWITH requires two arguments", span, &env));
                }
                let fullstring = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                let substring = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                match (fullstring, substring) {
                    (Value::String(s), Value::String(sub)) => Ok(Value::Boolean(s.ends_with(&sub))),
                    _ => Err(runtime_err(
                        "ENDSWITH requires two string arguments",
                        span,
                        &env,
                    )),
                }
            }
            _ => {
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

                let local_env =
                    Rc::new(RefCell::new(Environment::new_with_parent(Rc::clone(&env))));

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
        },

        AstNode::ListAccess(list, index) => {
            let current_value = evaluate_node(list, Rc::clone(&env), debug)?;
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;

            match (current_value, index_val) {
                (Value::List(elements), Value::Integer(i)) => {
                    let idx = i - 1;
                    if idx < 0 {
                        Err(runtime_err(
                            "List index out of bounds: index cannot be less than 1",
                            span,
                            &env,
                        ))
                    } else if (idx as usize) >= elements.len() {
                        Err(runtime_err(
                            format!("List index out of bounds: {} (size: {})", i, elements.len()),
                            span,
                            &env,
                        ))
                    } else {
                        Ok(elements[idx as usize].clone())
                    }
                }
                (Value::String(s), Value::Integer(i)) => {
                    let idx = i - 1;
                    if idx < 0 {
                        Err(runtime_err(
                            "String index out of bounds: index cannot be less than 1",
                            span,
                            &env,
                        ))
                    } else if (idx as usize) >= s.len() {
                        Err(runtime_err(
                            format!("String index out of bounds: {} (size: {})", i, s.len()),
                            span,
                            &env,
                        ))
                    } else {
                        let ch = s
                            .chars()
                            .nth(idx as usize)
                            .ok_or_else(|| runtime_err("Invalid string index", span, &env))?;
                        Ok(Value::String(ch.to_string()))
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
                    let idx = i - 1;
                    if idx >= 0 && (idx as usize) < elements.len() {
                        let mut new_elements = elements.clone();
                        new_elements[idx as usize] = new_val.clone();
                        env.borrow_mut()
                            .set(name.clone(), Value::List(new_elements));
                        Ok(new_val)
                    } else {
                        Err(runtime_err("List index out of bounds", span, &env))
                    }
                } else {
                    Err(runtime_err("Invalid list index", span, &env))
                }
            } else if let AstNode::ListAccess(inner_list, inner_index) = &list.node {
                let list_val = evaluate_node(inner_list, Rc::clone(&env), debug)?;
                let index_inner = evaluate_node(inner_index, Rc::clone(&env), debug)?;

                if let (Value::List(mut elements), Value::Integer(i)) = (list_val, index_inner) {
                    let idx = i - 1;
                    if idx >= 0
                        && (idx as usize) < elements.len()
                        && let Value::Integer(j) = index_val
                    {
                        let jdx = j - 1;
                        if let Value::List(mut inner_elements) = elements[idx as usize].clone()
                            && jdx >= 0
                            && (jdx as usize) < inner_elements.len()
                        {
                            inner_elements[jdx as usize] = new_val.clone();
                            elements[idx as usize] = Value::List(inner_elements);

                            if let AstNode::Identifier(name) = &inner_list.node {
                                env.borrow_mut().set(name.clone(), Value::List(elements));
                                return Ok(new_val);
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
                let start_idx = start - 1;
                let end_idx = end - 1;
                if start_idx >= 0 && end_idx >= start_idx && (end_idx as usize) <= s.len() {
                    Ok(Value::String(
                        s[start_idx as usize..=end_idx as usize].to_string(),
                    ))
                } else {
                    Err(runtime_err("Invalid substring indices", span, &env))
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
                if let Ok(n) = s.parse::<i64>() {
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
            let mut result = String::new();
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
                Value::List(elements) => Ok(Value::Integer(elements.len() as i64)),
                Value::String(s) => Ok(Value::Integer(s.len() as i64)),
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
                    let idx = i - 1;
                    if idx >= 0 && (idx as usize) <= elements.len() {
                        let mut new_elements = elements.clone();
                        new_elements.insert(idx as usize, insert_val.clone());
                        env.borrow_mut()
                            .set(name.clone(), Value::List(new_elements));
                        Ok(insert_val)
                    } else {
                        Err(runtime_err("List index out of bounds", span, &env))
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
                    let idx = i - 1;
                    if idx >= 0 && (idx as usize) < elements.len() {
                        let mut new_elements = elements.clone();
                        let removed_value = new_elements.remove(idx as usize);
                        env.borrow_mut()
                            .set(name.clone(), Value::List(new_elements));
                        Ok(removed_value)
                    } else {
                        Err(runtime_err("List index out of bounds", span, &env))
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
                    let mut rng = rand::rng();
                    Ok(Value::Integer(rng.random_range(min_int..=max_int)))
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
                    (Value::Integer(a_int), Value::Float(b_float)) => (*a_int as f64)
                        .partial_cmp(b_float)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    (Value::Float(a_float), Value::Integer(b_int)) => a_float
                        .partial_cmp(&(*b_int as f64))
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
        } => {
            let try_env = Rc::new(RefCell::new(Environment::new_with_parent(Rc::clone(&env))));

            match evaluate_node(try_block, Rc::clone(&try_env), debug) {
                Ok(result) => {
                    env.borrow_mut().output.push_str(&try_env.borrow().output);
                    Ok(result)
                }
                Err(Interruption::Return(val)) => {
                    env.borrow_mut().output.push_str(&try_env.borrow().output);
                    Err(Interruption::Return(val))
                }
                Err(Interruption::Error(error)) => {
                    env.borrow_mut().output.push_str(&try_env.borrow().output);

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
            }
        }

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
        Value::Integer(n) => Ok(Value::Float(f(n as f64))),
        _ => Err(runtime_err(
            format!("{} requires a numeric argument", name),
            span,
            env,
        )),
    }
}

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

        (Value::Integer(a), BinaryOperator::Add, Value::Integer(b)) => {
            if (*a > 0 && *b > i64::MAX - *a) || (*a < 0 && *b < i64::MIN - *a) {
                Ok(Value::Float(*a as f64 + *b as f64))
            } else {
                Ok(Value::Integer(a + b))
            }
        }
        (Value::Integer(a), BinaryOperator::Sub, Value::Integer(b)) => {
            if (*b > 0 && *a < i64::MIN + *b) || (*b < 0 && *a > i64::MAX + *b) {
                Ok(Value::Float(*a as f64 - *b as f64))
            } else {
                Ok(Value::Integer(a - b))
            }
        }
        (Value::Integer(a), BinaryOperator::Mul, Value::Integer(b)) => {
            if *a != 0 && *b != 0 {
                if (*a > 0 && *b > 0 && *a > i64::MAX / *b)
                    || (*a > 0 && *b < 0 && *b < i64::MIN / *a)
                    || (*a < 0 && *b > 0 && *a < i64::MIN / *b)
                    || (*a < 0 && *b < 0 && *a < i64::MAX / *b)
                {
                    Ok(Value::Float(*a as f64 * *b as f64))
                } else {
                    Ok(Value::Integer(a * b))
                }
            } else {
                Ok(Value::Integer(0))
            }
        }
        (Value::Integer(a), BinaryOperator::Div, Value::Integer(b)) => {
            if *b == 0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Integer(a / b))
            }
        }
        (Value::Integer(a), BinaryOperator::Mod, Value::Integer(b)) => {
            if *b == 0 {
                Err("Modulo by zero".to_string())
            } else {
                Ok(Value::Integer(a % b))
            }
        }

        (Value::Integer(a), BinaryOperator::Eq, Value::Integer(b)) => Ok(Value::Boolean(a == b)),
        (Value::Integer(a), BinaryOperator::NotEq, Value::Integer(b)) => Ok(Value::Boolean(a != b)),
        (Value::Integer(a), BinaryOperator::Lt, Value::Integer(b)) => Ok(Value::Boolean(a < b)),
        (Value::Integer(a), BinaryOperator::LtEq, Value::Integer(b)) => Ok(Value::Boolean(a <= b)),
        (Value::Integer(a), BinaryOperator::Gt, Value::Integer(b)) => Ok(Value::Boolean(a > b)),
        (Value::Integer(a), BinaryOperator::GtEq, Value::Integer(b)) => Ok(Value::Boolean(a >= b)),

        (Value::Boolean(a), BinaryOperator::And, Value::Boolean(b)) => Ok(Value::Boolean(*a && *b)),
        (Value::Boolean(a), BinaryOperator::Or, Value::Boolean(b)) => Ok(Value::Boolean(*a || *b)),

        (Value::String(a), BinaryOperator::Eq, Value::String(b)) => Ok(Value::Boolean(a == b)),
        (Value::String(a), BinaryOperator::NotEq, Value::String(b)) => Ok(Value::Boolean(a != b)),
        (Value::String(a), BinaryOperator::Lt, Value::String(b)) => Ok(Value::Boolean(a < b)),
        (Value::String(a), BinaryOperator::LtEq, Value::String(b)) => Ok(Value::Boolean(a <= b)),
        (Value::String(a), BinaryOperator::Gt, Value::String(b)) => Ok(Value::Boolean(a > b)),
        (Value::String(a), BinaryOperator::GtEq, Value::String(b)) => Ok(Value::Boolean(a >= b)),

        (Value::String(a), BinaryOperator::Add, Value::String(b)) => {
            Ok(Value::String(format!("{}{}", a, b)))
        }

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

        (Value::Integer(a), BinaryOperator::Add, Value::Float(b)) => {
            Ok(Value::Float(*a as f64 + b))
        }
        (Value::Float(a), BinaryOperator::Add, Value::Integer(b)) => {
            Ok(Value::Float(a + *b as f64))
        }
        (Value::Integer(a), BinaryOperator::Sub, Value::Float(b)) => {
            Ok(Value::Float(*a as f64 - b))
        }
        (Value::Float(a), BinaryOperator::Sub, Value::Integer(b)) => {
            Ok(Value::Float(a - *b as f64))
        }
        (Value::Integer(a), BinaryOperator::Mul, Value::Float(b)) => {
            Ok(Value::Float(*a as f64 * b))
        }
        (Value::Float(a), BinaryOperator::Mul, Value::Integer(b)) => {
            Ok(Value::Float(a * *b as f64))
        }
        (Value::Integer(a), BinaryOperator::Div, Value::Float(b)) => {
            if *b == 0.0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Float(*a as f64 / b))
            }
        }
        (Value::Float(a), BinaryOperator::Div, Value::Integer(b)) => {
            if *b == 0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Float(a / *b as f64))
            }
        }

        (Value::Boolean(a), BinaryOperator::Eq, Value::Boolean(b)) => Ok(Value::Boolean(a == b)),
        (Value::Boolean(a), BinaryOperator::NotEq, Value::Boolean(b)) => Ok(Value::Boolean(a != b)),

        (Value::List(a), BinaryOperator::Add, Value::List(b)) => {
            let mut result = a.clone();
            result.extend(b.iter().cloned());
            Ok(Value::List(result))
        }

        (Value::Float(a), BinaryOperator::Eq, Value::Float(b)) => Ok(Value::Boolean(a == b)),
        (Value::Float(a), BinaryOperator::NotEq, Value::Float(b)) => Ok(Value::Boolean(a != b)),
        (Value::Float(a), BinaryOperator::Lt, Value::Float(b)) => Ok(Value::Boolean(a < b)),
        (Value::Float(a), BinaryOperator::LtEq, Value::Float(b)) => Ok(Value::Boolean(a <= b)),
        (Value::Float(a), BinaryOperator::Gt, Value::Float(b)) => Ok(Value::Boolean(a > b)),
        (Value::Float(a), BinaryOperator::GtEq, Value::Float(b)) => Ok(Value::Boolean(a >= b)),

        (Value::Integer(a), BinaryOperator::Eq, Value::Float(b)) => {
            Ok(Value::Boolean(*a as f64 == *b))
        }
        (Value::Float(a), BinaryOperator::Eq, Value::Integer(b)) => {
            Ok(Value::Boolean(*a == *b as f64))
        }
        (Value::Integer(a), BinaryOperator::NotEq, Value::Float(b)) => {
            Ok(Value::Boolean(*a as f64 != *b))
        }
        (Value::Float(a), BinaryOperator::NotEq, Value::Integer(b)) => {
            Ok(Value::Boolean(*a != *b as f64))
        }
        (Value::Integer(a), BinaryOperator::Lt, Value::Float(b)) => {
            Ok(Value::Boolean((*a as f64) < *b))
        }
        (Value::Float(a), BinaryOperator::Lt, Value::Integer(b)) => {
            Ok(Value::Boolean(*a < *b as f64))
        }
        (Value::Integer(a), BinaryOperator::LtEq, Value::Float(b)) => {
            Ok(Value::Boolean((*a as f64) <= *b))
        }
        (Value::Float(a), BinaryOperator::LtEq, Value::Integer(b)) => {
            Ok(Value::Boolean(*a <= *b as f64))
        }
        (Value::Integer(a), BinaryOperator::Gt, Value::Float(b)) => {
            Ok(Value::Boolean((*a as f64) > *b))
        }
        (Value::Float(a), BinaryOperator::Gt, Value::Integer(b)) => {
            Ok(Value::Boolean(*a > *b as f64))
        }
        (Value::Integer(a), BinaryOperator::GtEq, Value::Float(b)) => {
            Ok(Value::Boolean((*a as f64) >= *b))
        }
        (Value::Float(a), BinaryOperator::GtEq, Value::Integer(b)) => {
            Ok(Value::Boolean(*a >= *b as f64))
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

fn gcd(mut m: i64, mut n: i64) -> i64 {
    while n != 0 {
        let temp = n;
        n = m % n;
        m = temp;
    }
    m.abs()
}

fn factorial(n: i64) -> i64 {
    if n > 20 {
        i64::MAX
    } else if n < 0 {
        0
    } else {
        (1..=n).product()
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
