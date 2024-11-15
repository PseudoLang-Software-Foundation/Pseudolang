use crate::parser::{AstNode, BinaryOperator, UnaryOperator};
use rand::Rng;
use std::collections::HashMap;
use std::io::{self, Write};

#[derive(Debug, Clone)]
enum Value {
    Integer(i32),
    Float(f32),
    String(String),
    Boolean(bool),
    List(Vec<Value>),
    Unit,
}

#[derive(Clone)]
struct Environment {
    variables: HashMap<String, Value>,
    procedures: HashMap<String, (Vec<String>, AstNode)>,
    classes: HashMap<String, AstNode>, // Added for class support
    output: String,
    return_value: Option<Value>,      // Add this field
    parent: Option<Box<Environment>>, // Add parent scope
}

impl Environment {
    fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            procedures: HashMap::new(),
            classes: HashMap::new(), // Added
            output: String::new(),
            return_value: None, // Initialize the new field
            parent: None,
        }
    }

    fn with_parent(parent: Environment) -> Self {
        Environment {
            variables: HashMap::new(),
            procedures: parent.procedures.clone(),
            classes: parent.classes.clone(),
            output: parent.output.clone(),
            return_value: None,
            parent: Some(Box::new(parent)),
        }
    }

    fn get(&self, name: &str) -> Option<Value> {
        // First check current environment
        if let Some(value) = self.variables.get(name) {
            return Some(value.clone());
        }
        // Then check parent environment if it exists
        if let Some(parent) = &self.parent {
            return parent.get(name);
        }
        None
    }

    fn set(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }
}

pub fn run(ast: AstNode) -> Result<String, String> {
    println!("Starting interpreter...");
    let mut env = Environment::new();
    let result = evaluate_node(&ast, &mut env)?;
    println!("Execution completed");
    Ok(format!("{}{}", env.output, value_to_string(&result)))
}

fn evaluate_node(node: &AstNode, env: &mut Environment) -> Result<Value, String> {
    println!("Evaluating node: {:?}", node);
    match node {
        AstNode::Program(statements) => {
            let mut last_value = Value::Unit;
            for stmt in statements {
                last_value = evaluate_node(stmt, env)?;
                // Check if this statement is a Display node
                if matches!(stmt, AstNode::Display(_)) {
                    last_value = Value::Unit;
                }
            }
            Ok(last_value)
        }

        AstNode::Block(statements) => {
            let mut last_value = Value::Unit;
            for stmt in statements {
                last_value = evaluate_node(stmt, env)?;
            }
            Ok(last_value)
        }

        AstNode::Integer(n) => Ok(Value::Integer(*n)),
        AstNode::Float(f) => Ok(Value::Float(*f)),
        AstNode::String(s) => Ok(Value::String(s.clone())),
        AstNode::Boolean(b) => Ok(Value::Boolean(*b)),
        AstNode::List(elements) => {
            let mut values = Vec::new();
            for elem in elements {
                values.push(evaluate_node(elem, env)?);
            }
            Ok(Value::List(values))
        }

        AstNode::Identifier(name) => env
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name)),

        AstNode::Assignment(target, value) => {
            let val = evaluate_node(value, env)?;
            if let AstNode::Identifier(name) = &**target {
                println!("Assigning {} = {:?}", name, val); // Add debug output
                env.set(name.clone(), val.clone());
                Ok(val)
            } else {
                Err("Invalid assignment target".to_string())
            }
        }

        AstNode::BinaryOp(left, op, right) => {
            let left_val = evaluate_node(left, env)?;
            let right_val = evaluate_node(right, env)?;
            evaluate_binary_op(&left_val, op, &right_val)
        }

        AstNode::UnaryOp(op, expr) => {
            let val = evaluate_node(expr, env)?;
            evaluate_unary_op(op, &val)
        }

        AstNode::If(condition, then_branch, else_branch) => {
            let cond_val = evaluate_node(condition, env)?;
            match cond_val {
                Value::Boolean(true) => evaluate_node(then_branch, env),
                Value::Boolean(false) => {
                    if let Some(else_branch) = else_branch {
                        evaluate_node(else_branch, env)
                    } else {
                        Ok(Value::Unit)
                    }
                }
                _ => Err("Condition must be a boolean".to_string()),
            }
        }

        AstNode::RepeatTimes(count, body) => {
            let count_val = evaluate_node(count, env)?;
            if let Value::Integer(n) = count_val {
                for _ in 0..n {
                    evaluate_node(body, env)?;
                }
                Ok(Value::Unit)
            } else {
                Err("REPEAT count must be an integer".to_string())
            }
        }

        AstNode::Display(expr) => {
            if let Some(expr) = expr {
                let value = evaluate_node(expr, env)?;
                let output = format!("{}\n", value_to_string(&value));
                env.output.push_str(&output);
            } else {
                env.output.push('\n');
            }
            Ok(Value::Unit) // Change this to Value::Unit so it doesn't print the return value
        }

        AstNode::DisplayInline(expr) => {
            let value = evaluate_node(expr, env)?;
            let output = value_to_string(&value);
            env.output.push_str(&output);
            Ok(Value::Unit)
        }

        AstNode::Input => {
            print!("> ");
            io::stdout().flush().map_err(|e| e.to_string())?;
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| e.to_string())?;
            Ok(Value::String(input.trim().to_string()))
        }

        AstNode::ProcedureDecl(name, params, body) => {
            env.procedures
                .insert(name.clone(), (params.clone(), (**body).clone()));
            Ok(Value::Unit)
        }

        AstNode::ProcedureCall(name, args) => {
            if let Some((params, body)) = env.procedures.get(name).cloned() {
                let mut new_env = Environment::with_parent(env.clone());

                if args.len() != params.len() {
                    return Err(format!("Wrong number of arguments for procedure {}", name));
                }

                // Evaluate arguments in caller's environment
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(evaluate_node(arg, env)?);
                }

                // Set parameters in procedure's environment
                for (param, arg_val) in params.iter().zip(evaluated_args) {
                    new_env.set(param.clone(), arg_val);
                }

                let result = evaluate_node(&body, &mut new_env)?;

                // Transfer output back to caller's environment
                env.output += &new_env.output;

                // Return any explicit return value first
                if let Some(return_val) = new_env.return_value {
                    Ok(return_val)
                } else {
                    Ok(result)
                }
            } else {
                Err(format!("Undefined procedure: {}", name))
            }
        }

        AstNode::ListAccess(list, index) => {
            let list_val = evaluate_node(list, env)?;
            let index_val = evaluate_node(index, env)?;

            if let (Value::List(elements), Value::Integer(i)) = (list_val, index_val) {
                let idx = i - 1; // Convert 1-based to 0-based indexing
                println!("List access: index {} (internal {})", i, idx); // Debug info
                if idx >= 0 && (idx as usize) < elements.len() {
                    Ok(elements[idx as usize].clone())
                } else {
                    Err(format!("List index out of bounds: {}", i))
                }
            } else {
                Err("Invalid list access".to_string())
            }
        }

        AstNode::ListAssignment(list, index, value) => {
            let index_val = evaluate_node(index, env)?;
            let new_val = evaluate_node(value, env)?;

            if let AstNode::Identifier(name) = &**list {
                if let Some(Value::List(mut elements)) = env.get(name) {
                    if let Value::Integer(i) = index_val {
                        let idx = i - 1; // Convert to 0-based
                        if idx >= 0 && (idx as usize) < elements.len() {
                            elements[idx as usize] = new_val.clone();
                            env.set(name.clone(), Value::List(elements));
                            Ok(new_val)
                        } else {
                            Err("List index out of bounds".to_string())
                        }
                    } else {
                        Err("Invalid list index".to_string())
                    }
                } else {
                    Err(format!("Variable {} is not a list", name))
                }
            } else {
                Err("Invalid list assignment target".to_string())
            }
        }

        AstNode::Substring(string, start, end) => {
            let str_val = evaluate_node(string, env)?;
            let start_val = evaluate_node(start, env)?;
            let end_val = evaluate_node(end, env)?;

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
            let s1 = evaluate_node(str1, env)?;
            let s2 = evaluate_node(str2, env)?;
            if let (Value::String(s1), Value::String(s2)) = (s1, s2) {
                Ok(Value::String(format!("{}{}", s1, s2)))
            } else {
                Err("CONCAT requires string arguments".to_string())
            }
        }

        AstNode::ToString(expr) => {
            let val = evaluate_node(expr, env)?;
            Ok(Value::String(value_to_string(&val)))
        }

        AstNode::ToNum(expr) => {
            let val = evaluate_node(expr, env)?;
            if let Value::String(s) = val {
                if let Ok(n) = s.parse::<i32>() {
                    Ok(Value::Integer(n))
                } else if let Ok(f) = s.parse::<f32>() {
                    Ok(Value::Float(f))
                } else {
                    Err("Cannot convert string to number".to_string())
                }
            } else {
                Err("TONUM requires string argument".to_string())
            }
        }

        AstNode::RepeatUntil(body, condition) => {
            let max_iterations = 1000;
            let mut iterations = 0;

            loop {
                iterations += 1;
                if iterations > max_iterations {
                    return Err(format!(
                        "Maximum loop iterations ({}) exceeded. Binary search may be stuck.",
                        max_iterations
                    ));
                }

                // Execute the body first (like do-while)
                let result = evaluate_node(body, env)?;

                // Check if this was a return statement
                if env.return_value.is_some() {
                    return Ok(result);
                }

                // Then check the condition
                match evaluate_node(condition, env)? {
                    Value::Boolean(true) => break,
                    Value::Boolean(false) => continue,
                    _ => return Err("REPEAT UNTIL condition must evaluate to boolean".to_string()),
                }
            }
            Ok(Value::Unit)
        }

        AstNode::ForEach(var_name, list, body) => {
            let list_val = evaluate_node(list, env)?;
            if let Value::List(elements) = list_val {
                let mut result = Value::Unit;
                for element in elements {
                    env.set(var_name.clone(), element);
                    result = evaluate_node(body, env)?;
                }
                Ok(result)
            } else {
                Err("FOR EACH requires list".to_string())
            }
        }

        AstNode::RawString(s) => Ok(Value::String(s.clone())),

        AstNode::FormattedString(template, vars) => {
            let mut values = Vec::new();
            for var_name in vars {
                if let Some(val) = env.get(var_name) {
                    values.push(value_to_string(&val));
                } else {
                    return Err(format!("Undefined variable in format string: {}", var_name));
                }
            }
            let result = template.replace("{}", "{}");
            Ok(Value::String(
                format!("{}", result).replace("{}", &values.join(" ")),
            ))
        }

        AstNode::Length(list) => {
            let list_val = evaluate_node(list, env)?;
            match list_val {
                Value::List(elements) => Ok(Value::Integer(elements.len() as i32)),
                Value::String(s) => Ok(Value::Integer(s.len() as i32)),
                _ => Err("LENGTH requires a list or string argument".to_string()),
            }
        }

        AstNode::ListInsert(list, index, value) => {
            let list_val = evaluate_node(list, env)?;
            let index_val = evaluate_node(index, env)?;
            let insert_val = evaluate_node(value, env)?;

            if let (Value::List(mut elements), Value::Integer(i)) = (list_val, index_val) {
                let idx = i - 1; // Convert 1-based to 0-based
                if idx >= 0 && (idx as usize) <= elements.len() {
                    elements.insert(idx as usize, insert_val);
                    if let AstNode::Identifier(name) = &**list {
                        env.set(name.clone(), Value::List(elements.clone()));
                        Ok(Value::List(elements))
                    } else {
                        Err("Invalid list target for INSERT".to_string())
                    }
                } else {
                    Err("List index out of bounds".to_string())
                }
            } else {
                Err("INSERT requires a list and integer index".to_string())
            }
        }

        AstNode::ListAppend(list, value) => {
            let list_val = evaluate_node(list, env)?;
            let append_val = evaluate_node(value, env)?;

            if let Value::List(mut elements) = list_val {
                elements.push(append_val);
                if let AstNode::Identifier(name) = &**list {
                    env.set(name.clone(), Value::List(elements.clone()));
                    Ok(Value::List(elements))
                } else {
                    Err("Invalid list target for APPEND".to_string())
                }
            } else {
                Err("APPEND requires a list argument".to_string())
            }
        }

        AstNode::ListRemove(list, index) => {
            let list_val = evaluate_node(list, env)?;
            let index_val = evaluate_node(index, env)?;

            if let (Value::List(mut elements), Value::Integer(i)) = (list_val, index_val) {
                let idx = i - 1; // Convert 1-based to 0-based
                if idx >= 0 && (idx as usize) < elements.len() {
                    elements.remove(idx as usize);
                    if let AstNode::Identifier(name) = &**list {
                        env.set(name.clone(), Value::List(elements.clone()));
                        Ok(Value::List(elements))
                    } else {
                        Err("Invalid list target for REMOVE".to_string())
                    }
                } else {
                    Err("List index out of bounds".to_string())
                }
            } else {
                Err("REMOVE requires a list and integer index".to_string())
            }
        }

        AstNode::Random(min, max) => {
            let min_val = evaluate_node(min, env)?;
            let max_val = evaluate_node(max, env)?;

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
            env.classes.insert(name.clone(), (**body).clone());
            Ok(Value::Unit)
        }

        AstNode::Import(path) => {
            // Read the file content
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read import file {}: {}", path, e))?;

            // Create a new lexer for the content
            let mut lexer = crate::lexer::Lexer::new(&content);
            let tokens = lexer.tokenize();

            // Parse the imported content
            let imported_ast = crate::parser::parse(tokens, false)
                .map_err(|e| format!("Failed to parse import file {}: {}", path, e))?;

            // Evaluate the imported code in the current environment
            evaluate_node(&imported_ast, env)?;

            Ok(Value::Unit)
        }

        AstNode::Return(expr) => {
            let value = evaluate_node(expr, env)?;
            env.return_value = Some(value.clone());
            // Immediately return from the procedure
            return Ok(value);
        }

        // Add implementations for remaining nodes as needed
        _ => Err(format!("Unimplemented node type: {:?}", node)),
    }
}

fn evaluate_binary_op(left: &Value, op: &BinaryOperator, right: &Value) -> Result<Value, String> {
    match (left, op, right) {
        (Value::Integer(a), BinaryOperator::Add, Value::Integer(b)) => Ok(Value::Integer(a + b)),
        (Value::Integer(a), BinaryOperator::Sub, Value::Integer(b)) => Ok(Value::Integer(a - b)),
        (Value::Integer(a), BinaryOperator::Mul, Value::Integer(b)) => Ok(Value::Integer(a * b)),
        (Value::Integer(a), BinaryOperator::Div, Value::Integer(b)) => {
            if *b == 0 {
                Err("Division by zero".to_string())
            } else {
                // Changed to ensure correct mid calculation
                Ok(Value::Integer((a + (b - 1) / 2) / b))
            }
        }
        (Value::Integer(a), BinaryOperator::Mod, Value::Integer(b)) => {
            if *b == 0 {
                Err("Modulo by zero".to_string())
            } else {
                Ok(Value::Integer(a % b))
            }
        }
        // Add more operator implementations
        (Value::Integer(a), BinaryOperator::Eq, Value::Integer(b)) => Ok(Value::Boolean(a == b)),
        (Value::Integer(a), BinaryOperator::NotEq, Value::Integer(b)) => Ok(Value::Boolean(a != b)),
        (Value::Integer(a), BinaryOperator::Lt, Value::Integer(b)) => Ok(Value::Boolean(a < b)),
        (Value::Integer(a), BinaryOperator::LtEq, Value::Integer(b)) => Ok(Value::Boolean(a <= b)),
        (Value::Integer(a), BinaryOperator::Gt, Value::Integer(b)) => Ok(Value::Boolean(a > b)),
        (Value::Integer(a), BinaryOperator::GtEq, Value::Integer(b)) => Ok(Value::Boolean(a >= b)),

        // Add boolean operators
        (Value::Boolean(a), BinaryOperator::And, Value::Boolean(b)) => Ok(Value::Boolean(*a && *b)),
        (Value::Boolean(a), BinaryOperator::Or, Value::Boolean(b)) => Ok(Value::Boolean(*a || *b)),

        // String comparisons
        (Value::String(a), BinaryOperator::Eq, Value::String(b)) => Ok(Value::Boolean(a == b)),
        (Value::String(a), BinaryOperator::NotEq, Value::String(b)) => Ok(Value::Boolean(a != b)),

        // Allow string concatenation with +
        (Value::String(a), BinaryOperator::Add, Value::String(b)) => {
            Ok(Value::String(format!("{}{}", a, b)))
        }

        // Handle float operations
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

        // Handle mixed integer/float operations by promoting to float
        (Value::Integer(a), BinaryOperator::Add, Value::Float(b)) => {
            Ok(Value::Float(*a as f32 + b))
        }
        (Value::Float(a), BinaryOperator::Add, Value::Integer(b)) => {
            Ok(Value::Float(a + *b as f32))
        }
        (Value::Integer(a), BinaryOperator::Sub, Value::Float(b)) => {
            Ok(Value::Float(*a as f32 - b))
        }
        (Value::Float(a), BinaryOperator::Sub, Value::Integer(b)) => {
            Ok(Value::Float(a - *b as f32))
        }
        (Value::Integer(a), BinaryOperator::Mul, Value::Float(b)) => {
            Ok(Value::Float(*a as f32 * b))
        }
        (Value::Float(a), BinaryOperator::Mul, Value::Integer(b)) => {
            Ok(Value::Float(a * *b as f32))
        }
        (Value::Integer(a), BinaryOperator::Div, Value::Float(b)) => {
            if *b == 0.0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Float(*a as f32 / b))
            }
        }
        (Value::Float(a), BinaryOperator::Div, Value::Integer(b)) => {
            if *b == 0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Float(a / *b as f32))
            }
        }

        // Handle integer division to produce float result
        // ...rest of existing match arms...
        _ => Err(format!(
            "Invalid operation: {:?} {:?} {:?}",
            left, op, right
        )),
    }
}

fn evaluate_unary_op(op: &UnaryOperator, val: &Value) -> Result<Value, String> {
    match (op, val) {
        (UnaryOperator::Neg, Value::Integer(n)) => Ok(Value::Integer(-n)),
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
    }
}
