use crate::error::{PseudoError, SourceTracker};
use crate::parser::{AstNode, BinaryOperator, UnaryOperator};
use rand::Rng;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct NodePosition {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct NodePositions {
    positions: HashMap<String, NodePosition>,
}

impl NodePositions {
    fn new() -> Self {
        Self {
            positions: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    fn track_node(&mut self, node_id: String, position: NodePosition) {
        self.positions.insert(node_id, position);
    }

    #[allow(dead_code)]
    fn get_position(&self, node_id: &str) -> Option<&NodePosition> {
        self.positions.get(node_id)
    }
}

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

#[derive(Clone)]
struct Environment {
    variables: HashMap<String, Value>,
    procedures: HashMap<String, (Vec<String>, AstNode)>,
    classes: HashMap<String, AstNode>,
    output: String,
    source_tracker: Option<SourceTracker>,
    return_value: Option<Value>,
    parent: Option<Rc<RefCell<Environment>>>,
    node_positions: NodePositions,
}

impl Environment {
    fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            procedures: HashMap::new(),
            classes: HashMap::new(),
            output: String::new(),
            source_tracker: None,
            return_value: None,
            parent: None,
            node_positions: NodePositions::new(),
        }
    }

    fn new_with_parent(parent: Rc<RefCell<Environment>>) -> Self {
        Environment {
            variables: HashMap::new(),
            procedures: parent.borrow().procedures.clone(),
            classes: parent.borrow().classes.clone(),
            output: String::new(),
            source_tracker: parent.borrow().source_tracker.clone(),
            return_value: None,
            parent: Some(Rc::clone(&parent)),
            node_positions: parent.borrow().node_positions.clone(),
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

    fn get_procedure(&self, name: &str) -> Option<(Vec<String>, AstNode)> {
        self.procedures.get(name).cloned()
    }

    #[allow(dead_code)]
    fn append_to_list(&mut self, name: &str, value: Value) -> Result<Value, PseudoError> {
        if let Some(Value::List(mut elements)) = self.get(name) {
            elements.push(value.clone());
            self.set(name.to_string(), Value::List(elements.clone()));
            Ok(Value::List(elements))
        } else {
            Err(PseudoError::new(&format!(
                "Variable {} is not a list",
                name
            )))
        }
    }

    fn set_source_tracker(&mut self, source: &str) {
        self.source_tracker = Some(SourceTracker::new(source));
    }

    #[allow(dead_code)]
    fn create_error(&self, message: &str, pos: Option<usize>) -> PseudoError {
        if let Some(ref source_tracker) = self.source_tracker {
            if let Some(position) = pos {
                source_tracker.create_error(message, position)
            } else {
                PseudoError::new(message)
            }
        } else {
            PseudoError::new(message)
        }
    }

    fn create_smart_error(&self, message: &str) -> PseudoError {
        if let Some(ref source_tracker) = self.source_tracker {
            source_tracker.create_smart_error(message)
        } else {
            PseudoError::new(message)
        }
    }
}

const MAX_STACK_DEPTH: usize = 10000000;
static mut CURRENT_STACK_DEPTH: usize = 0;

#[allow(dead_code)]
pub fn run(ast: AstNode) -> Result<String, String> {
    let env = Rc::new(RefCell::new(Environment::new()));
    let debug = false;
    if debug {
        println!("Starting interpreter...");
    }
    let _result = evaluate_node(&ast, Rc::clone(&env), debug)?;
    if debug {
        println!("Execution completed");
    }
    let output = env.borrow().output.clone();
    Ok(output)
}

pub fn run_with_source(ast: AstNode, source: &str) -> Result<String, PseudoError> {
    let env = Rc::new(RefCell::new(Environment::new()));
    env.borrow_mut().set_source_tracker(source);
    let debug = false;

    match evaluate_node(&ast, Rc::clone(&env), debug) {
        Ok(_) => Ok(env.borrow().output.clone()),
        Err(err) => Err(env.borrow().create_smart_error(&err)),
    }
}

fn evaluate_node(
    node: &AstNode,
    env: Rc<RefCell<Environment>>,
    debug: bool,
) -> Result<Value, String> {
    unsafe {
        if CURRENT_STACK_DEPTH > MAX_STACK_DEPTH {
            return Err("Stack overflow: maximum recursion depth exceeded".to_string());
        }
    }

    if debug {
        println!("Evaluating node: {:?}", node);
    }
    match node {
        AstNode::Program(statements) => {
            let mut last_value = Value::Unit;
            for stmt in statements {
                last_value = evaluate_node(stmt, Rc::clone(&env), debug)?;
            }
            Ok(last_value)
        }

        AstNode::Block(statements) => {
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
        AstNode::List(elements) => {
            let mut values = Vec::new();
            for elem in elements {
                values.push(evaluate_node(elem, Rc::clone(&env), debug)?);
            }
            Ok(Value::List(values))
        }

        AstNode::Identifier(name) => {
            let result = env.borrow().get(name);
            if result.is_none() {
                Err(format!("Undefined variable: {}", name))
            } else {
                Ok(result.unwrap())
            }
        }

        AstNode::Assignment(target, value) => {
            let val = evaluate_node(value, Rc::clone(&env), debug)?;
            if let AstNode::Identifier(name) = &**target {
                if debug {
                    println!("Assigning {} = {:?}", name, val);
                }
                if matches!(&**value, AstNode::FormattedString(_, _)) {
                    let output = value_to_string(&val);
                    env.borrow_mut().output.push_str(&output);
                    env.borrow_mut().output.push('\n');
                }
                env.borrow_mut().set(name.clone(), val.clone());
                Ok(val)
            } else {
                Err("Invalid assignment target".to_string())
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
                        Err("Right operand of AND must be boolean".to_string())
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
                        Err("Right operand of OR must be boolean".to_string())
                    }
                }
            }
            _ => {
                let left_val = evaluate_node(left_expr, Rc::clone(&env), debug)?;
                let right_val = evaluate_node(right_expr, Rc::clone(&env), debug)?;
                evaluate_binary_op(&left_val, op, &right_val)
            }
        },

        AstNode::UnaryOp(op, expr) => {
            let val = evaluate_node(expr, Rc::clone(&env), debug)?;
            evaluate_unary_op(op, &val)
        }

        AstNode::If(condition, then_branch, else_branch) => {
            let cond_val = evaluate_node(condition, Rc::clone(&env), debug)?;
            match cond_val {
                Value::Boolean(true) => evaluate_node(then_branch, Rc::clone(&env), debug),
                Value::Boolean(false) => {
                    if let Some(else_branch) = else_branch {
                        evaluate_node(else_branch, Rc::clone(&env), debug)
                    } else {
                        Ok(Value::Unit)
                    }
                }
                _ => Err("Condition must be a boolean".to_string()),
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
                Err("REPEAT count must be an integer".to_string())
            }
        }

        AstNode::Display(expr) => {
            if let Some(expr) = expr {
                let result = evaluate_node(expr, Rc::clone(&env), debug)?;
                let output = value_to_string(&result);
                if !std::thread::current()
                    .name()
                    .map_or(false, |name| name.starts_with("test"))
                {
                    println!("{}", output);
                    io::stdout().flush().unwrap();
                }
                env.borrow_mut().output.push_str(&output);
                env.borrow_mut().output.push('\n');
                Ok(result)
            } else {
                if !std::thread::current()
                    .name()
                    .map_or(false, |name| name.starts_with("test"))
                {
                    println!();
                }
                env.borrow_mut().output.push('\n');
                Ok(Value::Unit)
            }
        }

        AstNode::DisplayInline(expr) => {
            let value = evaluate_node(expr, Rc::clone(&env), debug)?;
            let output = value_to_string(&value);
            if !std::thread::current()
                .name()
                .map_or(false, |name| name.starts_with("test"))
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
                    .map_err(|e| e.to_string())?;
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
                Err("INPUT is not supported in this environment".to_string())
            }
        }

        AstNode::ProcedureDecl(name, params, body) => {
            env.borrow_mut()
                .procedures
                .insert(name.clone(), (params.clone(), (**body).clone()));
            Ok(Value::Unit)
        }

        AstNode::ProcedureCall(name, args) => {
            unsafe {
                CURRENT_STACK_DEPTH += 1;
            }

            let result = match name.as_str() {
                "SLEEP" => {
                    if args.len() != 1 {
                        return Err("SLEEP requires one argument".to_string());
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
                            _ => Err("SLEEP requires a numeric argument".to_string()),
                        }
                    }

                    #[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
                    {
                        let _seconds = evaluate_node(&args[0], Rc::clone(&env), debug)?;

                        log("SLEEP function is not fully supported in WebAssembly. The program will continue without pausing.");

                        Ok(Value::Unit)
                    }

                    #[cfg(all(target_arch = "wasm32", feature = "wasi"))]
                    {
                        Err("SLEEP is not supported in this environment".to_string())
                    }
                }
                "CONCAT" => {
                    if args.len() != 2 {
                        return Err("CONCAT requires two arguments".to_string());
                    }
                    let s1 = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    let s2 = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                    if let (Value::String(a), Value::String(b)) = (s1, s2) {
                        Ok(Value::String(format!("{}{}", a, b)))
                    } else {
                        Err("CONCAT requires string arguments".to_string())
                    }
                }
                "SUBSTRING" => {
                    if args.len() != 3 {
                        return Err("SUBSTRING requires three arguments".to_string());
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
                            Err("Invalid substring indices".to_string())
                        }
                    } else {
                        Err("Invalid substring arguments".to_string())
                    }
                }
                "LENGTH" => {
                    if args.len() != 1 {
                        return Err("LENGTH requires one argument".to_string());
                    }
                    let arg = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match arg {
                        Value::List(elements) => Ok(Value::Integer(elements.len() as i64)),
                        Value::String(s) => Ok(Value::Integer(s.len() as i64)),
                        _ => Err("LENGTH requires a list or string argument".to_string()),
                    }
                }
                "REMOVE" => {
                    if args.len() != 2 {
                        return Err("REMOVE requires two arguments".to_string());
                    }
                    evaluate_node(
                        &AstNode::Remove(Box::new(args[0].clone()), Box::new(args[1].clone())),
                        env,
                        debug,
                    )
                }
                "APPEND" => {
                    if args.len() != 2 {
                        return Err("APPEND requires two arguments".to_string());
                    }
                    evaluate_node(
                        &AstNode::Append(Box::new(args[0].clone()), Box::new(args[1].clone())),
                        env,
                        debug,
                    )
                }
                "INSERT" => {
                    if args.len() != 3 {
                        return Err("INSERT requires three arguments".to_string());
                    }
                    evaluate_node(
                        &AstNode::Insert(
                            Box::new(args[0].clone()),
                            Box::new(args[1].clone()),
                            Box::new(args[2].clone()),
                        ),
                        env,
                        debug,
                    )
                }
                "ABS" => {
                    if args.len() != 1 {
                        return Err("ABS requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Integer(n) => Ok(Value::Integer(n.abs())),
                        Value::Float(f) => Ok(Value::Float(f.abs())),
                        _ => Err("ABS requires a numeric argument".to_string()),
                    }
                }
                "CEIL" => {
                    if args.len() != 1 {
                        return Err("CEIL requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Integer(f.ceil() as i64)),
                        Value::Integer(n) => Ok(Value::Integer(n)),
                        _ => Err("CEIL requires a numeric argument".to_string()),
                    }
                }
                "FLOOR" => {
                    if args.len() != 1 {
                        return Err("FLOOR requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Integer(f.floor() as i64)),
                        Value::Integer(n) => Ok(Value::Integer(n)),
                        _ => Err("FLOOR requires a numeric argument".to_string()),
                    }
                }
                "POW" => {
                    if args.len() != 2 {
                        return Err("POW requires two arguments".to_string());
                    }
                    let base = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    let exponent = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                    match (base, exponent) {
                        (Value::Integer(a), Value::Integer(b)) => {
                            Ok(Value::Float((a as f64).powi(b as i32)))
                        }
                        (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a.powi(b as i32))),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(b))),
                        (Value::Integer(a), Value::Float(b)) => {
                            Ok(Value::Float((a as f64).powf(b)))
                        }
                        _ => Err("POW requires numeric arguments".to_string()),
                    }
                }
                "SQRT" => {
                    if args.len() != 1 {
                        return Err("SQRT requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Integer(n) => Ok(Value::Float((n as f64).sqrt())),
                        Value::Float(f) => Ok(Value::Float(f.sqrt())),
                        _ => Err("SQRT requires a numeric argument".to_string()),
                    }
                }
                "SIN" => {
                    if args.len() != 1 {
                        return Err("SIN requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Float(f.sin())),
                        Value::Integer(n) => Ok(Value::Float((n as f64).sin())),
                        _ => Err("SIN requires a numeric argument".to_string()),
                    }
                }
                "COS" => {
                    if args.len() != 1 {
                        return Err("COS requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Float(f.cos())),
                        Value::Integer(n) => Ok(Value::Float((n as f64).cos())),
                        _ => Err("COS requires a numeric argument".to_string()),
                    }
                }
                "TAN" => {
                    if args.len() != 1 {
                        return Err("TAN requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Float(f.tan())),
                        Value::Integer(n) => Ok(Value::Float((n as f64).tan())),
                        _ => Err("TAN requires a numeric argument".to_string()),
                    }
                }
                "ASIN" => {
                    if args.len() != 1 {
                        return Err("ASIN requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Float(f.asin())),
                        Value::Integer(n) => Ok(Value::Float((n as f64).asin())),
                        _ => Err("ASIN requires a numeric argument".to_string()),
                    }
                }
                "ACOS" => {
                    if args.len() != 1 {
                        return Err("ACOS requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Float(f.acos())),
                        Value::Integer(n) => Ok(Value::Float((n as f64).acos())),
                        _ => Err("ACOS requires a numeric argument".to_string()),
                    }
                }
                "ATAN" => {
                    if args.len() != 1 {
                        return Err("ATAN requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Float(f.atan())),
                        Value::Integer(n) => Ok(Value::Float((n as f64).atan())),
                        _ => Err("ATAN requires a numeric argument".to_string()),
                    }
                }
                "EXP" => {
                    if args.len() != 1 {
                        return Err("EXP requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Float(f.exp())),
                        Value::Integer(n) => Ok(Value::Float((n as f64).exp())),
                        _ => Err("EXP requires a numeric argument".to_string()),
                    }
                }
                "LOG" => {
                    if args.len() != 1 {
                        return Err("LOG requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Float(f.ln())),
                        Value::Integer(n) => Ok(Value::Float((n as f64).ln())),
                        _ => Err("LOG requires a numeric argument".to_string()),
                    }
                }
                "LOGTEN" => {
                    if args.len() != 1 {
                        return Err("LOGTEN requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Float(f.log10())),
                        Value::Integer(n) => Ok(Value::Float((n as f64).log10())),
                        _ => Err("LOGTEN requires a numeric argument".to_string()),
                    }
                }
                "LOGTWO" => {
                    if args.len() != 1 {
                        return Err("LOGTWO requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Float(f.log2())),
                        Value::Integer(n) => Ok(Value::Float((n as f64).log2())),
                        _ => Err("LOGTWO requires a numeric argument".to_string()),
                    }
                }
                "GCD" => {
                    if args.len() != 2 {
                        return Err("GCD requires two arguments".to_string());
                    }
                    let a = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    let b = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                    match (a, b) {
                        (Value::Integer(m), Value::Integer(n)) => Ok(Value::Integer(gcd(m, n))),
                        _ => Err("GCD requires integer arguments".to_string()),
                    }
                }
                "FACTORIAL" => {
                    if args.len() != 1 {
                        return Err("FACTORIAL requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    if let Value::Integer(n) = x {
                        Ok(Value::Integer(factorial(n)))
                    } else {
                        Err("FACTORIAL requires an integer argument".to_string())
                    }
                }
                "DEGREES" => {
                    if args.len() != 1 {
                        return Err("DEGREES requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Float(f.to_degrees())),
                        Value::Integer(n) => Ok(Value::Float((n as f64).to_degrees())),
                        _ => Err("DEGREES requires a numeric argument".to_string()),
                    }
                }
                "RADIANS" => {
                    if args.len() != 1 {
                        return Err("RADIANS requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Float(f.to_radians())),
                        Value::Integer(n) => Ok(Value::Float((n as f64).to_radians())),
                        _ => Err("RADIANS requires a numeric argument".to_string()),
                    }
                }
                "HYPOT" => {
                    if args.len() != 2 {
                        return Err("HYPOT requires two arguments".to_string());
                    }
                    let a = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    let b = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                    match (a, b) {
                        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.hypot(y))),
                        (Value::Integer(x), Value::Float(y)) => {
                            Ok(Value::Float((x as f64).hypot(y)))
                        }
                        (Value::Float(x), Value::Integer(y)) => Ok(Value::Float(x.hypot(y as f64))),
                        (Value::Integer(x), Value::Integer(y)) => {
                            Ok(Value::Float((x as f64).hypot(y as f64)))
                        }
                        _ => Err("HYPOT requires numeric arguments".to_string()),
                    }
                }
                "MIN" => {
                    if args.len() != 2 {
                        return Err("MIN requires two arguments".to_string());
                    }
                    let a = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    let b = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                    match (a, b) {
                        (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(x.min(y))),
                        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.min(y))),
                        (Value::Integer(x), Value::Float(y)) => Ok(Value::Float((x as f64).min(y))),
                        (Value::Float(x), Value::Integer(y)) => Ok(Value::Float(x.min(y as f64))),
                        _ => Err("MIN requires two numeric arguments".to_string()),
                    }
                }
                "MAX" => {
                    if args.len() != 2 {
                        return Err("MAX requires two arguments".to_string());
                    }
                    let a = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    let b = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                    match (a, b) {
                        (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(x.max(y))),
                        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.max(y))),
                        (Value::Integer(x), Value::Float(y)) => Ok(Value::Float((x as f64).max(y))),
                        (Value::Float(x), Value::Integer(y)) => Ok(Value::Float(x.max(y as f64))),
                        _ => Err("MAX requires two numeric arguments".to_string()),
                    }
                }
                "EXIT" => {
                    std::process::exit(0);
                }
                "ROUND" => {
                    if args.len() != 1 {
                        return Err("ROUND requires one argument".to_string());
                    }
                    let x = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    match x {
                        Value::Float(f) => Ok(Value::Integer(f.round() as i64)),
                        Value::Integer(n) => Ok(Value::Integer(n)),
                        _ => Err("ROUND requires a numeric argument".to_string()),
                    }
                }
                "SPLIT" => {
                    if args.len() != 2 {
                        return Err("SPLIT requires two arguments".to_string());
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
                        _ => Err("SPLIT requires two string arguments".to_string()),
                    }
                }
                "TRIM" => {
                    if args.len() != 1 {
                        return Err("TRIM requires one argument".to_string());
                    }
                    let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    if let Value::String(s) = str_val {
                        Ok(Value::String(s.trim().to_string()))
                    } else {
                        Err("TRIM requires a string argument".to_string())
                    }
                }
                "REPLACE" => {
                    if args.len() != 3 {
                        return Err("REPLACE requires three arguments".to_string());
                    }
                    let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    let from_val = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                    let to_val = evaluate_node(&args[2], Rc::clone(&env), debug)?;
                    match (str_val, from_val, to_val) {
                        (Value::String(s), Value::String(from), Value::String(to)) => {
                            Ok(Value::String(s.replace(&from, &to)))
                        }
                        _ => Err("REPLACE requires three string arguments".to_string()),
                    }
                }
                "UPPERCASE" => {
                    if args.len() != 1 {
                        return Err("UPPERCASE requires one argument".to_string());
                    }
                    let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    if let Value::String(s) = str_val {
                        Ok(Value::String(s.to_uppercase()))
                    } else {
                        Err("UPPERCASE requires a string argument".to_string())
                    }
                }
                "LOWERCASE" => {
                    if args.len() != 1 {
                        return Err("LOWERCASE requires one argument".to_string());
                    }
                    let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    if let Value::String(s) = str_val {
                        Ok(Value::String(s.to_lowercase()))
                    } else {
                        Err("LOWERCASE requires a string argument".to_string())
                    }
                }
                "TIMESTAMP" => match args.len() {
                    0 => {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map_err(|e| e.to_string())?;
                            let secs = now.as_secs() as f64;
                            let nanos = now.subsec_nanos() as f64 / 1_000_000_000.0;
                            Ok(Value::Float(secs + nanos))
                        }

                        // cool high precision fix for browser
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
                            Err("TIMESTAMP is not supported in this environment".to_string())
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
                                    Err(e) => Err(format!("Invalid datetime format: {}", e)),
                                }
                            } else {
                                Err("TIMESTAMP requires a datetime string".to_string())
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
                                _ => Err("TIME requires a numeric timestamp".to_string()),
                            }
                        }

                        #[cfg(all(target_arch = "wasm32", feature = "wasi"))]
                        {
                            Err("TIMESTAMP is not supported in this environment".to_string())
                        }
                    }
                    _ => Err("TIMESTAMP requires 0 or 1 arguments".to_string()),
                },
                "TIME" => {
                    if args.len() != 1 {
                        return Err("TIME requires one argument".to_string());
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
                                    .ok_or("Invalid timestamp")?;
                                Ok(Value::String(dt.naive_local().to_string()))
                            }
                            Value::Float(ts) => {
                                use chrono::{TimeZone, Utc};
                                let secs = ts.floor() as i64;
                                let nanos = ((ts - ts.floor()) * 1_000_000_000.0) as u32;
                                let dt = Utc
                                    .timestamp_opt(secs, nanos)
                                    .single()
                                    .ok_or("Invalid timestamp")?;
                                Ok(Value::String(dt.naive_local().to_string()))
                            }
                            _ => Err("TIME requires a numeric timestamp".to_string()),
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
                            _ => Err("TIME requires a numeric timestamp".to_string()),
                        }
                    }

                    #[cfg(all(target_arch = "wasm32", feature = "wasi"))]
                    {
                        Err("TIME is not supported in this environment".to_string())
                    }
                }
                "TIMEZONE" => {
                    if args.len() != 2 {
                        return Err(
                            "TIMEZONE requires two arguments: timestamp and timezone".to_string()
                        );
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
                                .ok_or("Invalid timestamp")?,
                            Value::Float(ts) => {
                                let secs = ts.floor() as i64;
                                let nanos = ((ts - ts.floor()) * 1_000_000_000.0) as u32;
                                Utc.timestamp_opt(secs, nanos)
                                    .single()
                                    .ok_or("Invalid timestamp")?
                            }
                            _ => return Err("TIMEZONE requires a numeric timestamp".to_string()),
                        };

                        let tz: Tz = tz
                            .parse()
                            .map_err(|_| format!("Invalid timezone: {}", tz))?;

                        let dt_tz = dt_utc.with_timezone(&tz);
                        Ok(Value::String(dt_tz.naive_local().to_string()))
                    } else {
                        Err("TIMEZONE requires a timezone name (string)".to_string())
                    }
                }
                "TIMEZONES" => {
                    if !args.is_empty() {
                        return Err("TIMEZONES takes no arguments".to_string());
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
                        return Err("MILLITIME takes no arguments".to_string());
                    }
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| e.to_string())?;
                    let millis = now.as_millis();
                    // Cap at i64::MAX to avoid overflow
                    let millis = std::cmp::min(millis, i64::MAX as u128) as i64;
                    Ok(Value::Integer(millis))
                }
                "CONTAINS" => {
                    if args.len() != 2 {
                        return Err("CONTAINS requires two arguments".to_string());
                    }
                    let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    let text_val = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                    match (str_val, text_val) {
                        (Value::String(s), Value::String(t)) => Ok(Value::Boolean(s.contains(&t))),
                        _ => Err("CONTAINS requires two string arguments".to_string()),
                    }
                }
                "FIND" => {
                    if args.len() != 2 {
                        return Err("FIND requires two arguments".to_string());
                    }
                    let str_val = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    let text_val = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                    match (str_val, text_val) {
                        (Value::String(s), Value::String(t)) => match s.find(&t) {
                            Some(index) => Ok(Value::Integer((index + 1) as i64)),
                            None => Ok(Value::Integer(-1)),
                        },
                        _ => Err("FIND requires two string arguments".to_string()),
                    }
                }
                "RANGE" => match args.len() {
                    1 => {
                        let end = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                        if let Value::Integer(end_val) = end {
                            if end_val < 1 {
                                return Err("RANGE end value must be greater than 0".to_string());
                            }
                            let list: Vec<Value> =
                                (1..=end_val).map(|i| Value::Integer(i)).collect();
                            Ok(Value::List(list))
                        } else {
                            Err("RANGE requires integer arguments".to_string())
                        }
                    }
                    2 => {
                        let start = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                        let end = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                        if let (Value::Integer(start_val), Value::Integer(end_val)) = (start, end) {
                            if end_val < start_val {
                                return Err(
                                    "RANGE end value must be greater than or equal to start value"
                                        .to_string(),
                                );
                            }
                            let list: Vec<Value> =
                                (start_val..=end_val).map(|i| Value::Integer(i)).collect();
                            Ok(Value::List(list))
                        } else {
                            Err("RANGE requires integer arguments".to_string())
                        }
                    }
                    _ => Err("RANGE requires one or two arguments".to_string()),
                },
                "STARTSWITH" => {
                    if args.len() != 2 {
                        return Err("STARTSWITH requires two arguments".to_string());
                    }
                    let fullstring = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    let substring = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                    match (fullstring, substring) {
                        (Value::String(s), Value::String(sub)) => {
                            Ok(Value::Boolean(s.starts_with(&sub)))
                        }
                        _ => Err("STARTSWITH requires two string arguments".to_string()),
                    }
                }
                "ENDSWITH" => {
                    if args.len() != 2 {
                        return Err("ENDSWITH requires two arguments".to_string());
                    }
                    let fullstring = evaluate_node(&args[0], Rc::clone(&env), debug)?;
                    let substring = evaluate_node(&args[1], Rc::clone(&env), debug)?;
                    match (fullstring, substring) {
                        (Value::String(s), Value::String(sub)) => {
                            Ok(Value::Boolean(s.ends_with(&sub)))
                        }
                        _ => Err("ENDSWITH requires two string arguments".to_string()),
                    }
                }
                _ => {
                    let procedure = env
                        .borrow()
                        .get_procedure(name)
                        .ok_or_else(|| format!("Procedure '{}' not found", name))?;

                    let local_env =
                        Rc::new(RefCell::new(Environment::new_with_parent(Rc::clone(&env))));

                    let (params, _) = procedure.clone();
                    for (param, arg) in params.iter().zip(args) {
                        let arg_value = evaluate_node(arg, Rc::clone(&env), debug)?;
                        local_env.borrow_mut().set(param.clone(), arg_value);
                    }

                    let (_, body) = procedure;
                    let result = match evaluate_node(&body, Rc::clone(&local_env), debug) {
                        Err(e) if e == "Return" => local_env
                            .borrow()
                            .return_value
                            .clone()
                            .unwrap_or(Value::Unit),
                        Ok(val) => {
                            if let Some(return_value) = local_env.borrow().return_value.clone() {
                                return_value
                            } else {
                                val
                            }
                        }
                        Err(e) => return Err(e),
                    };

                    env.borrow_mut().output.push_str(&local_env.borrow().output);
                    Ok(result)
                }
            };

            unsafe {
                if CURRENT_STACK_DEPTH > 0 {
                    CURRENT_STACK_DEPTH -= 1;
                }
            }

            result
        }
        AstNode::ListAccess(list, index) => {
            let current_value = evaluate_node(list, Rc::clone(&env), debug)?;
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;

            match (current_value, index_val) {
                (Value::List(elements), Value::Integer(i)) => {
                    let idx = i - 1;
                    if idx < 0 {
                        Err("List index out of bounds: index cannot be less than 1".to_string())
                    } else if (idx as usize) >= elements.len() {
                        Err(format!(
                            "List index out of bounds: {} (size: {})",
                            i,
                            elements.len()
                        ))
                    } else {
                        Ok(elements[idx as usize].clone())
                    }
                }
                (Value::String(s), Value::Integer(i)) => {
                    let idx = i - 1;
                    if idx < 0 {
                        Err("String index out of bounds: index cannot be less than 1".to_string())
                    } else if (idx as usize) >= s.len() {
                        Err(format!(
                            "String index out of bounds: {} (size: {})",
                            i,
                            s.len()
                        ))
                    } else {
                        let ch = s.chars().nth(idx as usize).ok_or("Invalid string index")?;
                        Ok(Value::String(ch.to_string()))
                    }
                }
                _ => Err(
                    "Invalid index access - expected list or string and integer index".to_string(),
                ),
            }
        }

        AstNode::ListAssignment(list, index, value) => {
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;
            let new_val = evaluate_node(value, Rc::clone(&env), debug)?;

            if let AstNode::Identifier(name) = &**list {
                let elements = if let Some(Value::List(elements)) = env.borrow().get(name) {
                    elements
                } else {
                    return Err(format!("Variable {} is not a list", name));
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
                        Err("List index out of bounds".to_string())
                    }
                } else {
                    Err("Invalid list index".to_string())
                }
            } else {
                if let AstNode::ListAccess(inner_list, inner_index) = &**list {
                    let list_val = evaluate_node(inner_list, Rc::clone(&env), debug)?;
                    let index_inner = evaluate_node(inner_index, Rc::clone(&env), debug)?;

                    if let (Value::List(mut elements), Value::Integer(i)) = (list_val, index_inner)
                    {
                        let idx = i - 1;
                        if idx >= 0 && (idx as usize) < elements.len() {
                            if let Value::Integer(j) = index_val {
                                let jdx = j - 1;
                                if let Value::List(mut inner_elements) =
                                    elements[idx as usize].clone()
                                {
                                    if jdx >= 0 && (jdx as usize) < inner_elements.len() {
                                        inner_elements[jdx as usize] = new_val.clone();
                                        elements[idx as usize] = Value::List(inner_elements);

                                        if let AstNode::Identifier(name) = &**inner_list {
                                            env.borrow_mut()
                                                .set(name.clone(), Value::List(elements));
                                            return Ok(new_val);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err("Invalid nested list assignment".to_string())
                } else {
                    Err("Invalid list assignment target".to_string())
                }
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
                    Err("Invalid substring indices".to_string())
                }
            } else {
                Err("Invalid substring arguments".to_string())
            }
        }

        AstNode::Concat(str1, str2) => {
            let s1 = evaluate_node(str1, Rc::clone(&env), debug)?;
            let s2 = evaluate_node(str2, Rc::clone(&env), debug)?;
            if let (Value::String(s1), Value::String(s2)) = (s1, s2) {
                Ok(Value::String(format!("{}{}", s1, s2)))
            } else {
                Err("CONCAT requires string arguments".to_string())
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
                    Err("Cannot convert string to number".to_string())
                }
            } else {
                Err("TONUM requires string argument".to_string())
            }
        }

        AstNode::RepeatUntil(body, condition) => {
            let max_iterations = 1000000;
            let mut iterations = 0;

            loop {
                iterations += 1;
                if iterations > max_iterations {
                    return Err("Maximum loop iterations exceeded".to_string());
                }

                let result = evaluate_node(body, Rc::clone(&env), debug)?;

                if env.borrow().return_value.is_some() {
                    return Ok(result);
                }

                let cond_val = evaluate_node(condition, Rc::clone(&env), debug)?;
                match cond_val {
                    Value::Boolean(true) => break,
                    Value::Boolean(false) => continue,
                    _ => return Err("REPEAT UNTIL condition must evaluate to boolean".to_string()),
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
                _ => Err("FOR EACH requires list or string".to_string()),
            }
        }

        AstNode::RawString(s) => Ok(Value::String(s.clone())),

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
                _ => Err("LENGTH requires a list or string argument".to_string()),
            }
        }

        AstNode::ListInsert(list, index, value) | AstNode::Insert(list, index, value) => {
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;
            let insert_val = evaluate_node(value, Rc::clone(&env), debug)?;

            if let AstNode::Identifier(name) = &**list {
                let elements = if let Some(Value::List(elements)) = env.borrow().get(name) {
                    elements
                } else {
                    return Err(format!("Variable {} is not a list", name));
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
                        Err("List index out of bounds".to_string())
                    }
                } else {
                    Err("Invalid list index".to_string())
                }
            } else {
                Err("INSERT requires a list variable".to_string())
            }
        }

        AstNode::ListAppend(list, value) | AstNode::Append(list, value) => {
            let append_val = evaluate_node(value, Rc::clone(&env), debug)?;

            if let AstNode::Identifier(name) = &**list {
                let elements = if let Some(Value::List(elements)) = env.borrow().get(name) {
                    elements
                } else {
                    return Err(format!("Variable {} is not a list", name));
                };

                let mut new_elements = elements.clone();
                new_elements.push(append_val.clone());
                env.borrow_mut()
                    .set(name.clone(), Value::List(new_elements));
                Ok(append_val)
            } else {
                Err("APPEND requires a list variable".to_string())
            }
        }

        AstNode::ListRemove(list, index) | AstNode::Remove(list, index) => {
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;

            if let AstNode::Identifier(name) = &**list {
                let elements = if let Some(Value::List(elements)) = env.borrow().get(name) {
                    elements
                } else {
                    return Err(format!("Variable {} is not a list", name));
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
                        Err("List index out of bounds".to_string())
                    }
                } else {
                    Err("REMOVE requires an integer index".to_string())
                }
            } else {
                Err("REMOVE requires a list variable".to_string())
            }
        }

        AstNode::Random(min, max) => {
            let min_val = evaluate_node(min, Rc::clone(&env), debug)?;
            let max_val = evaluate_node(max, Rc::clone(&env), debug)?;

            match (min_val, max_val) {
                (Value::Integer(min_int), Value::Integer(max_int)) => {
                    if min_int > max_int {
                        return Err("Min value must be less than or equal to max value".to_string());
                    }
                    let mut rng = rand::thread_rng();
                    Ok(Value::Integer(rng.gen_range(min_int..=max_int)))
                }
                _ => Err("RANDOM requires integer arguments".to_string()),
            }
        }

        AstNode::ClassDecl(name, body) => {
            env.borrow_mut()
                .classes
                .insert(name.clone(), (**body).clone());
            Ok(Value::Unit)
        }

        AstNode::Import(path) => {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read import file {}: {}", path, e))?;

            let mut lexer = crate::lexer::Lexer::new(&content);
            let tokens = lexer.tokenize();

            let imported_ast = crate::parser::parse(tokens, false)
                .map_err(|e| format!("Failed to parse import file {}: {}", path, e))?;

            evaluate_node(&imported_ast, Rc::clone(&env), debug)?;

            Ok(Value::Unit)
        }

        AstNode::Return(expr) => {
            let value = evaluate_node(expr, Rc::clone(&env), debug)?;
            env.borrow_mut().return_value = Some(value);
            Err("Return".to_string())
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
                Err("SORT requires a list as an argument".to_string())
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
                Err(error) => {
                    env.borrow_mut().output.push_str(&try_env.borrow().output);

                    let catch_env =
                        Rc::new(RefCell::new(Environment::new_with_parent(Rc::clone(&env))));
                    if let Some(var_name) = error_var {
                        catch_env
                            .borrow_mut()
                            .set(var_name.clone(), Value::String(error));
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
                let ast = parser.parse_expression(debug)?;

                evaluate_node(&ast, Rc::clone(&env), debug)
            } else {
                Err("EVAL requires a string argument".to_string())
            }
        }
        _ => Err(format!("Unimplemented node type: {:?}", node)),
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
