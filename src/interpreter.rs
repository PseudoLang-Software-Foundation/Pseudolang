use crate::error::{PSLError, Span, StackFrame};
use crate::parser::{AstNode, BinaryOperator, Spanned, UnaryOperator};
use crate::system;
use num_bigint::BigInt;
use num_traits::{FromPrimitive, One, Signed, ToPrimitive, Zero};
use rand::RngExt;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasi"))]
use std::thread;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasi"))]
use std::time::Duration;

#[derive(Debug, Clone)]
enum Value {
    Integer(BigInt),
    Float(f64),
    String(String),
    Boolean(bool),
    List(Vec<Value>),
    /// Insertion-ordered dictionary. Overwriting an existing key keeps its
    /// original position; a new key is appended. See [`Dict`].
    Dictionary(Dict),
    Unit,
    Null,
    NaN,
}

/// The only value kinds allowed as dictionary keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DictKey {
    String(String),
    Integer(BigInt),
    Boolean(bool),
}

/// Coerce a runtime value into a dictionary key, rejecting the illegal kinds.
fn value_to_key(value: &Value) -> Result<DictKey, String> {
    match value {
        Value::String(s) => Ok(DictKey::String(s.clone())),
        Value::Integer(n) => Ok(DictKey::Integer(n.clone())),
        Value::Boolean(b) => Ok(DictKey::Boolean(*b)),
        _ => Err("Dictionary keys must be strings, integers, or booleans".to_string()),
    }
}

fn key_to_value(key: &DictKey) -> Value {
    match key {
        DictKey::String(s) => Value::String(s.clone()),
        DictKey::Integer(n) => Value::Integer(n.clone()),
        DictKey::Boolean(b) => Value::Boolean(*b),
    }
}

fn key_to_string(key: &DictKey) -> String {
    match key {
        DictKey::String(s) => s.clone(),
        DictKey::Integer(n) => n.to_string(),
        DictKey::Boolean(b) => b.to_string(),
    }
}

/// Dictionaries at or below this many entries are searched by linear scan, and
/// only larger ones pay for the hash index.
///
/// Hashing a `DictKey` costs more than a handful of `==` comparisons: an
/// integer key is a `BigInt`, so hashing it is measurably dearer than comparing
/// it. Timing a hot 3-lookups-per-iteration loop against dictionaries of 9, 16,
/// 24, 32, 48 and 64 integer keys put the break-even at ~32 entries -- below it
/// the index cost up to 18%, above it the scan cost 13% at 48 keys and 35% at
/// 64, growing without bound from there. String keys break even earlier but are
/// flat either way in this range, so the integer crossover sets the limit.
const DICT_LINEAR_SCAN_LIMIT: usize = 32;

/// The body of a dictionary.
///
/// `entries` is the authoritative, insertion-ordered association vector: every
/// observable ordering (DISPLAY, KEYS, VALUES, FOR EACH) reads it directly, so
/// overwriting a key keeps its original position and a new key is appended.
/// `index` is a pure accelerator mapping each key to its position in `entries`;
/// it is absent while the dictionary is small, and once built every mutation
/// below carries it along, so it is never consulted in a stale state. A
/// dictionary that shrinks back under the limit simply keeps its index: it is
/// still correct, and rebuilding on the way down would cost more than it saves.
#[derive(Debug, Clone, Default)]
struct DictInner {
    entries: Vec<(DictKey, Value)>,
    index: Option<HashMap<DictKey, usize>>,
}

impl DictInner {
    /// Position of `key` in `entries`: one hash probe when indexed, a linear
    /// scan while the dictionary is still small.
    fn position(&self, key: &DictKey) -> Option<usize> {
        match &self.index {
            Some(index) => index.get(key).copied(),
            None => self.entries.iter().position(|(k, _)| k == key),
        }
    }

    /// Whether `key` is present, without building the `Option<usize>` that
    /// `position` returns.
    fn contains(&self, key: &DictKey) -> bool {
        match &self.index {
            Some(index) => index.contains_key(key),
            None => self.entries.iter().any(|(k, _)| k == key),
        }
    }

    /// Insert or overwrite a key, preserving insertion order.
    fn insert(&mut self, key: DictKey, value: Value) {
        if let Some(pos) = self.position(&key) {
            self.entries[pos].1 = value;
            return;
        }
        let pos = self.entries.len();
        match &mut self.index {
            Some(index) => {
                index.insert(key.clone(), pos);
                self.entries.push((key, value));
            }
            None => {
                self.entries.push((key, value));
                if self.entries.len() > DICT_LINEAR_SCAN_LIMIT {
                    self.build_index();
                }
            }
        }
    }

    /// Remove a key, keeping the surviving entries in insertion order.
    fn remove(&mut self, key: &DictKey) -> Option<Value> {
        let pos = self.position(key)?;
        let (_, removed) = self.entries.remove(pos);
        // Every entry after `pos` shifted down one slot, so the index has to be
        // repaired; `Vec::remove` is already O(n), so this costs nothing extra.
        if let Some(index) = &mut self.index {
            index.remove(key);
            for slot in index.values_mut() {
                if *slot > pos {
                    *slot -= 1;
                }
            }
        }
        Some(removed)
    }

    fn build_index(&mut self) {
        let mut index = HashMap::with_capacity(self.entries.len());
        for (pos, (key, _)) in self.entries.iter().enumerate() {
            index.insert(key.clone(), pos);
        }
        self.index = Some(index);
    }
}

/// Insertion-ordered dictionary with average O(1) lookup and O(1) copy.
///
/// The body sits behind an `Rc` and is copied on write, which is exactly
/// PseudoLang's copy-on-assign value semantics: `b <- a` shares the body and
/// the first mutation of either side forks it, so neither can observe the
/// other's writes. Sharing only makes copying cheap -- no caller can reach the
/// body except through the methods here, all of which take `&mut self` before
/// touching it.
#[derive(Clone, Default)]
struct Dict {
    inner: Rc<DictInner>,
}

/// Printed as the bare association list a dictionary used to be, so the `{:?}`
/// of a `Value` -- which reaches user-visible text, e.g. the "Invalid operation"
/// message -- is unchanged by the wrapper types.
impl std::fmt::Debug for Dict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.entries.fmt(f)
    }
}

impl Dict {
    fn len(&self) -> usize {
        self.inner.entries.len()
    }

    fn get(&self, key: &DictKey) -> Option<&Value> {
        self.inner
            .position(key)
            .map(|pos| &self.inner.entries[pos].1)
    }

    /// The value stored at `key`, borrowed for mutation. Forks a shared body,
    /// so a nested write through this reference cannot leak into a copy.
    fn get_mut(&mut self, key: &DictKey) -> Option<&mut Value> {
        let inner = Rc::make_mut(&mut self.inner);
        let pos = inner.position(key)?;
        Some(&mut inner.entries[pos].1)
    }

    fn contains_key(&self, key: &DictKey) -> bool {
        self.inner.contains(key)
    }

    fn insert(&mut self, key: DictKey, value: Value) {
        Rc::make_mut(&mut self.inner).insert(key, value);
    }

    fn remove(&mut self, key: &DictKey) -> Option<Value> {
        Rc::make_mut(&mut self.inner).remove(key)
    }

    fn iter(&self) -> std::slice::Iter<'_, (DictKey, Value)> {
        self.inner.entries.iter()
    }

    fn keys(&self) -> impl Iterator<Item = &DictKey> {
        self.inner.entries.iter().map(|(k, _)| k)
    }

    fn values(&self) -> impl Iterator<Item = &Value> {
        self.inner.entries.iter().map(|(_, v)| v)
    }
}

/// Number of characters in `s`.
///
/// Every string position in PseudoLang is a character position, never a byte
/// offset: `LENGTH`, `s[i]`, `SUBSTRING` and `FIND` all agree, so an index
/// produced by one can be handed straight to another even when the string
/// holds non-ASCII text. Pure-ASCII strings take the O(1) byte-length path,
/// where the two counts coincide.
#[inline]
fn str_char_len(s: &str) -> usize {
    if s.is_ascii() {
        s.len()
    } else {
        s.chars().count()
    }
}

/// The inclusive character range `start..=end` of `s`, or `None` when the range
/// runs off the end of the string or is reversed.
fn char_range(s: &str, start: usize, end: usize) -> Option<&str> {
    if end < start {
        return None;
    }
    // A string never has more characters than bytes, so an `end` at or past
    // `s.len()` is out of range whatever the encoding. This also bounds `end`
    // below `usize::MAX`, so the `end + 1` further down cannot overflow.
    if end >= s.len() {
        return None;
    }
    if s.is_ascii() {
        // Byte offsets and character offsets coincide, and every byte is a
        // char boundary, so the slice is safe.
        return Some(&s[start..=end]);
    }
    let mut begin = None;
    let mut stop = None;
    let mut seen = 0usize;
    for (nth, (byte_idx, _)) in s.char_indices().enumerate() {
        seen = nth + 1;
        if nth == start {
            begin = Some(byte_idx);
        }
        if nth == end + 1 {
            stop = Some(byte_idx);
            break;
        }
    }
    match (begin, stop) {
        (Some(begin), Some(stop)) => Some(&s[begin..stop]),
        // The range ends on the last character, so it runs to the end.
        (Some(begin), None) if end < seen => Some(&s[begin..]),
        _ => None,
    }
}

/// LENGTH of the value kinds that have one, without copying it.
fn container_len(value: &Value) -> Option<usize> {
    match value {
        Value::List(elements) => Some(elements.len()),
        Value::String(s) => Some(str_char_len(s)),
        Value::Dictionary(entries) => Some(entries.len()),
        _ => None,
    }
}

/// What kind of container a variable holds, probed without copying it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ContainerKind {
    List,
    Dictionary,
    Other,
}

enum Interruption {
    Return(Value),
    Error(PSLError),
    /// EXIT: stop the program with this status.
    ///
    /// Unwinds rather than calling `process::exit` at the point of the call, so the
    /// run's output survives. Only [`run_with_mode`] decides what a status means: the
    /// CLI turns it into the process's own exit code, while a library or WASM caller
    /// gets the output back and keeps its process. Distinct from `Error`, so TRY does
    /// not catch it.
    Exit(i32),
}

type EvalResult = Result<Value, Interruption>;

fn runtime_err(msg: impl Into<String>, span: Span, env: &Rc<RefCell<Environment>>) -> Interruption {
    // Spans are per-file, so an error raised while an imported file is executing has
    // to carry that file's text: resolved against the entry script the offsets land on
    // unrelated lines. `invoke_procedure` enters a procedure's declaring file, so this
    // is also right for a library procedure called long after its IMPORT returned.
    let (source, origin) = match env.borrow().modules.borrow().current_source() {
        Some((source, name)) => (Some(source), Some(name)),
        None => (None, None),
    };
    Interruption::Error(PSLError {
        message: msg.into(),
        span: Some(span),
        stack_trace: env.borrow().get_call_stack(),
        source,
        origin,
    })
}

/// Rank of a value's kind for [`sort_cmp`]. Numbers sort before strings, then
/// booleans, then containers, then the empty values.
fn sort_rank(value: &Value) -> u8 {
    match value {
        Value::Integer(_) | Value::Float(_) => 0,
        Value::String(_) => 1,
        Value::Boolean(_) => 2,
        Value::List(_) => 3,
        Value::Dictionary(_) => 4,
        Value::Null => 5,
        Value::NaN => 6,
        Value::Unit => 7,
    }
}

/// Total order over `Value`, used by SORT.
///
/// This MUST be a total order. A comparator that reports unrelated kinds as
/// `Equal` is not transitive -- `1 < 2` while `1 == "x"` and `2 == "x"` -- and
/// Rust's sort detects the inconsistency and PANICS, aborting the interpreter
/// on a list a user is perfectly entitled to write. Comparing the kind rank
/// first makes mixed lists group by kind instead, which is deterministic and
/// cannot panic. Integers and floats share a rank so they still interleave
/// numerically, which is the documented behaviour.
fn sort_cmp(a: &Value, b: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => x.total_cmp(y),
        (Value::Integer(x), Value::Float(y)) => bigint_to_f64(x).total_cmp(y),
        (Value::Float(x), Value::Integer(y)) => x.total_cmp(&bigint_to_f64(y)),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::Boolean(x), Value::Boolean(y)) => x.cmp(y),
        // Same-kind values with no meaningful order (two lists, two
        // dictionaries, two NULLs) compare equal; `sort_by` is stable, so they
        // keep their original relative order.
        _ => sort_rank(a).cmp(&sort_rank(b)).then(Ordering::Equal),
    }
}

const MAX_STACK_DEPTH: usize = 1000;

/// How deeply EVAL and EXECUTE may nest inside one another.
///
/// Much smaller than [`MAX_STACK_DEPTH`]. One level of nested source evaluation
/// holds a lexer, a token vector, a parser and an AST live across the recursive
/// call: about 35 KiB of real stack per level in a debug build, against about
/// 4 KiB for a procedure frame. At 1000 levels the process dies of a genuine
/// stack overflow before the counter trips, which `code <- "EVAL(code)"` did.
/// 32 keeps the worst case near a megabyte and still allows any real nesting.
const MAX_META_DEPTH: usize = 32;
const MAX_LOOP_ITERATIONS: usize = 1_000_000;

/// Size of the userspace buffer used when streaming straight to stdout.
const STREAM_BUF_BYTES: usize = 64 * 1024;

/// Where a run's program output goes.
///
/// Chosen once, before evaluation starts. [`OutputMode::Capture`] is for
/// callers that are handed the whole output back as a `String` -- the test
/// suite, `execute_code(.., return_output = true)` and WASM.
/// [`OutputMode::Stdout`] is for the CLI, which wants the text on the terminal
/// as it is produced rather than accumulated in RAM until the program ends.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OutputMode {
    /// Accumulate everything into a `String` and return it.
    Capture,
    /// Write through to a locked, buffered stdout; return an empty `String`.
    Stdout,
}

/// The sink every DISPLAY writes through.
///
/// Deliberately an enum rather than a `Box<dyn Write>`. The hot arm is
/// `Capture`, which wants a plain `String::push_str`: no `io::Result` to
/// inspect, no UTF-8 re-validation, no vtable indirection, and the whole call
/// still inlines. A trait object would force every DISPLAY through an indirect
/// `write_all(&[u8])` whose `io::Result` the caller then throws away. The
/// enum's discriminant test is one perfectly-predicted branch, since a given
/// run only ever takes one arm.
enum OutputSink {
    Capture(String),
    Stream {
        writer: io::BufWriter<io::StdoutLock<'static>>,
        /// First write/flush failure seen, if any. Reported once the run ends
        /// so a full disk cannot silently truncate a program's output.
        error: Option<io::Error>,
        /// Flush after every write, reproducing the old `println!` +
        /// `stdout().flush()` behaviour. Set when stdout is a terminal (so
        /// interactive prompts still appear before the program blocks on
        /// INPUT) or when debug tracing is on (so the trace, which goes out
        /// through `println!`, stays interleaved in the right order).
        autoflush: bool,
    },
}

/// Keep the FIRST io error seen; later ones are usually consequences of it.
///
/// `BrokenPipe` is deliberately NOT an error: `fpli run prog.psl | head` closes
/// the pipe early, and every well-behaved Unix program treats that as a normal
/// end of output rather than a failure.
fn record_io(slot: &mut Option<io::Error>, result: io::Result<()>) {
    if let Err(e) = result
        && e.kind() != io::ErrorKind::BrokenPipe
        && slot.is_none()
    {
        *slot = Some(e);
    }
}

fn stdout_is_terminal() -> bool {
    #[cfg(any(not(target_arch = "wasm32"), feature = "wasi"))]
    {
        use std::io::IsTerminal;
        io::stdout().is_terminal()
    }
    #[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
    {
        false
    }
}

impl OutputSink {
    fn new(mode: OutputMode, debug: bool) -> Self {
        match mode {
            OutputMode::Capture => OutputSink::Capture(String::default()),
            OutputMode::Stdout => OutputSink::Stream {
                writer: io::BufWriter::with_capacity(STREAM_BUF_BYTES, io::stdout().lock()),
                error: None,
                autoflush: debug || stdout_is_terminal(),
            },
        }
    }

    /// Program output with no trailing newline (DISPLAYINLINE).
    #[inline]
    fn write_str(&mut self, s: &str) {
        match self {
            OutputSink::Capture(buf) => buf.push_str(s),
            OutputSink::Stream {
                writer,
                error,
                autoflush,
            } => {
                record_io(error, writer.write_all(s.as_bytes()));
                if *autoflush {
                    record_io(error, writer.flush());
                }
            }
        }
    }

    /// Program output followed by a newline (DISPLAY).
    #[inline]
    fn write_line(&mut self, s: &str) {
        match self {
            OutputSink::Capture(buf) => {
                buf.push_str(s);
                buf.push('\n');
            }
            OutputSink::Stream {
                writer,
                error,
                autoflush,
            } => {
                record_io(error, writer.write_all(s.as_bytes()));
                record_io(error, writer.write_all(b"\n"));
                if *autoflush {
                    record_io(error, writer.flush());
                }
            }
        }
    }

    /// Text that has only ever reached the *captured* output and was never
    /// printed by the CLI: the f-string-assignment echo and the INPUT echo (the
    /// terminal has already echoed what the user typed). Keeping these
    /// capture-only is what makes the sink a pure refactor of observable CLI
    /// behaviour.
    #[inline]
    fn record_line(&mut self, s: &str) {
        if let OutputSink::Capture(buf) = self {
            buf.push_str(s);
            buf.push('\n');
        }
    }

    /// Text that goes to the terminal but is never captured (the INPUT prompt).
    /// Unused on `wasm32-unknown-unknown`, where INPUT goes through `prompt()`.
    #[allow(dead_code)]
    fn write_prompt(&mut self, s: &str) {
        match self {
            OutputSink::Capture(_) => {
                print!("{}", s);
                let _ = io::stdout().flush();
            }
            OutputSink::Stream { writer, error, .. } => {
                record_io(error, writer.write_all(s.as_bytes()));
                record_io(error, writer.flush());
            }
        }
    }

    /// Push whatever is buffered out to the OS. Called before anything that
    /// stalls or blocks the program (INPUT, SLEEP) and once at the end of the
    /// run, including the error path.
    fn flush(&mut self) {
        if let OutputSink::Stream { writer, error, .. } = self {
            record_io(error, writer.flush());
        }
    }

    /// Take the first write failure seen, if any.
    fn take_write_error(&mut self) -> Option<io::Error> {
        match self {
            OutputSink::Capture(_) => None,
            OutputSink::Stream { error, .. } => error.take(),
        }
    }

    /// Finish the run: flush anything pending and yield the captured text
    /// (empty when streaming).
    fn finish(&mut self) -> String {
        match self {
            OutputSink::Capture(buf) => std::mem::take(buf),
            OutputSink::Stream { writer, error, .. } => {
                record_io(error, writer.flush());
                String::default()
            }
        }
    }
}

/// A declared procedure: its parameter names, its body, and the file it was
/// written in.
///
/// Held behind an `Rc` because the whole procedure table is snapshotted into
/// every child scope; sharing the bodies keeps that snapshot from deep-copying
/// the AST of every procedure in the program on every call.
///
/// The third field makes SCRIPTPATH lexical: a running procedure reports the file
/// it was written in, matching Python's `__file__`, which is what a library needs
/// to find a data file beside itself. `None` for a program with no location
/// (EVAL, the library API, the browser playground). One shared `Rc<PathBuf>` per
/// file costs a refcount bump per call instead of a path copy.
type Procedure = Rc<(Vec<String>, Spanned, Option<Rc<PathBuf>>)>;

/// Name -> procedure map.
///
/// The `Rc` makes a scope snapshot O(1). It is copy-on-write via
/// `Rc::make_mut`, which preserves the visibility rule that a scope's
/// declarations are private to that scope: mutating a table that a child scope
/// also holds forks it instead of writing through.
type ProcedureTable = Rc<HashMap<String, Procedure>>;

#[derive(Clone)]
struct Environment {
    variables: HashMap<String, Value>,
    procedures: ProcedureTable,
    /// Shared by every scope in the run. Child scopes used to own a private
    /// `String` that was concatenated into the parent's on every procedure
    /// return and every caught error; sharing one sink removes those copies
    /// entirely and keeps the write order identical, because the writes already
    /// happened in program order and the copies only re-materialised that
    /// order. It also fixes a real bug: a CATCH block that DISPLAYed and then
    /// RETURNed never reached its copy-up, so its output was silently dropped.
    output: Rc<RefCell<OutputSink>>,
    parent: Option<Rc<RefCell<Environment>>>,
    call_stack: Rc<RefCell<Vec<StackFrame>>>,
    parsed_flags: Rc<HashMap<String, String>>,
    /// Which files this run is made of. Shared by every scope, like the sink and
    /// the call stack, because IMPORT can appear at any depth.
    modules: Rc<RefCell<ModuleState>>,
    /// How many EVAL/EXECUTE evaluations are currently nested, guarded by
    /// [`MAX_META_DEPTH`]. Counted separately from the call stack because a
    /// level of nested source costs an order of magnitude more real stack than a
    /// procedure frame does.
    meta_depth: Rc<Cell<usize>>,
}

/// Bookkeeping for a program spread across several `.psl` files.
#[derive(Default)]
struct ModuleState {
    /// Absolute path of the entry script, when the run came from a file at all.
    /// `None` for [`run_with_source`], EVAL-only use and the WASM playground,
    /// where there is no file on disk to be relative to.
    entry: Option<Rc<PathBuf>>,
    /// The files whose code is executing, innermost last. The top of this stack
    /// is what a relative IMPORT resolves against and what SCRIPTPATH reports, so
    /// a library can reach its own neighbours without caring where the
    /// interpreter was started from.
    ///
    /// Pushed both by IMPORT, for as long as the imported file's own top level
    /// runs, and by a call into a procedure that was declared in a different
    /// file, for the duration of that call.
    stack: Vec<Rc<PathBuf>>,
    /// Source that EVAL or EXECUTE is running, innermost last.
    ///
    /// It has no path, so it cannot go in `sources`, and it takes precedence: while
    /// generated source is being evaluated, spans index into it and nothing else.
    generated: Vec<(Rc<str>, String)>,
    /// The text of every imported file, by path.
    ///
    /// Keyed rather than stacked because a file is entered two ways: by IMPORT, and by
    /// a call into a procedure declared in it. A parallel stack pushed only at import
    /// time would be empty for the second, which is the common case -- a library
    /// procedure called long after its IMPORT returned.
    sources: HashMap<PathBuf, Rc<str>>,
    /// Files already imported, in import order.
    ///
    /// Recorded before the body runs, which makes IMPORT idempotent and stops a
    /// cycle from recursing: an import leading back to a file is skipped. Same
    /// semantics as Python -- a cycle is allowed, and each body runs once.
    loaded: Vec<PathBuf>,
}

impl ModuleState {
    /// The file whose code is executing right now: the innermost import or
    /// cross-file procedure call, or else the entry script.
    fn current_file(&self) -> Option<Rc<PathBuf>> {
        self.stack.last().or(self.entry.as_ref()).map(Rc::clone)
    }

    /// The source of the file executing right now, when that is an imported one.
    ///
    /// `None` while the entry script runs: its text is what the caller already holds
    /// and passes to `format`.
    fn current_source(&self) -> Option<(Rc<str>, String)> {
        if let Some((source, label)) = self.generated.last() {
            return Some((Rc::clone(source), label.clone()));
        }
        let file = self.stack.last()?;
        let source = self.sources.get(file.as_path())?;
        Some((Rc::clone(source), file.display().to_string()))
    }

    /// Whether `file` is already the file being executed. Compares by pointer
    /// first because every procedure declared in one file shares that file's
    /// single `Rc`, which makes the common case -- a program whose procedures all
    /// live in the same file -- a pointer comparison per call.
    fn is_current(&self, file: &Rc<PathBuf>) -> bool {
        match self.stack.last().or(self.entry.as_ref()) {
            Some(current) => Rc::ptr_eq(current, file) || current == file,
            None => false,
        }
    }
}

impl Environment {
    fn new(mode: OutputMode, debug: bool) -> Self {
        Environment {
            variables: HashMap::new(),           // skipcq: RS-W1079
            procedures: Rc::new(HashMap::new()), // skipcq: RS-W1079
            output: Rc::new(RefCell::new(OutputSink::new(mode, debug))), // skipcq: RS-W1079
            parent: None,
            call_stack: Rc::new(RefCell::new(Vec::new())), // skipcq: RS-W1079
            parsed_flags: Rc::new(HashMap::new()),         // skipcq: RS-W1079
            modules: Rc::new(RefCell::new(ModuleState::default())), // skipcq: RS-W1079
            meta_depth: Rc::new(Cell::new(0)),             // skipcq: RS-W1079
        }
    }

    fn new_with_parent(parent: Rc<RefCell<Environment>>) -> Self {
        let (procedures, output, call_stack, parsed_flags, modules, meta_depth) = {
            let p = parent.borrow();
            (
                Rc::clone(&p.procedures),
                Rc::clone(&p.output),
                Rc::clone(&p.call_stack),
                Rc::clone(&p.parsed_flags),
                Rc::clone(&p.modules),
                Rc::clone(&p.meta_depth),
            )
        };
        Environment {
            variables: HashMap::new(), // skipcq: RS-W1079
            procedures,
            output,
            parent: Some(Rc::clone(&parent)),
            call_stack,
            parsed_flags,
            modules,
            meta_depth,
        }
    }

    /// The run-wide output sink. Cheap to reach: every scope holds the same
    /// handle, so there is no walk up the parent chain.
    #[inline]
    fn sink(&self) -> &Rc<RefCell<OutputSink>> {
        &self.output
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

    /// Look `name` up along the scope chain and hand the binding to `f` without
    /// copying it, so reading one element out of a container does not clone the
    /// container. `None` means the name is unbound.
    ///
    /// `f` runs while every scope from here to the owning one is immutably
    /// borrowed, so it must not evaluate anything that could mutate a scope.
    fn with_var<R>(&self, name: &str, f: impl FnOnce(&Value) -> R) -> Option<R> {
        if let Some(value) = self.variables.get(name) {
            return Some(f(value));
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow().with_var(name, f);
        }
        None
    }

    /// Mutate the container bound to `name` in place, returning `None` when the
    /// name is unbound.
    ///
    /// Assignment always writes into the *current* scope, so a binding
    /// inherited from an enclosing scope is copied down into this one before
    /// being mutated -- exactly what the old read-clone / rebuild / write-back
    /// sequence did, minus the two deep copies. If `f` rejects the value, a
    /// binding that only existed because of this call is dropped again, so a
    /// failed operation leaves every scope untouched.
    ///
    /// `f` must not touch the environment: this holds a mutable borrow of it.
    fn with_var_mut<T, E>(
        &mut self,
        name: &str,
        f: impl FnOnce(&mut Value) -> Result<T, E>,
    ) -> Option<Result<T, E>> {
        let copied_down = !self.variables.contains_key(name);
        if copied_down {
            let inherited = self.parent.as_ref()?.borrow().get(name)?;
            self.variables.insert(name.to_string(), inherited);
        }
        let result = f(self.variables.get_mut(name)?);
        if result.is_err() && copied_down {
            self.variables.remove(name);
        }
        Some(result)
    }

    fn set(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    fn get_procedure(&self, name: &str) -> Option<Procedure> {
        self.procedures.get(name).cloned()
    }

    /// Every variable name visible from here, innermost scope first, with a name
    /// shadowed by an inner scope reported once. Sorted, so introspecting the
    /// environment gives the same answer every run despite the hash map.
    fn visible_variable_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        self.collect_variable_names(&mut names);
        names.sort();
        names.dedup();
        names
    }

    fn collect_variable_names(&self, out: &mut Vec<String>) {
        out.extend(self.variables.keys().cloned());
        if let Some(ref parent) = self.parent {
            parent.borrow().collect_variable_names(out);
        }
    }

    /// The names of every declared procedure, sorted. The procedure table is
    /// shared run-wide, so this needs no walk up the scope chain.
    fn procedure_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.procedures.keys().cloned().collect();
        names.sort();
        names
    }

    /// The value bound to `name` in this scope only, ignoring enclosing ones.
    ///
    /// Walking the parent chain would copy an inherited binding down into this
    /// scope on restore, giving a different program state.
    fn local(&self, name: &str) -> Option<Value> {
        self.variables.get(name).cloned()
    }

    /// Drop a binding from this scope only, leaving any enclosing one intact.
    fn remove_local(&mut self, name: &str) {
        self.variables.remove(name);
    }

    /// Remove a binding from this scope, reporting whether there was one.
    ///
    /// This scope only. Reaching up the chain would let a procedure delete a
    /// caller's variable, which no assignment can do: `SETVAR` writes the current
    /// scope, so `UNSETVAR` removes from the current scope.
    fn unset(&mut self, name: &str) -> bool {
        self.variables.remove(name).is_some()
    }

    fn declare_procedure(&mut self, name: String, procedure: Procedure) {
        Rc::make_mut(&mut self.procedures).insert(name, procedure);
    }

    /// The scope's procedure table, sharing rather than copying it.
    fn procedure_table(&self) -> ProcedureTable {
        Rc::clone(&self.procedures)
    }

    /// Adopt every procedure from `other` that this scope does not already declare.
    ///
    /// A scope's own declaration of a name wins, which keeps the rule that a nested
    /// `PROCEDURE` is private to the scope that declared it.
    fn merge_procedures(&mut self, other: &ProcedureTable) {
        if other.keys().all(|name| self.procedures.contains_key(name)) {
            return;
        }
        let table = Rc::make_mut(&mut self.procedures);
        for (name, procedure) in other.iter() {
            table
                .entry(name.clone())
                .or_insert_with(|| Rc::clone(procedure));
        }
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

/// Capturing run: the whole output is accumulated and returned. Used by the
/// test suite, the library API and WASM.
pub fn run_with_source(ast: Spanned, source: &str, args: &[String]) -> Result<String, PSLError> {
    run_with_mode(ast, source, args, OutputMode::Capture, false, None)
}

/// Capturing run of a program that came from a file.
///
/// `script_path` is what IMPORT resolves relative paths against and what
/// SCRIPTPATH and ISMAIN report, so a program made of several files behaves the
/// same whichever directory it is launched from.
// Reached through the library API and the test suite, not by the `fpli` binary,
// which goes through `core::execute_code`.
#[allow(dead_code)]
pub fn run_with_source_at(
    ast: Spanned,
    source: &str,
    args: &[String],
    script_path: Option<&Path>,
) -> Result<String, PSLError> {
    run_with_mode(
        ast,
        source,
        args,
        OutputMode::Capture,
        false,
        script_path.map(Path::to_path_buf),
    )
}

/// Run with an explicit output destination.
///
/// [`OutputMode::Stdout`] streams through a locked, buffered stdout and returns
/// an empty `String`. The sink is flushed before this returns on *every* path,
/// error included, so whatever the program printed before failing reaches the
/// terminal ahead of the caller's stderr report.
///
/// `debug` here only selects the sink's flush policy; the evaluator's own trace
/// flag stays `false`, exactly as it always has been.
pub fn run_with_mode(
    ast: Spanned,
    _source: &str,
    args: &[String],
    mode: OutputMode,
    debug: bool,
    script_path: Option<PathBuf>,
) -> Result<String, PSLError> {
    let env = Rc::new(RefCell::new(Environment::new(mode, debug)));
    init_env_with_args(&env, args);
    if let Some(path) = script_path {
        // Fully resolved from the start, for two reasons. The program may CHDIR,
        // and an entry recorded as a relative path would then no longer point at
        // the same file. And it has to be comparable with the canonicalised paths
        // IMPORT produces, or a library that imports the entry script would not be
        // recognised as importing something already running -- on macOS, for
        // instance, `/tmp/x.psl` and its real path `/private/tmp/x.psl` are the
        // same file spelled two ways.
        let spelled = path.to_string_lossy().into_owned();
        let resolved = system::realpath(&spelled)
            .or_else(|_| system::abspath(&spelled))
            .map(PathBuf::from)
            .unwrap_or(path);
        env.borrow().modules.borrow_mut().entry = Some(Rc::new(resolved));
    }
    let result = evaluate_node(&ast, Rc::clone(&env), false);
    let sink = Rc::clone(env.borrow().sink());
    let output = sink.borrow_mut().finish();
    // A failed write to stdout (a full disk, a closed descriptor) must not be
    // swallowed: the program would otherwise exit 0 having silently lost
    // output. A real error beats a plausible-looking empty result.
    if let Some(err) = sink.borrow_mut().take_write_error() {
        return Err(PSLError {
            message: format!("Failed writing program output: {}", err),
            span: None,
            stack_trace: Vec::new(),
            source: None,
            origin: None,
        });
    }
    match result {
        Ok(_) | Err(Interruption::Return(_)) => Ok(output),
        Err(Interruption::Exit(code)) => {
            // Streaming to a terminal means this is the CLI, and EXIT there has to
            // set the process's status. Everything printed is already flushed above.
            if matches!(mode, OutputMode::Stdout) {
                let _ = io::stdout().flush();
                std::process::exit(code);
            }
            // Capturing means a library, test or WASM caller: hand back what the
            // program printed instead of killing the host process.
            Ok(output)
        }
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

/// Evaluate a node in *statement position*, where its value is discarded.
///
/// Behaviour is identical to [`evaluate_node`] in every observable way; the
/// only difference is that an assignment is free to append into the string it
/// already holds instead of building a new one, which it cannot do when the
/// assignment's value is still needed. Contexts that DO observe the value --
/// the last statement of a block (a procedure without RETURN yields it) and the
/// last iteration of FOR EACH -- still go through [`evaluate_node`].
fn evaluate_for_effect(
    node: &Spanned,
    env: Rc<RefCell<Environment>>,
    debug: bool,
) -> Result<(), Interruption> {
    match &node.node {
        AstNode::Assignment(target, value) => {
            if let Some(result) = try_self_append(target, value, &env, debug) {
                return result;
            }
            evaluate_node(node, env, debug).map(|_| ())
        }

        // The only node kinds that recurse back into this function, and so the
        // only ones needing the stack-headroom check `evaluate_node` makes.
        // Every other arm ends in a call to `evaluate_node`, which makes it.
        AstNode::Program(_) | AstNode::Block(_) | AstNode::If(_, _, _) => {
            evaluate_nested_for_effect(node, env, debug)
        }

        _ => evaluate_node(node, env, debug).map(|_| ()),
    }
}

fn evaluate_nested_for_effect(
    node: &Spanned,
    env: Rc<RefCell<Environment>>,
    debug: bool,
) -> Result<(), Interruption> {
    #[cfg(not(target_arch = "wasm32"))]
    return stacker::maybe_grow(64 * 1024, 2 * 1024 * 1024, || {
        evaluate_nested_for_effect_impl(node, env, debug)
    });

    #[cfg(target_arch = "wasm32")]
    evaluate_nested_for_effect_impl(node, env, debug)
}

fn evaluate_nested_for_effect_impl(
    node: &Spanned,
    env: Rc<RefCell<Environment>>,
    debug: bool,
) -> Result<(), Interruption> {
    match &node.node {
        // Every statement of a discarded block is itself discarded.
        AstNode::Program(statements) | AstNode::Block(statements) => {
            for stmt in statements {
                evaluate_for_effect(stmt, Rc::clone(&env), debug)?;
            }
            Ok(())
        }

        // A discarded IF discards whichever branch it takes. Mirrors the
        // `AstNode::If` arm of `evaluate_node_impl`, error and all.
        AstNode::If(condition, then_branch, else_branch) => {
            let cond_val = evaluate_node(condition, Rc::clone(&env), debug)?;
            match cond_val {
                Value::Boolean(true) => evaluate_for_effect(then_branch, env, debug),
                Value::Boolean(false) => match else_branch {
                    Some(else_branch) => evaluate_for_effect(else_branch, env, debug),
                    None => Ok(()),
                },
                _ => Err(runtime_err("Condition must be a boolean", node.span, &env)),
            }
        }

        _ => evaluate_node(node, env, debug).map(|_| ()),
    }
}

/// Run one loop-body iteration, in statement position unless the loop hands its
/// body's value back (FOR EACH evaluates to the value of its LAST iteration).
fn evaluate_loop_body(
    body: &Spanned,
    env: Rc<RefCell<Environment>>,
    debug: bool,
    is_last: bool,
) -> EvalResult {
    if is_last {
        evaluate_node(body, env, debug)
    } else {
        evaluate_for_effect(body, env, debug).map(|()| Value::Unit)
    }
}

/// The source expression of a self-append assignment: `x <- x + <expr>` or
/// `x <- CONCAT(x, <expr>)`, both written with `name` as the left operand.
fn self_append_source<'a>(name: &str, value: &'a Spanned) -> Option<&'a Spanned> {
    let (left, right) = match &value.node {
        AstNode::BinaryOp(left, BinaryOperator::Add, right) => (left, right),
        AstNode::Concat(left, right) => (left, right),
        _ => return None,
    };
    match &left.node {
        AstNode::Identifier(left_name) if left_name == name => Some(right),
        _ => None,
    }
}

/// Fast path for the string-building idiom every student writes:
///
/// ```text
/// s <- ""
/// REPEAT n TIMES { s <- s + "x" }
/// ```
///
/// Evaluated literally that is O(n^2): each iteration clones `s` out of the
/// environment, allocates a fresh `String` for the concatenation, and clones it
/// again on the way back in. Appending into the `String` that is already there
/// makes the loop O(n) amortised and drops the allocation per iteration.
///
/// Returns `None` when the shape does not apply, leaving the caller to take the
/// ordinary assignment path. `Some` means the assignment is finished (or has
/// failed); the right-hand side has been evaluated exactly once either way.
fn try_self_append(
    target: &Spanned,
    value: &Spanned,
    env: &Rc<RefCell<Environment>>,
    debug: bool,
) -> Option<Result<(), Interruption>> {
    if debug {
        // Keep the `Assigning ...` trace exactly as the ordinary path prints it.
        return None;
    }
    let AstNode::Identifier(name) = &target.node else {
        return None;
    };
    let source = self_append_source(name, value)?;

    // One lookup does double duty, so a non-string target pays nothing for
    // passing through here. A string is appended to in place below; anything
    // else (`i <- i + 1`, a list, ...) comes back as the copy the ordinary path
    // would have made when it evaluated the left operand, and is combined with
    // the right-hand side further down instead. An unbound name is left to the
    // ordinary path, which owns that error.
    let bound = {
        let scope = env.borrow();
        scope.with_var(name, |current| match current {
            Value::String(_) => None,
            other => Some(other.clone()),
        })
    };
    let not_a_string = bound?;

    // Evaluated BEFORE the mutable borrow below, which is what makes
    // `s <- s + s`, `s <- s + f(s)` and friends safe: the right-hand side may
    // read the target or call into a procedure, and taking the borrow first
    // would panic the RefCell.
    let appended = match evaluate_node(source, Rc::clone(env), debug) {
        Ok(val) => val,
        Err(err) => return Some(Err(err)),
    };

    // Past this point the right-hand side has already run, so there is no
    // falling back to the ordinary path: that would run its side effects twice.
    if let Some(left) = not_a_string {
        return Some(store_combined(name, left, appended, value, env));
    }
    let Value::String(text) = appended else {
        // `s <- s + 1` and the like: the ordinary path's type error, from the
        // same span, with the right-hand side still evaluated only once.
        return Some(combine_with_current(name, appended, value, env));
    };
    let stored = env
        .borrow_mut()
        .with_var_mut(name, |current| match current {
            Value::String(s) => {
                s.push_str(&text);
                Ok(())
            }
            _ => Err(()),
        });
    if matches!(stored, Some(Ok(()))) {
        return Some(Ok(()));
    }
    // Only reachable if evaluating the right-hand side rebound the target.
    Some(combine_with_current(name, Value::String(text), value, env))
}

/// Combine the target's own value with an already-evaluated right-hand side and
/// store the result, exactly as the ordinary assignment path would have.
fn store_combined(
    name: &str,
    left: Value,
    rhs: Value,
    value: &Spanned,
    env: &Rc<RefCell<Environment>>,
) -> Result<(), Interruption> {
    let combined = match &value.node {
        AstNode::Concat(_, _) => match (&left, &rhs) {
            (Value::String(a), Value::String(b)) => Value::String(format!("{}{}", a, b)),
            _ => {
                return Err(runtime_err(
                    "CONCAT requires string arguments",
                    value.span,
                    env,
                ));
            }
        },
        _ => evaluate_binary_op(&left, &BinaryOperator::Add, &rhs)
            .map_err(|msg| runtime_err(msg, value.span, env))?,
    };
    env.borrow_mut().set(name.to_string(), combined);
    Ok(())
}

/// [`store_combined`] for the paths that have not already copied the target out
/// of the environment.
fn combine_with_current(
    name: &str,
    rhs: Value,
    value: &Spanned,
    env: &Rc<RefCell<Environment>>,
) -> Result<(), Interruption> {
    let current = env.borrow().get(name);
    match current {
        Some(left) => store_combined(name, left, rhs, value, env),
        None => Err(runtime_err(
            format!("Undefined variable: {}", name),
            value.span,
            env,
        )),
    }
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
            // Only the last statement's value survives, so every earlier one
            // runs in statement position (see `evaluate_for_effect`).
            let Some((last, leading)) = statements.split_last() else {
                return Ok(Value::Unit);
            };
            for stmt in leading {
                evaluate_for_effect(stmt, Rc::clone(&env), debug)?;
            }
            evaluate_node(last, Rc::clone(&env), debug)
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

        AstNode::Dictionary(pairs) => {
            let mut entries = Dict::default();
            for (key_expr, value_expr) in pairs {
                let key_val = evaluate_node(key_expr, Rc::clone(&env), debug)?;
                let key = value_to_key(&key_val).map_err(|msg| runtime_err(msg, span, &env))?;
                let value = evaluate_node(value_expr, Rc::clone(&env), debug)?;
                entries.insert(key, value);
            }
            Ok(Value::Dictionary(entries))
        }

        AstNode::Identifier(name) => match env.borrow().get(name) {
            Some(val) => Ok(val),
            None => Err(runtime_err(undefined_variable_message(name), span, &env)),
        },

        AstNode::Assignment(target, value) => {
            let val = evaluate_node(value, Rc::clone(&env), debug)?;
            if let AstNode::Identifier(name) = &target.node {
                if debug {
                    println!("Assigning {} = {:?}", name, val);
                }
                if matches!(&value.node, AstNode::FormattedString(_, _)) {
                    // Capture-only: assigning an f-string has never printed to
                    // the CLI's stdout, it only ever showed up in the captured
                    // output.
                    let output = value_to_string(&val);
                    env.borrow().sink().borrow_mut().record_line(&output);
                }
                env.borrow_mut().set(name.clone(), val.clone());
                Ok(val)
            } else {
                Err(runtime_err("Invalid assignment target", span, &env))
            }
        }

        AstNode::BinaryOp(left_expr, op, right_expr) => match op {
            // Both operands are type-checked, not just the right one: a
            // non-boolean left operand is an error even though the operator
            // short-circuits.
            BinaryOperator::And => {
                let left_val = evaluate_node(left_expr, Rc::clone(&env), debug)?;
                let Value::Boolean(left_bool) = left_val else {
                    return Err(runtime_err(
                        "Left operand of AND must be boolean",
                        span,
                        &env,
                    ));
                };
                if !left_bool {
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
                let Value::Boolean(left_bool) = left_val else {
                    return Err(runtime_err(
                        "Left operand of OR must be boolean",
                        span,
                        &env,
                    ));
                };
                if left_bool {
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
                    evaluate_for_effect(body, Rc::clone(&env), debug)?;
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
                env.borrow().sink().borrow_mut().write_line(&output);
                Ok(result)
            }
            None => {
                env.borrow().sink().borrow_mut().write_line("");
                Ok(Value::Unit)
            }
        },

        AstNode::DisplayInline(expr) => {
            let value = evaluate_node(expr, Rc::clone(&env), debug)?;
            let output = value_to_string(&value);
            env.borrow().sink().borrow_mut().write_str(&output);
            Ok(Value::Unit)
        }

        AstNode::Input(prompt) => {
            #[cfg(any(not(target_arch = "wasm32"), feature = "wasi"))]
            {
                let mut input_str = String::default();

                if let Some(prompt_expr) = prompt {
                    let prompt_val = evaluate_node(prompt_expr, Rc::clone(&env), debug)?;
                    let prompt_str = value_to_string(&prompt_val);
                    // Through the sink, so a buffered stream is drained (and
                    // the prompt is on screen) before we block on stdin.
                    env.borrow().sink().borrow_mut().write_prompt(&prompt_str);
                } else {
                    // No prompt of our own, but the program may have just
                    // DISPLAYINLINEd one. Drain before blocking either way.
                    env.borrow().sink().borrow_mut().flush();
                }

                io::stdin()
                    .read_line(&mut input_str)
                    .map_err(|e| runtime_err(e.to_string(), span, &env))?;
                let input = input_str.trim().to_string();

                if prompt.is_none() {
                    // Capture-only echo, as before: the terminal has already
                    // echoed what the user typed.
                    env.borrow().sink().borrow_mut().record_line(&input);
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
                    env.borrow().sink().borrow_mut().record_line(&input);
                }

                Ok(Value::String(input))
            }
        }

        AstNode::ProcedureDecl(name, params, body) => {
            let declared_in = env.borrow().modules.borrow().current_file();
            env.borrow_mut().declare_procedure(
                name.clone(),
                Rc::new((params.clone(), (**body).clone(), declared_in)),
            );
            Ok(Value::Unit)
        }

        AstNode::ProcedureCall(name, args) => {
            if let Some(result) = eval_builtin(name, args, &env, span, debug) {
                return result;
            }
            // Arguments are evaluated in the caller's scope, before the callee's
            // scope exists, which is what `invoke_procedure` then binds them in.
            let mut arg_values = Vec::with_capacity(args.len());
            for arg in args {
                arg_values.push(evaluate_node(arg, Rc::clone(&env), debug)?);
            }
            invoke_procedure(name, arg_values, &env, span, debug)
        }

        AstNode::ListAccess(list, index) => {
            if let Some(result) = eval_indexed_read_in_place(list, index, &env, span, debug) {
                return result;
            }
            let current_value = evaluate_node(list, Rc::clone(&env), debug)?;
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;
            index_value(&current_value, &index_val, span, &env)
        }

        AstNode::ListAssignment(list, index, value) => {
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;
            let new_val = evaluate_node(value, Rc::clone(&env), debug)?;
            let ret = new_val.clone();
            assign_indexed(list, index_val, new_val, &env, span, debug)?;
            Ok(ret)
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
                    (Some(si), Some(ei)) if !start_idx.is_negative() => char_range(&s, si, ei)
                        .map(|slice| Value::String(slice.to_string()))
                        .ok_or_else(|| runtime_err("Invalid substring indices", span, &env)),
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

                evaluate_for_effect(body, Rc::clone(&env), debug)?;

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
                    let count = elements.len();
                    for (i, element) in elements.into_iter().enumerate() {
                        env.borrow_mut().set(var_name.clone(), element);
                        result = evaluate_loop_body(body, Rc::clone(&env), debug, i + 1 == count)?;
                    }
                    Ok(result)
                }
                Value::String(s) => {
                    let mut result = Value::Unit;
                    let count = s.chars().count();
                    for (i, c) in s.chars().enumerate() {
                        env.borrow_mut()
                            .set(var_name.clone(), Value::String(c.to_string()));
                        result = evaluate_loop_body(body, Rc::clone(&env), debug, i + 1 == count)?;
                    }
                    Ok(result)
                }
                Value::Dictionary(entries) => {
                    let mut result = Value::Unit;
                    let count = entries.len();
                    for (i, (key, _)) in entries.iter().enumerate() {
                        env.borrow_mut().set(var_name.clone(), key_to_value(key));
                        result = evaluate_loop_body(body, Rc::clone(&env), debug, i + 1 == count)?;
                    }
                    Ok(result)
                }
                _ => Err(runtime_err(
                    "FOR EACH requires list, string, or dictionary",
                    span,
                    &env,
                )),
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

        AstNode::Length(list) => with_value(list, &env, debug, container_len)?
            .map(|len| Value::Integer(BigInt::from(len)))
            .ok_or_else(|| {
                runtime_err(
                    "LENGTH requires a list, string, or dictionary argument",
                    span,
                    &env,
                )
            }),

        AstNode::Insert(list, index, value) => {
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;
            let insert_val = evaluate_node(value, Rc::clone(&env), debug)?;

            if let AstNode::Identifier(name) = &list.node {
                if !env
                    .borrow()
                    .with_var(name, |value| matches!(value, Value::List(_)))
                    .unwrap_or(false)
                {
                    return Err(runtime_err(
                        format!("Variable {} is not a list", name),
                        span,
                        &env,
                    ));
                }

                if let Value::Integer(i) = index_val {
                    let idx = &i - BigInt::one();
                    let outcome = env.borrow_mut().with_var_mut(name, |value| {
                        let Value::List(elements) = value else {
                            return Err(());
                        };
                        match idx.to_usize() {
                            Some(uidx) if !idx.is_negative() && uidx <= elements.len() => {
                                elements.insert(uidx, insert_val.clone());
                                Ok(())
                            }
                            _ => Err(()),
                        }
                    });
                    match outcome {
                        Some(Ok(())) => Ok(insert_val),
                        _ => Err(runtime_err("List index out of bounds", span, &env)),
                    }
                } else {
                    Err(runtime_err("Invalid list index", span, &env))
                }
            } else {
                Err(runtime_err("INSERT requires a list variable", span, &env))
            }
        }

        AstNode::Append(list, value) => {
            let append_val = evaluate_node(value, Rc::clone(&env), debug)?;

            if let AstNode::Identifier(name) = &list.node {
                let outcome = env.borrow_mut().with_var_mut(name, |value| {
                    let Value::List(elements) = value else {
                        return Err(());
                    };
                    elements.push(append_val.clone());
                    Ok(())
                });
                match outcome {
                    Some(Ok(())) => Ok(append_val),
                    _ => Err(runtime_err(
                        format!("Variable {} is not a list", name),
                        span,
                        &env,
                    )),
                }
            } else {
                Err(runtime_err("APPEND requires a list variable", span, &env))
            }
        }

        AstNode::Remove(list, index) => {
            let index_val = evaluate_node(index, Rc::clone(&env), debug)?;

            if let AstNode::Identifier(name) = &list.node {
                let kind = env.borrow().with_var(name, |value| match value {
                    Value::Dictionary(_) => ContainerKind::Dictionary,
                    Value::List(_) => ContainerKind::List,
                    _ => ContainerKind::Other,
                });
                match kind {
                    Some(ContainerKind::Dictionary) => {
                        return dict_remove_entry(name, &index_val, &env, span);
                    }
                    Some(ContainerKind::List) => {}
                    _ => {
                        return Err(runtime_err(
                            format!("Variable {} is not a list", name),
                            span,
                            &env,
                        ));
                    }
                }

                if let Value::Integer(i) = index_val {
                    let idx = &i - BigInt::one();
                    let outcome = env.borrow_mut().with_var_mut(name, |value| {
                        let Value::List(elements) = value else {
                            return Err(());
                        };
                        match idx.to_usize() {
                            Some(uidx) if !idx.is_negative() && uidx < elements.len() => {
                                Ok(elements.remove(uidx))
                            }
                            _ => Err(()),
                        }
                    });
                    match outcome {
                        Some(Ok(removed_value)) => Ok(removed_value),
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

        AstNode::Import(path) => eval_import(path, &env, span, debug),

        AstNode::Return(expr) => {
            let value = evaluate_node(expr, Rc::clone(&env), debug)?;
            Err(Interruption::Return(value))
        }

        AstNode::Sort(list_expr) => {
            let list_val = evaluate_node(list_expr, Rc::clone(&env), debug)?;
            if let Value::List(mut elements) = list_val {
                elements.sort_by(sort_cmp);
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
            // EXIT is not an error, so CATCH must let it through.
            Err(Interruption::Exit(code)) => Err(Interruption::Exit(code)),
            Err(Interruption::Error(error)) => {
                // The catch block runs in the current scope, like `IF`, `FOR EACH`,
                // `REPEAT` and the TRY block. A child scope threw the assignments
                // away, and
                //
                //     TRY { config <- READFILE(p) } CATCH (e) { config <- "" }
                //     DISPLAY(config)
                //
                // failed with "Undefined variable: config". Only the error
                // variable is scoped to the block, and whatever it shadowed in
                // this scope is put back afterwards.
                let shadowed = error_var
                    .as_ref()
                    .map(|name| (name.clone(), env.borrow().local(name)));
                if let Some(var_name) = error_var {
                    env.borrow_mut()
                        .set(var_name.clone(), Value::String(error.message));
                }
                let result = evaluate_node(catch_block, Rc::clone(&env), debug);
                // Restored on the error path too, so a CATCH that itself fails, or
                // that RETURNs, does not leave the error variable behind.
                if let Some((name, previous)) = shadowed {
                    match previous {
                        Some(value) => env.borrow_mut().set(name, value),
                        None => env.borrow_mut().remove_local(&name),
                    }
                }
                result
            }
        },

        AstNode::Eval(expr) => {
            let expr_val = evaluate_node(expr, Rc::clone(&env), debug)?;
            if let Value::String(s) = expr_val {
                // Frame-guarded: `code <- "EVAL(code)"` would otherwise recurse
                // through the real stack without ever touching MAX_STACK_DEPTH.
                with_meta_frame("EVAL", &s, &env, span, || {
                    let mut lexer = crate::lexer::Lexer::new(&s);
                    let tokens = lexer.tokenize();
                    let mut parser = crate::parser::Parser::new(tokens);
                    let ast = parser.parse_expression(debug).map_err(|mut e| {
                        e.span = Some(span);
                        Interruption::Error(e)
                    })?;

                    evaluate_node(&ast, Rc::clone(&env), debug)
                })
            } else {
                Err(runtime_err("EVAL requires a string argument", span, &env))
            }
        }

        AstNode::Comment => Ok(Value::Unit),
    }
}

/// Call a user-declared procedure with arguments that have already been
/// evaluated.
///
/// Split out of `AstNode::ProcedureCall` so that CALL can dispatch by name at
/// runtime and get identical semantics -- the same recursion guard, the same
/// stack frame for error traces, and the same treatment of a `RETURN` as the
/// call's value.
fn invoke_procedure(
    name: &str,
    args: Vec<Value>,
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if env.borrow().stack_depth() >= MAX_STACK_DEPTH {
        return Err(runtime_err(
            "Stack overflow: maximum recursion depth exceeded",
            span,
            env,
        ));
    }
    let procedure = env
        .borrow()
        .get_procedure(name)
        .ok_or_else(|| runtime_err(format!("Procedure '{}' not found", name), span, env))?;
    let local_env = Rc::new(RefCell::new(Environment::new_with_parent(Rc::clone(env))));
    let (params, body, declared_in) = (&procedure.0, &procedure.1, &procedure.2);
    // `zip` stops at the shorter side, so a call with too few arguments leaves
    // the remaining parameters unbound and extra arguments are ignored -- the
    // behaviour this interpreter has always had.
    for (param, arg) in params.iter().zip(args) {
        local_env.borrow_mut().set(param.clone(), arg);
    }

    // Enter the procedure's own file for the duration of the call, so SCRIPTPATH
    // and a relative IMPORT inside the body see where the code was *written*
    // rather than where it was called from. Skipped -- to a single pointer
    // comparison -- when the procedure lives in the file already executing, which
    // is every call in a one-file program.
    let modules = Rc::clone(&env.borrow().modules);
    let entered_file = match declared_in {
        Some(file) => {
            let already_here = modules.borrow().is_current(file);
            if already_here {
                false
            } else {
                modules.borrow_mut().stack.push(Rc::clone(file));
                true
            }
        }
        None => false,
    };

    env.borrow().push_frame(StackFrame {
        name: name.to_string(),
        span,
    });
    let body_result = evaluate_node(body, Rc::clone(&local_env), debug);
    env.borrow().pop_frame();
    if entered_file {
        modules.borrow_mut().stack.pop();
    }
    // No output copy-up: `local_env` shares the caller's sink, so the callee's
    // writes are already in place, in order.
    match body_result {
        Err(Interruption::Return(val)) => Ok(val),
        other => other,
    }
}

/// Find the file an IMPORT names.
///
/// A relative path is resolved against the directory of the file doing the
/// importing first, and only then against the process's working directory. That
/// ordering is what lets a library sit next to its own dependencies and be
/// imported correctly no matter where `fpli` was launched from, while still
/// keeping the old working-directory behaviour as a fallback. A `.psl` extension
/// is appended when the name has none, so `IMPORT "strings"` and
/// `IMPORT "strings.psl"` mean the same file.
///
/// The returned path is canonicalised so that `lib.psl`, `./lib.psl` and an
/// absolute spelling of the same file are recognised as one module.
fn resolve_import(
    path: &str,
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> Result<PathBuf, Interruption> {
    let requested = Path::new(path);
    // Directories to try the name against, in order. An absolute path has none: it
    // already names where to look.
    let mut bases: Vec<PathBuf> = Vec::new();
    if !requested.is_absolute() {
        let importer = env.borrow().modules.borrow().current_file();
        if let Some(dir) = importer.as_deref().and_then(|p| p.parent()) {
            bases.push(dir.to_path_buf());
        }
        bases.push(PathBuf::from("."));
    }

    let mut tried: Vec<String> = Vec::new();
    for joined in std::iter::once(requested.to_path_buf())
        .filter(|_| requested.is_absolute())
        .chain(bases.iter().map(|base| base.join(requested)))
    {
        // The bare name first, so an explicit extension always wins over the
        // implicit one.
        let mut candidates = vec![joined.clone()];
        if requested.extension().is_none() {
            candidates.push(joined.with_extension("psl"));
        }
        for candidate in candidates {
            if candidate.is_file() {
                // Canonicalised, so that every spelling of one file -- `lib.psl`,
                // `./lib.psl`, an absolute path, a symlink -- is recognised as the
                // same module. `strip_unc` keeps Windows' `\\?\` prefix out of the
                // paths MODULES and SCRIPTPATH hand back.
                return std::fs::canonicalize(&candidate)
                    .map(|resolved| PathBuf::from(system::strip_unc(&resolved)))
                    .map_err(|e| {
                        runtime_err(
                            format!("Could not resolve import '{}': {}", candidate.display(), e),
                            span,
                            env,
                        )
                    });
            }
            tried.push(candidate.display().to_string());
        }
    }

    Err(runtime_err(
        format!(
            "Could not find imported file '{}'. Tried: {}",
            path,
            tried.join(", ")
        ),
        span,
        env,
    ))
}

/// Load and run an imported file, at most once per run.
///
/// The file's declarations land in the importing scope, which is the flat
/// namespace PseudoLang has always had. Everything else about IMPORT is new: the
/// path resolution above, the once-only guarantee, and the file stack that lets
/// a library import its own neighbours.
fn eval_import(path: &str, env: &Rc<RefCell<Environment>>, span: Span, debug: bool) -> EvalResult {
    let resolved = resolve_import(path, env, span)?;

    // Recorded before the body runs, so a cycle finds the file already loaded
    // and stops instead of recursing.
    {
        let modules = Rc::clone(&env.borrow().modules);
        let mut modules = modules.borrow_mut();
        // The entry script counts as already running even though it was never
        // "imported": a library that imports the file it was itself imported from
        // must not restart it. Without this, importing the entry script re-ran its
        // whole top level -- including any ISMAIN block -- half-way through the
        // first run, before the declarations that block depends on existed.
        if modules.loaded.contains(&resolved) || modules.entry.as_deref() == Some(&resolved) {
            return Ok(Value::Unit);
        }
        modules.loaded.push(resolved.clone());
        modules.stack.push(Rc::new(resolved.clone()));
    }

    // The imported file's declarations go into the *root* scope, not the scope the
    // IMPORT was written in. A flat namespace is what the language documents, and an
    // IMPORT inside a procedure body or a CATCH block would otherwise declare
    // everything into that block's scope and lose it all on the way out -- while
    // still marking the file loaded, so a later top-level IMPORT of the same file
    // became a silent no-op and its names were unreachable for the rest of the run.
    let root = root_env(env);

    let result = (|| -> EvalResult {
        let content = std::fs::read_to_string(&resolved).map_err(|e| {
            runtime_err(
                format!("Failed to read imported file {}: {}", resolved.display(), e),
                span,
                env,
            )
        })?;

        let mut lexer = crate::lexer::Lexer::new(&content);
        let tokens = lexer.tokenize();
        let imported_ast = crate::parser::parse(tokens, debug).map_err(|e| {
            runtime_err(
                format!(
                    "Failed to parse imported file {}: {}",
                    resolved.display(),
                    e.format(&content)
                ),
                span,
                env,
            )
        })?;

        env.borrow()
            .modules
            .borrow_mut()
            .sources
            .insert(resolved.clone(), Rc::from(content.as_str()));
        let outcome = evaluate_node(&imported_ast, Rc::clone(&root), debug);
        // Procedure tables are snapshotted into each scope when it is created, so a
        // scope that already existed when the import ran would not see the names the
        // import just declared -- which is what an IMPORT inside a procedure body
        // hits. Merge the root's table down the chain to the importing scope.
        let table = root.borrow().procedure_table();
        let mut scope = Rc::clone(env);
        loop {
            if Rc::ptr_eq(&scope, &root) {
                break;
            }
            scope.borrow_mut().merge_procedures(&table);
            let parent = scope.borrow().parent.clone();
            match parent {
                Some(parent) => scope = parent,
                None => break,
            }
        }
        match outcome {
            // A top-level RETURN ends the imported file, nothing more. Letting it
            // through made it unwind the *importing* program: everything after the
            // IMPORT was skipped and the run ended silently with status 0.
            Ok(_) | Err(Interruption::Return(_)) => Ok(Value::Unit),
            // The error already carries the file and source it came from, so it is
            // passed through untouched. Re-wrapping it here nested the diagnostic once
            // per level of import.
            Err(other) => Err(other),
        }
    })();

    let modules = Rc::clone(&env.borrow().modules);
    {
        let mut modules = modules.borrow_mut();
        // Popped on the error path too: a failed import must not leave the stack
        // claiming that file is still executing.
        modules.stack.pop();
        // A file whose body failed is *not* loaded. Leaving it recorded made the
        // failure permanent: a retry after fixing the cause silently did nothing,
        // and MODULES() listed a module that never finished.
        if result.is_err() {
            modules.loaded.retain(|loaded| loaded != &resolved);
        }
    }
    result
}

/// The outermost scope of the run: where IMPORT puts what it declares.
fn root_env(env: &Rc<RefCell<Environment>>) -> Rc<RefCell<Environment>> {
    let mut current = Rc::clone(env);
    loop {
        let parent = current.borrow().parent.clone();
        match parent {
            Some(parent) => current = parent,
            None => return current,
        }
    }
}

/// Remove `key_val` from the dictionary held by variable `name` in place,
/// returning the removed value. Callers have already established that `name` is
/// bound to a dictionary.
fn dict_remove_entry(
    name: &str,
    key_val: &Value,
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> EvalResult {
    let key = value_to_key(key_val).map_err(|msg| runtime_err(msg, span, env))?;
    let outcome = env.borrow_mut().with_var_mut(name, |value| {
        let Value::Dictionary(entries) = value else {
            return Err(format!("Variable {} is not a dictionary", name));
        };
        match entries.remove(&key) {
            Some(removed) => Ok(removed),
            None => Err(format!("Key not found: {}", key_to_string(&key))),
        }
    });
    match outcome {
        Some(Ok(removed)) => Ok(removed),
        Some(Err(msg)) => Err(runtime_err(msg, span, env)),
        None => Err(runtime_err(
            format!("Variable {} is not a dictionary", name),
            span,
            env,
        )),
    }
}

/// Write `new_val` at `index` inside a list or dictionary, in place. Missing
/// list indices are an error; missing dictionary keys are created.
fn container_set(container: &mut Value, index: &Value, new_val: Value) -> Result<(), String> {
    match container {
        Value::List(elements) => {
            if let Value::Integer(i) = index {
                let idx = i - BigInt::one();
                match idx.to_usize() {
                    Some(uidx) if uidx < elements.len() => {
                        elements[uidx] = new_val;
                        Ok(())
                    }
                    _ => Err("List index out of bounds".to_string()),
                }
            } else {
                Err("Invalid list index".to_string())
            }
        }
        Value::Dictionary(entries) => {
            let key = value_to_key(index)?;
            entries.insert(key, new_val);
            Ok(())
        }
        _ => Err("Invalid index assignment - expected list or dictionary".to_string()),
    }
}

/// Borrow `container[index]` mutably while walking an assignment path. Only
/// lists and dictionaries can appear as intermediate containers.
fn container_get_mut<'a>(container: &'a mut Value, index: &Value) -> Result<&'a mut Value, String> {
    match container {
        Value::List(elements) => {
            if let Value::Integer(i) = index {
                let idx = i - BigInt::one();
                match idx.to_usize() {
                    Some(uidx) if uidx < elements.len() => Ok(&mut elements[uidx]),
                    _ => Err("List index out of bounds".to_string()),
                }
            } else {
                Err("Invalid list index".to_string())
            }
        }
        Value::Dictionary(entries) => {
            let key = value_to_key(index)?;
            match entries.get_mut(&key) {
                Some(value) => Ok(value),
                None => Err(format!("Key not found: {}", key_to_string(&key))),
            }
        }
        _ => Err("Invalid index assignment - expected list or dictionary".to_string()),
    }
}

/// Borrow `container[index]` in place for a read.
///
/// `Ok(None)` means the container is not one whose elements can be handed out
/// by reference -- indexing a string materialises a fresh one-character value
/// -- so the caller falls back to [`index_value`]. Every error message matches
/// the general read path exactly.
fn index_ref<'a>(
    container: &'a Value,
    index: &Value,
    span: Span,
    env: &Rc<RefCell<Environment>>,
) -> Result<Option<&'a Value>, Interruption> {
    if let Value::Dictionary(entries) = container {
        let key = value_to_key(index).map_err(|msg| runtime_err(msg, span, env))?;
        return match entries.get(&key) {
            Some(value) => Ok(Some(value)),
            None => Err(runtime_err(
                format!("Key not found: {}", key_to_string(&key)),
                span,
                env,
            )),
        };
    }
    match (container, index) {
        (Value::List(elements), Value::Integer(i)) => {
            let idx = i - BigInt::one();
            if idx.is_negative() {
                Err(runtime_err(
                    "List index out of bounds: index cannot be less than 1",
                    span,
                    env,
                ))
            } else {
                let uidx = idx
                    .to_usize()
                    .ok_or_else(|| runtime_err("List index too large", span, env))?;
                if uidx >= elements.len() {
                    Err(runtime_err(
                        format!("List index out of bounds: {} (size: {})", i, elements.len()),
                        span,
                        env,
                    ))
                } else {
                    Ok(Some(&elements[uidx]))
                }
            }
        }
        _ => Ok(None),
    }
}

/// Read `container[index]`, cloning only the element that was selected.
fn index_value(
    container: &Value,
    index: &Value,
    span: Span,
    env: &Rc<RefCell<Environment>>,
) -> EvalResult {
    if let Some(value) = index_ref(container, index, span, env)? {
        return Ok(value.clone());
    }
    match (container, index) {
        (Value::String(s), Value::Integer(i)) => {
            let idx = i - BigInt::one();
            if idx.is_negative() {
                Err(runtime_err(
                    "String index out of bounds: index cannot be less than 1",
                    span,
                    env,
                ))
            } else {
                let uidx = idx
                    .to_usize()
                    .ok_or_else(|| runtime_err("String index too large", span, env))?;
                match s.chars().nth(uidx) {
                    Some(ch) => Ok(Value::String(ch.to_string())),
                    None => Err(runtime_err(
                        format!(
                            "String index out of bounds: {} (size: {})",
                            i,
                            str_char_len(s)
                        ),
                        span,
                        env,
                    )),
                }
            }
        }
        _ => Err(runtime_err(
            "Invalid index access - expected list or string and integer index",
            span,
            env,
        )),
    }
}

/// Follow an `a[i][j]...` chain, descending through intermediate containers by
/// reference so that only the finally selected element is cloned. Each step
/// carries the span of the access it came from, so errors are reported against
/// the same node the general path would blame.
fn index_chain(
    container: &Value,
    steps: &[(Value, Span)],
    env: &Rc<RefCell<Environment>>,
) -> EvalResult {
    let Some(((index, step_span), rest)) = steps.split_first() else {
        return Ok(container.clone());
    };
    if rest.is_empty() {
        return index_value(container, index, *step_span, env);
    }
    match index_ref(container, index, *step_span, env)? {
        Some(inner) => index_chain(inner, rest, env),
        None => {
            let inner = index_value(container, index, *step_span, env)?;
            index_chain(&inner, rest, env)
        }
    }
}

/// Walk an index chain purely to surface its errors, cloning nothing.
///
/// The general path evaluates an index expression only after the access to its
/// left has succeeded; the in-place path evaluates them one at a time and calls
/// this in between, so a bad index still wins over an error in the next index
/// expression.
fn check_index_prefix(
    container: &Value,
    steps: &[(Value, Span)],
    env: &Rc<RefCell<Environment>>,
) -> Result<(), Interruption> {
    let Some(((index, step_span), rest)) = steps.split_first() else {
        return Ok(());
    };
    match index_ref(container, index, *step_span, env)? {
        Some(inner) => check_index_prefix(inner, rest, env),
        None => {
            let inner = index_value(container, index, *step_span, env)?;
            check_index_prefix(&inner, rest, env)
        }
    }
}

/// Conservative purity test: true only for expression forms that cannot write
/// to the environment.
///
/// Reading a container in place skips the defensive copy the general path makes
/// before evaluating the index, which is only sound when evaluating the index
/// cannot modify the container being read.
fn is_side_effect_free(node: &AstNode) -> bool {
    match node {
        AstNode::Integer(_)
        | AstNode::Float(_)
        | AstNode::String(_)
        | AstNode::RawString(_)
        | AstNode::Boolean(_)
        | AstNode::Null
        | AstNode::NaN
        | AstNode::Identifier(_) => true,
        AstNode::BinaryOp(left, _, right) => {
            is_side_effect_free(&left.node) && is_side_effect_free(&right.node)
        }
        AstNode::UnaryOp(_, expr) => is_side_effect_free(&expr.node),
        AstNode::ListAccess(base, index) => {
            is_side_effect_free(&base.node) && is_side_effect_free(&index.node)
        }
        AstNode::Length(expr) => is_side_effect_free(&expr.node),
        _ => false,
    }
}

/// Fast path for `name[i]`, `name[i][j]`, ...: select the element inside the
/// container where it lives, instead of cloning the whole container once per
/// index level.
///
/// Returns `None` when the access does not qualify and the general path should
/// run. It requires every index expression to be side-effect free, because the
/// general path snapshots the container before evaluating them and this does
/// not.
fn eval_indexed_read_in_place(
    list: &Spanned,
    index: &Spanned,
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> Option<EvalResult> {
    // `name[i]` is by far the most common shape and is worth keeping free of
    // the bookkeeping (and heap traffic) the general chain below needs.
    if let AstNode::Identifier(name) = &list.node {
        if !is_side_effect_free(&index.node) {
            return None;
        }
        if let Some(err) = undefined_variable_error(name, list.span, env) {
            return Some(Err(err));
        }
        let index_val = match evaluate_node(index, Rc::clone(env), debug) {
            Ok(value) => value,
            Err(interruption) => return Some(Err(interruption)),
        };
        let borrowed = env.borrow();
        return borrowed.with_var(name, |value| index_value(value, &index_val, span, env));
    }

    let mut steps: Vec<(&Spanned, Span)> = vec![(index, span)];
    let mut current = list;
    let (name, name_span) = loop {
        match &current.node {
            AstNode::Identifier(name) => break (name, current.span),
            AstNode::ListAccess(base, base_index) => {
                steps.push((base_index, current.span));
                current = base;
            }
            _ => return None,
        }
    };
    steps.reverse();
    if !steps
        .iter()
        .all(|(expr, _)| is_side_effect_free(&expr.node))
    {
        return None;
    }
    if let Some(err) = undefined_variable_error(name, name_span, env) {
        return Some(Err(err));
    }

    let mut evaluated: Vec<(Value, Span)> = Vec::with_capacity(steps.len());
    for (position, (expr, step_span)) in steps.iter().enumerate() {
        match evaluate_node(expr, Rc::clone(env), debug) {
            Ok(value) => evaluated.push((value, *step_span)),
            Err(interruption) => return Some(Err(interruption)),
        }
        if position + 1 < steps.len() {
            let borrowed = env.borrow();
            let checked =
                borrowed.with_var(name, |value| check_index_prefix(value, &evaluated, env));
            drop(borrowed);
            if let Some(Err(interruption)) = checked {
                return Some(Err(interruption));
            }
        }
    }
    let borrowed = env.borrow();
    borrowed.with_var(name, |value| index_chain(value, &evaluated, env))
}

/// The general path evaluates the base of an access first, so an unbound name
/// has to be reported before any index expression runs.
fn undefined_variable_error(
    name: &str,
    span: Span,
    env: &Rc<RefCell<Environment>>,
) -> Option<Interruption> {
    if env.borrow().with_var(name, |_| ()).is_some() {
        return None;
    }
    Some(runtime_err(undefined_variable_message(name), span, env))
}

/// What to say about a name nothing is bound to.
///
/// Most of the built-ins take no arguments, so leaving off the parentheses is an easy
/// slip, and "Undefined variable: CWD" gives no hint that `CWD()` was meant.
fn undefined_variable_message(name: &str) -> String {
    if is_builtin_name(name) {
        format!(
            "Undefined variable: {}. '{}' is a built-in function; write {}() to call it",
            name, name, name
        )
    } else {
        format!("Undefined variable: {}", name)
    }
}

/// Whether `name` is a built-in function.
///
/// A plain list rather than a probe call: dispatching for real would run the
/// built-in, and answering "is PROCESSES a name I know" should not enumerate every
/// process. `test_every_builtin_is_listed_for_the_undefined_variable_hint` reads the
/// dispatcher's own match arms and fails if this list falls behind.
/// Every name [`eval_builtin`] dispatches, sorted for `binary_search`.
const BUILTIN_NAMES: &[&str] = &[
    "ABS",
    "ABSPATH",
    "ACOS",
    "APPENDFILE",
    "ARCH",
    "ASIN",
    "ATAN",
    "BASENAME",
    "CACHEDIR",
    "CALL",
    "CEIL",
    "CHDIR",
    "CONFIGDIR",
    "CONTAINS",
    "COPYFILE",
    "COS",
    "CPUCOUNT",
    "CWD",
    "DATADIR",
    "DEGREES",
    "DELETEDIR",
    "DELETEFILE",
    "DELETETREE",
    "DICTIONARY",
    "DIRNAME",
    "ENDSWITH",
    "ENVVARS",
    "EXEC",
    "EXECUTE",
    "EXIT",
    "EXP",
    "EXTENSION",
    "FACTORIAL",
    "FILEEXISTS",
    "FILEMTIME",
    "FILESIZE",
    "FIND",
    "FLOOR",
    "GCD",
    "GETARG",
    "GETENV",
    "GETKEY",
    "GETVAR",
    "HASARG",
    "HASKEY",
    "HOMEDIR",
    "HOSTNAME",
    "HYPOT",
    "ISDEFINED",
    "ISDIR",
    "ISFILE",
    "ISMAIN",
    "JOINPATH",
    "KERNELVERSION",
    "KEYS",
    "KILL",
    "LISTDIR",
    "LOGTEN",
    "LOGTWO",
    "LOWERCASE",
    "MAKEDIR",
    "MAX",
    "MILLITIME",
    "MIN",
    "MODULES",
    "OSFAMILY",
    "OSNAME",
    "OSVERSION",
    "PHYSICALCPUS",
    "PID",
    "PLATFORM",
    "POW",
    "PROCEDURES",
    "PROCESSES",
    "PROCESSINFO",
    "RADIANS",
    "RANGE",
    "READFILE",
    "READLINES",
    "REALPATH",
    "REMOVEKEY",
    "RENAME",
    "REPLACE",
    "ROUND",
    "SCRIPTPATH",
    "SETENV",
    "SETKEY",
    "SETVAR",
    "SHELL",
    "SIN",
    "SLEEP",
    "SPLIT",
    "SQRT",
    "STARTSWITH",
    "SYSINFO",
    "TAN",
    "TEMPDIR",
    "TIME",
    "TIMESTAMP",
    "TIMEZONE",
    "TIMEZONES",
    "TOTALMEMORY",
    "TRIM",
    "TYPEOF",
    "UNSETENV",
    "UNSETVAR",
    "UPPERCASE",
    "UPTIME",
    "USEDMEMORY",
    "USERNAME",
    "VALUES",
    "VARIABLES",
    "VERSION",
    "WHICH",
    "WRITEFILE",
];

fn is_builtin_name(name: &str) -> bool {
    BUILTIN_NAMES.binary_search(&name).is_ok()
}

/// Evaluate `expr` and hand the result to `f` by reference, skipping the copy
/// when the expression is nothing but a variable name.
///
/// In that case `f` runs while the owning scope is immutably borrowed, so it
/// must not evaluate anything that could mutate a scope.
fn with_value<R>(
    expr: &Spanned,
    env: &Rc<RefCell<Environment>>,
    debug: bool,
    f: impl FnOnce(&Value) -> R,
) -> Result<R, Interruption> {
    if let AstNode::Identifier(name) = &expr.node {
        let borrowed = env.borrow();
        return match borrowed.with_var(name, f) {
            Some(result) => Ok(result),
            None => {
                drop(borrowed);
                Err(runtime_err(
                    format!("Undefined variable: {}", name),
                    expr.span,
                    env,
                ))
            }
        };
    }
    let value = evaluate_node(expr, Rc::clone(env), debug)?;
    Ok(f(&value))
}

/// Write `new_val` into `container` at the nested location named by `path`,
/// which is ordered from the root container outward, descending in place.
fn set_in_path(container: &mut Value, path: &[Value], new_val: Value) -> Result<(), String> {
    let (index, rest) = path
        .split_first()
        .ok_or_else(|| "Invalid list assignment target".to_string())?;
    if rest.is_empty() {
        return container_set(container, index, new_val);
    }
    let inner = container_get_mut(container, index)?;
    set_in_path(inner, rest, new_val)
}

/// Assign into `target[index_val]`, where `target` is either a variable or
/// itself an indexed access. Nested paths of any depth are followed in place
/// from the variable at the root of the path. Every index expression along the
/// path is evaluated exactly once, so indices with side effects (a procedure
/// call, RANDOM, INPUT) read and write the same slot.
///
/// The index and the assigned value are both fully evaluated before the root
/// container is borrowed mutably, which is what lets `a[i] <- a[j]` work.
fn assign_indexed(
    target: &Spanned,
    index_val: Value,
    new_val: Value,
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> Result<(), Interruption> {
    let mut path = vec![index_val];
    let mut current = target;
    let name = loop {
        match &current.node {
            AstNode::Identifier(name) => break name.clone(),
            AstNode::ListAccess(inner_target, inner_index) => {
                path.push(evaluate_node(inner_index, Rc::clone(env), debug)?);
                current = inner_target;
            }
            _ => return Err(runtime_err("Invalid list assignment target", span, env)),
        }
    };
    path.reverse();

    let not_a_container = || format!("Variable {} is not a list or dictionary", name);
    let outcome = env.borrow_mut().with_var_mut(&name, |container| {
        if !matches!(container, Value::List(_) | Value::Dictionary(_)) {
            return Err(not_a_container());
        }
        set_in_path(container, &path, new_val)
    });
    match outcome {
        Some(Ok(())) => Ok(()),
        Some(Err(msg)) => Err(runtime_err(msg, span, env)),
        None => Err(runtime_err(not_a_container(), span, env)),
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
        "EXIT" => Some(eval_builtin_exit(args, env, span, debug)),
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
        "DICTIONARY" => Some(eval_builtin_dictionary(args, env, span)),
        "KEYS" => Some(eval_builtin_keys(args, env, span, debug)),
        "VALUES" => Some(eval_builtin_values(args, env, span, debug)),
        "HASKEY" => Some(eval_builtin_haskey(args, env, span, debug)),
        "GETKEY" => Some(eval_builtin_getkey(args, env, span, debug)),
        "SETKEY" => Some(eval_builtin_setkey(args, env, span, debug)),
        "REMOVEKEY" => Some(eval_builtin_removekey(args, env, span, debug)),
        "HASARG" => Some(eval_builtin_hasarg(args, env, span, debug)),
        "GETARG" => Some(eval_builtin_getarg(args, env, span, debug)),
        "READFILE" => Some(eval_builtin_readfile(args, env, span, debug)),
        "READLINES" => Some(eval_builtin_readlines(args, env, span, debug)),
        "WRITEFILE" => Some(eval_builtin_writefile(args, env, span, debug)),
        "APPENDFILE" => Some(eval_builtin_appendfile(args, env, span, debug)),
        "FILEEXISTS" => Some(eval_builtin_fileexists(args, env, span, debug)),
        "FILESIZE" => Some(eval_builtin_filesize(args, env, span, debug)),
        "DELETEFILE" => Some(eval_builtin_deletefile(args, env, span, debug)),
        "LISTDIR" => Some(eval_builtin_listdir(args, env, span, debug)),
        "MAKEDIR" => Some(eval_builtin_makedir(args, env, span, debug)),
        "DELETEDIR" => Some(eval_builtin_deletedir(args, env, span, debug)),
        "DELETETREE" => Some(eval_builtin_deletetree(args, env, span, debug)),
        "FILEMTIME" => Some(eval_builtin_filemtime(args, env, span, debug)),
        "RENAME" => Some(eval_builtin_rename(args, env, span, debug)),
        "COPYFILE" => Some(eval_builtin_copyfile(args, env, span, debug)),

        "GETENV" => Some(eval_builtin_getenv(args, env, span, debug)),
        "SETENV" => Some(eval_builtin_setenv(args, env, span, debug)),
        "UNSETENV" => Some(eval_builtin_unsetenv(args, env, span, debug)),
        "ENVVARS" => Some(eval_builtin_envvars(args, env, span)),

        "EXEC" => Some(eval_builtin_exec(args, env, span, debug)),
        "SHELL" => Some(eval_builtin_shell(args, env, span, debug)),
        "WHICH" => Some(eval_builtin_which(args, env, span, debug)),

        "PID" => Some(eval_builtin_pid(args, env, span)),
        "KILL" => Some(eval_builtin_kill(args, env, span, debug)),
        "PROCESSINFO" => Some(eval_builtin_processinfo(args, env, span, debug)),
        "PROCESSES" => Some(eval_builtin_processes(args, env, span)),

        "CWD" => Some(eval_builtin_cwd(args, env, span)),
        "CHDIR" => Some(eval_builtin_chdir(args, env, span, debug)),
        "JOINPATH" => Some(eval_builtin_joinpath(args, env, span, debug)),
        "BASENAME" => Some(eval_builtin_path_part(args, env, span, debug, "BASENAME")),
        "DIRNAME" => Some(eval_builtin_path_part(args, env, span, debug, "DIRNAME")),
        "EXTENSION" => Some(eval_builtin_path_part(args, env, span, debug, "EXTENSION")),
        "ABSPATH" => Some(eval_builtin_abspath(args, env, span, debug)),
        "REALPATH" => Some(eval_builtin_realpath(args, env, span, debug)),
        "ISFILE" => Some(eval_builtin_isfile(args, env, span, debug)),
        "ISDIR" => Some(eval_builtin_isdir(args, env, span, debug)),
        "TEMPDIR" => Some(eval_builtin_tempdir(args, env, span)),
        "HOMEDIR" => Some(eval_builtin_user_dir(args, env, span, "HOMEDIR", "home")),
        "CONFIGDIR" => Some(eval_builtin_user_dir(
            args,
            env,
            span,
            "CONFIGDIR",
            "config",
        )),
        "CACHEDIR" => Some(eval_builtin_user_dir(args, env, span, "CACHEDIR", "cache")),
        "DATADIR" => Some(eval_builtin_user_dir(args, env, span, "DATADIR", "data")),

        "PLATFORM" => Some(eval_machine_string(args, env, span, "PLATFORM")),
        "ARCH" => Some(eval_machine_string(args, env, span, "ARCH")),
        "OSFAMILY" => Some(eval_machine_string(args, env, span, "OSFAMILY")),
        "OSNAME" => Some(eval_machine_string(args, env, span, "OSNAME")),
        "OSVERSION" => Some(eval_machine_string(args, env, span, "OSVERSION")),
        "KERNELVERSION" => Some(eval_machine_string(args, env, span, "KERNELVERSION")),
        "HOSTNAME" => Some(eval_machine_string(args, env, span, "HOSTNAME")),
        "USERNAME" => Some(eval_machine_string(args, env, span, "USERNAME")),
        "VERSION" => Some(eval_machine_string(args, env, span, "VERSION")),
        "CPUCOUNT" => Some(eval_machine_number(args, env, span, "CPUCOUNT")),
        "PHYSICALCPUS" => Some(eval_machine_number(args, env, span, "PHYSICALCPUS")),
        "TOTALMEMORY" => Some(eval_machine_number(args, env, span, "TOTALMEMORY")),
        "USEDMEMORY" => Some(eval_machine_number(args, env, span, "USEDMEMORY")),
        "UPTIME" => Some(eval_machine_number(args, env, span, "UPTIME")),
        "SYSINFO" => Some(eval_builtin_sysinfo(args, env, span)),

        "TYPEOF" => Some(eval_builtin_typeof(args, env, span, debug)),
        "EXECUTE" => Some(eval_builtin_execute(args, env, span, debug)),
        "ISDEFINED" => Some(eval_builtin_isdefined(args, env, span, debug)),
        "GETVAR" => Some(eval_builtin_getvar(args, env, span, debug)),
        "SETVAR" => Some(eval_builtin_setvar(args, env, span, debug)),
        "UNSETVAR" => Some(eval_builtin_unsetvar(args, env, span, debug)),
        "VARIABLES" => Some(eval_builtin_variables(args, env, span)),
        "PROCEDURES" => Some(eval_builtin_procedures(args, env, span)),
        "CALL" => Some(eval_builtin_call(args, env, span, debug)),

        "SCRIPTPATH" => Some(eval_builtin_scriptpath(args, env, span)),
        "ISMAIN" => Some(eval_builtin_ismain(args, env, span)),
        "MODULES" => Some(eval_builtin_modules(args, env, span)),
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
    // Drain whatever is pending before we stall the program, so a progress
    // message printed just before a SLEEP is on screen during the sleep.
    env.borrow().sink().borrow_mut().flush();
    let _ = io::stdout().flush();
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
    let haystack = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let needle = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match (haystack, needle) {
        (Value::String(s), Value::String(t)) => Ok(Value::Boolean(s.contains(&t))),
        // Membership, not substring, once the first argument is a container.
        // Several built-ins now hand back lists (LISTDIR, PROCEDURES, VARIABLES,
        // MODULES, KEYS), and asking whether one holds a given element should not
        // require writing a FOR EACH loop.
        (Value::List(items), needle) => Ok(Value::Boolean(
            items.iter().any(|item| values_equal(item, &needle)),
        )),
        // For a dictionary this asks about keys, matching the way `in` reads for
        // a mapping and agreeing with HASKEY.
        (Value::Dictionary(dict), needle) => match value_to_key(&needle) {
            Ok(key) => Ok(Value::Boolean(dict.contains_key(&key))),
            // An illegal key cannot be present, and answering FALSE is more
            // useful here than refusing the question.
            Err(_) => Ok(Value::Boolean(false)),
        },
        _ => Err(runtime_err(
            "CONTAINS requires a string, list or dictionary as its first argument",
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
        // The result is a character position so it can be fed straight back
        // into `s[i]` or `SUBSTRING`, which are character based too.
        (Value::String(s), Value::String(t)) => match s.find(&t) {
            Some(byte_idx) => Ok(Value::Integer(BigInt::from(
                str_char_len(&s[..byte_idx]) + 1,
            ))),
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

fn eval_builtin_dictionary(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> EvalResult {
    if !args.is_empty() {
        return Err(runtime_err("DICTIONARY takes no arguments", span, env));
    }
    Ok(Value::Dictionary(Dict::default()))
}

fn eval_builtin_keys(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("KEYS requires one argument", span, env));
    }
    with_value(&args[0], env, debug, |value| match value {
        Value::Dictionary(entries) => Some(entries.keys().map(key_to_value).collect()),
        _ => None,
    })?
    .map(Value::List)
    .ok_or_else(|| runtime_err("KEYS requires a dictionary argument", span, env))
}

fn eval_builtin_values(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 1 {
        return Err(runtime_err("VALUES requires one argument", span, env));
    }
    with_value(&args[0], env, debug, |value| match value {
        Value::Dictionary(entries) => Some(entries.values().cloned().collect()),
        _ => None,
    })?
    .map(Value::List)
    .ok_or_else(|| runtime_err("VALUES requires a dictionary argument", span, env))
}

fn eval_builtin_haskey(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("HASKEY requires two arguments", span, env));
    }
    let dict = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let key_val = evaluate_node(&args[1], Rc::clone(env), debug)?;
    match dict {
        Value::Dictionary(entries) => match value_to_key(&key_val) {
            // A value that could never be a key simply is not present, so the
            // guard form `IF HASKEY(d, k)` stays usable for any k.
            Err(_) => Ok(Value::Boolean(false)),
            Ok(key) => Ok(Value::Boolean(entries.contains_key(&key))),
        },
        _ => Err(runtime_err(
            "HASKEY requires a dictionary argument",
            span,
            env,
        )),
    }
}

fn eval_builtin_getkey(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() < 2 || args.len() > 3 {
        return Err(runtime_err("GETKEY requires 2 or 3 arguments", span, env));
    }
    let dict = evaluate_node(&args[0], Rc::clone(env), debug)?;
    let key_val = evaluate_node(&args[1], Rc::clone(env), debug)?;
    let Value::Dictionary(entries) = dict else {
        return Err(runtime_err(
            "GETKEY requires a dictionary argument",
            span,
            env,
        ));
    };
    // A value that could never be a key is simply absent: with a default that
    // means the default, and without one the usual "Key not found" error.
    let key = match value_to_key(&key_val) {
        Ok(key) => key,
        Err(_) if args.len() == 3 => return evaluate_node(&args[2], Rc::clone(env), debug),
        Err(msg) => return Err(runtime_err(msg, span, env)),
    };
    match entries.get(&key) {
        Some(value) => Ok(value.clone()),
        None if args.len() == 3 => evaluate_node(&args[2], Rc::clone(env), debug),
        None => Err(runtime_err(
            format!("Key not found: {}", key_to_string(&key)),
            span,
            env,
        )),
    }
}

fn eval_builtin_setkey(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 3 {
        return Err(runtime_err("SETKEY requires three arguments", span, env));
    }
    let AstNode::Identifier(name) = &args[0].node else {
        return Err(runtime_err(
            "SETKEY requires a dictionary variable",
            span,
            env,
        ));
    };
    let key_val = evaluate_node(&args[1], Rc::clone(env), debug)?;
    let new_val = evaluate_node(&args[2], Rc::clone(env), debug)?;
    if !env
        .borrow()
        .with_var(name, |value| matches!(value, Value::Dictionary(_)))
        .unwrap_or(false)
    {
        return Err(runtime_err(
            format!("Variable {} is not a dictionary", name),
            span,
            env,
        ));
    }
    let key = value_to_key(&key_val).map_err(|msg| runtime_err(msg, span, env))?;
    let outcome = env.borrow_mut().with_var_mut(name, |value| {
        let Value::Dictionary(entries) = value else {
            return Err(());
        };
        entries.insert(key, new_val.clone());
        Ok(())
    });
    match outcome {
        Some(Ok(())) => Ok(new_val),
        _ => Err(runtime_err(
            format!("Variable {} is not a dictionary", name),
            span,
            env,
        )),
    }
}

fn eval_builtin_removekey(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() != 2 {
        return Err(runtime_err("REMOVEKEY requires two arguments", span, env));
    }
    let AstNode::Identifier(name) = &args[0].node else {
        return Err(runtime_err(
            "REMOVEKEY requires a dictionary variable",
            span,
            env,
        ));
    };
    let key_val = evaluate_node(&args[1], Rc::clone(env), debug)?;
    if !env
        .borrow()
        .with_var(name, |value| matches!(value, Value::Dictionary(_)))
        .unwrap_or(false)
    {
        return Err(runtime_err(
            format!("Variable {} is not a dictionary", name),
            span,
            env,
        ));
    }
    dict_remove_entry(name, &key_val, env, span)
}

/// Reject a file builtin on targets with no filesystem to reach.
///
/// `std::fs` compiles for `wasm32-unknown-unknown` but every call there fails
/// with an opaque "operation not supported" from the browser sandbox, so the
/// guard reports the real reason instead. Native targets and WASI both have a
/// real filesystem, so there it is a no-op. The error is an ordinary runtime
/// error, so a program that wants to degrade gracefully can wrap the call in
/// TRY/CATCH.
#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
fn fs_guard(name: &str, span: Span, env: &Rc<RefCell<Environment>>) -> Result<(), Interruption> {
    Err(runtime_err(
        format!(
            "{} is not supported in WebAssembly: the browser sandbox has no filesystem",
            name
        ),
        span,
        env,
    ))
}

#[cfg(any(not(target_arch = "wasm32"), feature = "wasi"))]
fn fs_guard(_name: &str, _span: Span, _env: &Rc<RefCell<Environment>>) -> Result<(), Interruption> {
    Ok(())
}

/// Evaluate the single argument every file builtin starts with: a path, which
/// must be a string. Paths are taken verbatim, so a relative one resolves
/// against the process's working directory rather than the script's location.
fn eval_path_arg(
    name: &str,
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> Result<String, Interruption> {
    match evaluate_node(&args[0], Rc::clone(env), debug)? {
        Value::String(path) => Ok(path),
        _ => Err(runtime_err(
            format!("{} requires a string path", name),
            span,
            env,
        )),
    }
}

/// Surface an `io::Error` as a PSL runtime error naming both the builtin and the
/// path, since "No such file or directory" alone says nothing about which file.
fn fs_err(
    name: &str,
    path: &str,
    error: &io::Error,
    span: Span,
    env: &Rc<RefCell<Environment>>,
) -> Interruption {
    runtime_err(
        format!("{} failed for '{}': {}", name, path, error),
        span,
        env,
    )
}

/// Shared prologue for the file builtins that take exactly one path argument.
fn eval_one_path_builtin(
    name: &str,
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> Result<String, Interruption> {
    if args.len() != 1 {
        return Err(runtime_err(
            format!("{} requires one argument", name),
            span,
            env,
        ));
    }
    fs_guard(name, span, env)?;
    eval_path_arg(name, args, env, span, debug)
}

/// Shared prologue for the file builtins that take a path and the text to write.
fn eval_path_and_text(
    name: &str,
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> Result<(String, String), Interruption> {
    if args.len() != 2 {
        return Err(runtime_err(
            format!("{} requires two arguments", name),
            span,
            env,
        ));
    }
    fs_guard(name, span, env)?;
    let path = eval_path_arg(name, args, env, span, debug)?;
    match evaluate_node(&args[1], Rc::clone(env), debug)? {
        Value::String(text) => Ok((path, text)),
        _ => Err(runtime_err(
            format!(
                "{} requires a string as its second argument -- use TOSTRING to write a non-string value",
                name
            ),
            span,
            env,
        )),
    }
}

fn eval_builtin_readfile(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let path = eval_one_path_builtin("READFILE", args, env, span, debug)?;
    match std::fs::read_to_string(&path) {
        Ok(contents) => Ok(Value::String(contents)),
        Err(e) => Err(fs_err("READFILE", &path, &e, span, env)),
    }
}

fn eval_builtin_readlines(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let path = eval_one_path_builtin("READLINES", args, env, span, debug)?;
    match std::fs::read_to_string(&path) {
        // `lines` strips the terminator, treats "\r\n" and "\n" alike, and does
        // not invent a trailing empty line for a file that ends in a newline --
        // which is what a program iterating the result expects.
        Ok(contents) => Ok(Value::List(
            contents
                .lines()
                .map(|line| Value::String(line.to_string()))
                .collect(),
        )),
        Err(e) => Err(fs_err("READLINES", &path, &e, span, env)),
    }
}

fn eval_builtin_writefile(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let (path, text) = eval_path_and_text("WRITEFILE", args, env, span, debug)?;
    match std::fs::write(&path, text.as_bytes()) {
        Ok(()) => Ok(Value::Unit),
        Err(e) => Err(fs_err("WRITEFILE", &path, &e, span, env)),
    }
}

fn eval_builtin_appendfile(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let (path, text) = eval_path_and_text("APPENDFILE", args, env, span, debug)?;
    let opened = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path);
    match opened.and_then(|mut file| file.write_all(text.as_bytes())) {
        Ok(()) => Ok(Value::Unit),
        Err(e) => Err(fs_err("APPENDFILE", &path, &e, span, env)),
    }
}

fn eval_builtin_fileexists(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let path = eval_one_path_builtin("FILEEXISTS", args, env, span, debug)?;
    // A path that cannot be inspected (no permission on a parent directory, say)
    // is reported as absent rather than as an error: the question asked is only
    // whether this program can see something there.
    Ok(Value::Boolean(std::path::Path::new(&path).exists()))
}

fn eval_builtin_filesize(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let path = eval_one_path_builtin("FILESIZE", args, env, span, debug)?;
    match std::fs::metadata(&path) {
        // Bytes, not characters: LENGTH(READFILE(p)) is the character count, and
        // for non-ASCII text the two legitimately differ.
        Ok(metadata) => Ok(Value::Integer(BigInt::from(metadata.len()))),
        Err(e) => Err(fs_err("FILESIZE", &path, &e, span, env)),
    }
}

fn eval_builtin_deletefile(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let path = eval_one_path_builtin("DELETEFILE", args, env, span, debug)?;
    // Files only, and said so plainly. The OS is no help here: `remove_file` on a
    // directory reports EPERM on macOS and EISDIR on Linux, neither of which tells
    // the reader that the *kind* of path was the problem.
    if system::is_dir(&path) && !system::is_symlink(&path) {
        return Err(runtime_err(
            format!(
                "DELETEFILE will not remove a directory ('{}'). Use DELETEDIR for an empty one, or DELETETREE to remove it and everything inside.",
                path
            ),
            span,
            env,
        ));
    }
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(Value::Unit),
        Err(e) => Err(fs_err("DELETEFILE", &path, &e, span, env)),
    }
}

/// Remove an empty directory -- the safe default, and `os.rmdir`.
///
/// This is the counterpart `MAKEDIR` had been missing: without it a program that
/// created a scratch tree had no way to clean it up, and the only escape hatch was
/// a non-portable `SHELL("rm -rf ...")`.
fn eval_builtin_deletedir(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let path = eval_one_path_builtin("DELETEDIR", args, env, span, debug)?;
    // Empty directories only, which is what makes this safe by construction: a
    // directory holding anything refuses to go, so a mistyped path cannot destroy
    // work.
    match std::fs::remove_dir(&path) {
        Ok(()) => Ok(Value::Unit),
        Err(e) => {
            let hint = if system::is_dir(&path) {
                ". Use DELETETREE to remove a directory that still has contents."
            } else {
                ""
            };
            Err(runtime_err(
                format!("DELETEDIR failed for '{}': {}{}", path, e, hint),
                span,
                env,
            ))
        }
    }
}

/// Remove a directory and everything inside it -- `shutil.rmtree`.
///
/// A separate built-in rather than a flag on [`eval_builtin_deletedir`], so that
/// the destructive one is spelled out at the call site and cannot be reached by
/// accident.
fn eval_builtin_deletetree(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let path = eval_one_path_builtin("DELETETREE", args, env, span, debug)?;
    // Refuses a plain file, so a path that turned out to name something narrower
    // than expected is reported rather than quietly acted on.
    if system::is_file(&path) || system::is_symlink(&path) {
        return Err(runtime_err(
            format!(
                "DELETETREE removes directories, and '{}' is a file or a link. Use DELETEFILE.",
                path
            ),
            span,
            env,
        ));
    }
    match std::fs::remove_dir_all(&path) {
        Ok(()) => Ok(Value::Unit),
        Err(e) => Err(fs_err("DELETETREE", &path, &e, span, env)),
    }
}

/// When a file was last modified, as Unix seconds.
///
/// Seconds rather than a formatted string so the result feeds straight into TIME
/// and TIMEZONE, and so two files can simply be compared: `IF FILEMTIME(src) >
/// FILEMTIME(out)`.
fn eval_builtin_filemtime(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let path = eval_one_path_builtin("FILEMTIME", args, env, span, debug)?;
    let metadata =
        std::fs::metadata(&path).map_err(|e| fs_err("FILEMTIME", &path, &e, span, env))?;
    let modified = metadata
        .modified()
        .map_err(|e| fs_err("FILEMTIME", &path, &e, span, env))?;
    // A file stamped before 1970 -- rare, but possible on a restored archive --
    // yields a negative number rather than an error.
    let seconds = match modified.duration_since(std::time::UNIX_EPOCH) {
        Ok(since) => BigInt::from(since.as_secs()),
        Err(before) => -BigInt::from(before.duration().as_secs()),
    };
    Ok(Value::Integer(seconds))
}

fn eval_builtin_listdir(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let path = eval_one_path_builtin("LISTDIR", args, env, span, debug)?;
    let entries = std::fs::read_dir(&path).map_err(|e| fs_err("LISTDIR", &path, &e, span, env))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| fs_err("LISTDIR", &path, &e, span, env))?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    // `read_dir` yields entries in whatever order the filesystem stores them, so
    // sorting is what makes a program that lists a directory reproducible.
    names.sort();
    Ok(Value::List(names.into_iter().map(Value::String).collect()))
}

fn eval_builtin_makedir(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let path = eval_one_path_builtin("MAKEDIR", args, env, span, debug)?;
    // Recursive, and succeeds when the directory is already there: MAKEDIR states
    // the directory should exist rather than that it should be created now.
    match std::fs::create_dir_all(&path) {
        Ok(()) => Ok(Value::Unit),
        Err(e) => Err(fs_err("MAKEDIR", &path, &e, span, env)),
    }
}

// ---------------------------------------------------------------------------
// Shared argument handling for the host-facing builtins
// ---------------------------------------------------------------------------

fn expect_no_args(
    name: &str,
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> Result<(), Interruption> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(runtime_err(
            format!("{} takes no arguments", name),
            span,
            env,
        ))
    }
}

fn expect_arity(
    name: &str,
    args: &[Spanned],
    wanted: usize,
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> Result<(), Interruption> {
    if args.len() == wanted {
        return Ok(());
    }
    let plural = if wanted == 1 { "argument" } else { "arguments" };
    Err(runtime_err(
        format!("{} requires {} {}", name, wanted, plural),
        span,
        env,
    ))
}

/// Evaluate one argument that has to be a string, naming the position in the
/// error so a three-argument builtin says which one was wrong.
fn eval_string_arg(
    name: &str,
    arg: &Spanned,
    what: &str,
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> Result<String, Interruption> {
    match evaluate_node(arg, Rc::clone(env), debug)? {
        Value::String(s) => Ok(s),
        _ => Err(runtime_err(
            format!("{} requires a string {}", name, what),
            span,
            env,
        )),
    }
}

/// Evaluate one argument that has to be a non-negative integer small enough to
/// be a process id.
fn eval_pid_arg(
    name: &str,
    arg: &Spanned,
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> Result<u32, Interruption> {
    match evaluate_node(arg, Rc::clone(env), debug)? {
        Value::Integer(n) => n.to_u32().ok_or_else(|| {
            runtime_err(
                format!("{} requires a process id that fits in 32 bits", name),
                span,
                env,
            )
        }),
        _ => Err(runtime_err(
            format!("{} requires an integer process id", name),
            span,
            env,
        )),
    }
}

/// Build a dictionary with string keys, preserving the order given.
fn dict_of(pairs: Vec<(&str, Value)>) -> Value {
    let mut dict = Dict::default();
    for (key, value) in pairs {
        dict.insert(DictKey::String(key.to_string()), value);
    }
    Value::Dictionary(dict)
}

/// A fact the platform may genuinely not know: `NULL` rather than an empty
/// string, so "unknown" and "known to be empty" stay distinguishable.
fn optional_string(value: Option<String>) -> Value {
    match value {
        Some(s) => Value::String(s),
        None => Value::Null,
    }
}

fn u64_value(n: u64) -> Value {
    Value::Integer(BigInt::from(n))
}

/// Push everything the program has printed all the way out to the terminal.
///
/// The sink's own flush moves buffered text into stdout; stdout is itself
/// buffered, so it needs flushing too before the interpreter blocks on something
/// slow. Used by the built-ins that wait on the outside world.
fn flush_all(env: &Rc<RefCell<Environment>>) {
    env.borrow().sink().borrow_mut().flush();
    let _ = io::stdout().flush();
}

/// Turn a `Result<_, String>` from [`crate::system`] into a PSL runtime error.
fn sys_err<T>(
    result: Result<T, String>,
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> Result<T, Interruption> {
    result.map_err(|msg| runtime_err(msg, span, env))
}

// ---------------------------------------------------------------------------
// Filesystem operations beyond plain reading and writing
// ---------------------------------------------------------------------------

fn eval_builtin_rename(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("RENAME", args, 2, env, span)?;
    fs_guard("RENAME", span, env)?;
    let from = eval_string_arg("RENAME", &args[0], "source path", env, span, debug)?;
    let to = eval_string_arg("RENAME", &args[1], "destination path", env, span, debug)?;
    // Also the move operation: `fs::rename` relocates within a filesystem and
    // replaces an existing destination, which is what `os.rename` does too.
    match std::fs::rename(&from, &to) {
        Ok(()) => Ok(Value::Unit),
        Err(e) => Err(runtime_err(
            format!("RENAME failed for '{}' -> '{}': {}", from, to, e),
            span,
            env,
        )),
    }
}

fn eval_builtin_copyfile(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("COPYFILE", args, 2, env, span)?;
    fs_guard("COPYFILE", span, env)?;
    let from = eval_string_arg("COPYFILE", &args[0], "source path", env, span, debug)?;
    let to = eval_string_arg("COPYFILE", &args[1], "destination path", env, span, debug)?;
    // `fs::copy` truncates the destination before reading the source, so copying a
    // file onto itself destroyed it and still reported success.
    if system::is_same_file(&from, &to) {
        return Err(runtime_err(
            format!(
                "COPYFILE would copy '{}' onto itself, which would destroy it",
                from
            ),
            span,
            env,
        ));
    }
    match std::fs::copy(&from, &to) {
        // The byte count is the one genuinely useful thing a copy can report,
        // and it is what `fs::copy` already returns.
        Ok(bytes) => Ok(u64_value(bytes)),
        Err(e) => Err(runtime_err(
            format!("COPYFILE failed for '{}' -> '{}': {}", from, to, e),
            span,
            env,
        )),
    }
}

// ---------------------------------------------------------------------------
// Environment variables
// ---------------------------------------------------------------------------

fn eval_builtin_getenv(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.is_empty() || args.len() > 2 {
        return Err(runtime_err(
            "GETENV requires one or two arguments",
            span,
            env,
        ));
    }
    let name = eval_string_arg("GETENV", &args[0], "variable name", env, span, debug)?;
    match system::env_var(&name) {
        Some(value) => Ok(Value::String(value)),
        // With a default supplied, a missing variable is the expected case; with
        // none, it is a mistake worth reporting -- the same split as GETARG.
        None if args.len() == 2 => evaluate_node(&args[1], Rc::clone(env), debug),
        None => Err(runtime_err(
            format!(
                "Environment variable '{}' is not set. Pass a second argument to GETENV to supply a default.",
                name
            ),
            span,
            env,
        )),
    }
}

fn eval_builtin_setenv(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("SETENV", args, 2, env, span)?;
    let name = eval_string_arg("SETENV", &args[0], "variable name", env, span, debug)?;
    let value = eval_string_arg("SETENV", &args[1], "value", env, span, debug)?;
    sys_err(system::set_env_var(&name, &value), env, span)?;
    Ok(Value::Unit)
}

fn eval_builtin_unsetenv(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("UNSETENV", args, 1, env, span)?;
    let name = eval_string_arg("UNSETENV", &args[0], "variable name", env, span, debug)?;
    sys_err(system::unset_env_var(&name), env, span)?;
    Ok(Value::Unit)
}

fn eval_builtin_envvars(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> EvalResult {
    expect_no_args("ENVVARS", args, env, span)?;
    let mut dict = Dict::default();
    for (name, value) in system::env_vars() {
        dict.insert(DictKey::String(name), Value::String(value));
    }
    Ok(Value::Dictionary(dict))
}

// ---------------------------------------------------------------------------
// Running other programs
// ---------------------------------------------------------------------------

/// Shape every command result the same way, so EXEC and SHELL are
/// interchangeable at the call site.
///
/// `exitcode` is `NULL` when the child was killed by a signal rather than
/// exiting on its own, which is the one case where "the exit status" does not
/// exist. Callers that only care whether it worked can compare against 0.
fn command_output_value(output: system::CommandOutput) -> Value {
    dict_of(vec![
        (
            "exitcode",
            match output.exit_code {
                Some(code) => Value::Integer(BigInt::from(code)),
                None => Value::Null,
            },
        ),
        ("stdout", Value::String(output.stdout)),
        ("stderr", Value::String(output.stderr)),
    ])
}

fn eval_builtin_exec(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.is_empty() || args.len() > 2 {
        return Err(runtime_err("EXEC requires one or two arguments", span, env));
    }
    let program = eval_string_arg("EXEC", &args[0], "program name", env, span, debug)?;
    let mut argv: Vec<String> = Vec::new();
    if args.len() == 2 {
        match evaluate_node(&args[1], Rc::clone(env), debug)? {
            Value::List(items) => {
                for item in items {
                    match item {
                        Value::String(s) => argv.push(s),
                        other => {
                            return Err(runtime_err(
                                format!(
                                    "EXEC arguments must all be strings, found {}. Use TOSTRING to pass a number.",
                                    type_name(&other)
                                ),
                                span,
                                env,
                            ));
                        }
                    }
                }
            }
            _ => {
                return Err(runtime_err(
                    "EXEC requires a list of strings as its second argument",
                    span,
                    env,
                ));
            }
        }
    }
    // Waiting on a child can take arbitrarily long, so anything this program has
    // already displayed is pushed all the way out first -- the same reason SLEEP
    // flushes. The child's own output is captured into the returned dictionary
    // rather than written to the terminal, so the two cannot interleave.
    flush_all(env);
    let output = sys_err(system::exec(&program, &argv), env, span)?;
    Ok(command_output_value(output))
}

fn eval_builtin_shell(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("SHELL", args, 1, env, span)?;
    let command = eval_string_arg("SHELL", &args[0], "command line", env, span, debug)?;
    flush_all(env);
    let output = sys_err(system::shell(&command), env, span)?;
    Ok(command_output_value(output))
}

fn eval_builtin_which(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("WHICH", args, 1, env, span)?;
    let program = eval_string_arg("WHICH", &args[0], "program name", env, span, debug)?;
    // NULL rather than an error: "is this tool installed?" is a question, and the
    // answer "no" is not a failure.
    Ok(optional_string(system::which(&program)))
}

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

fn eval_builtin_exit(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.len() > 1 {
        return Err(runtime_err(
            "EXIT takes no arguments, or one exit code",
            span,
            env,
        ));
    }
    let code = match args.first() {
        None => 0,
        Some(arg) => match evaluate_node(arg, Rc::clone(env), debug)? {
            // Exit statuses are truncated to 8 bits by every one of the three
            // platforms, so a code outside 0-255 is rejected rather than
            // silently becoming a different number.
            Value::Integer(n) => n
                .to_i32()
                .filter(|c| (0..=255).contains(c))
                .ok_or_else(|| {
                    runtime_err("EXIT requires an exit code between 0 and 255", span, env)
                })?,
            _ => return Err(runtime_err("EXIT requires an integer exit code", span, env)),
        },
    };
    Err(Interruption::Exit(code))
}

fn eval_builtin_pid(args: &[Spanned], env: &Rc<RefCell<Environment>>, span: Span) -> EvalResult {
    expect_no_args("PID", args, env, span)?;
    Ok(Value::Integer(BigInt::from(system::current_pid())))
}

fn process_info_value(info: &system::ProcessInfo) -> Value {
    dict_of(vec![
        ("pid", Value::Integer(BigInt::from(info.pid))),
        ("name", Value::String(info.name.clone())),
        ("memory", u64_value(info.memory_bytes)),
        (
            "parent",
            match info.parent_pid {
                Some(pid) => Value::Integer(BigInt::from(pid)),
                None => Value::Null,
            },
        ),
    ])
}

fn eval_builtin_processinfo(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("PROCESSINFO", args, 1, env, span)?;
    let pid = eval_pid_arg("PROCESSINFO", &args[0], env, span, debug)?;
    // NULL for "nothing is running under that pid", which a program checking on
    // a child it started needs to be able to ask without catching an error.
    match system::process_info(pid) {
        Some(info) => Ok(process_info_value(&info)),
        None => Ok(Value::Null),
    }
}

fn eval_builtin_processes(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> EvalResult {
    expect_no_args("PROCESSES", args, env, span)?;
    Ok(Value::List(
        system::processes().iter().map(process_info_value).collect(),
    ))
}

fn eval_builtin_kill(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("KILL", args, 1, env, span)?;
    let pid = eval_pid_arg("KILL", &args[0], env, span, debug)?;
    if pid == system::current_pid() {
        return Err(runtime_err(
            "KILL refuses to terminate the interpreter itself; use EXIT instead",
            span,
            env,
        ));
    }
    // `FALSE` means the request was understood but refused -- typically a
    // process owned by another user. A pid that does not exist at all is an
    // error, because that is a bug in the program rather than a permission fact.
    Ok(Value::Boolean(sys_err(system::kill(pid), env, span)?))
}

// ---------------------------------------------------------------------------
// Working directory and paths
// ---------------------------------------------------------------------------

fn eval_builtin_cwd(args: &[Spanned], env: &Rc<RefCell<Environment>>, span: Span) -> EvalResult {
    expect_no_args("CWD", args, env, span)?;
    Ok(Value::String(sys_err(system::cwd(), env, span)?))
}

fn eval_builtin_chdir(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("CHDIR", args, 1, env, span)?;
    let path = eval_string_arg("CHDIR", &args[0], "path", env, span, debug)?;
    sys_err(system::chdir(&path), env, span)?;
    Ok(Value::Unit)
}

fn eval_builtin_joinpath(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.is_empty() {
        return Err(runtime_err(
            "JOINPATH requires at least one argument",
            span,
            env,
        ));
    }
    let mut segments = Vec::with_capacity(args.len());
    for arg in args {
        segments.push(eval_string_arg(
            "JOINPATH",
            arg,
            "path segment",
            env,
            span,
            debug,
        )?);
    }
    Ok(Value::String(system::join_paths(&segments)))
}

/// The three pure-syntax path queries, which differ only in which part they
/// pick out and never touch the filesystem.
fn eval_builtin_path_part(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
    name: &str,
) -> EvalResult {
    expect_arity(name, args, 1, env, span)?;
    let path = eval_string_arg(name, &args[0], "path", env, span, debug)?;
    let part = match name {
        "BASENAME" => system::basename(&path),
        "DIRNAME" => system::dirname(&path),
        _ => system::extension(&path),
    };
    Ok(Value::String(part))
}

fn eval_builtin_abspath(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("ABSPATH", args, 1, env, span)?;
    let path = eval_string_arg("ABSPATH", &args[0], "path", env, span, debug)?;
    Ok(Value::String(sys_err(system::abspath(&path), env, span)?))
}

fn eval_builtin_realpath(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("REALPATH", args, 1, env, span)?;
    fs_guard("REALPATH", span, env)?;
    let path = eval_string_arg("REALPATH", &args[0], "path", env, span, debug)?;
    Ok(Value::String(sys_err(system::realpath(&path), env, span)?))
}

fn eval_builtin_isfile(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let path = eval_one_path_builtin("ISFILE", args, env, span, debug)?;
    Ok(Value::Boolean(system::is_file(&path)))
}

fn eval_builtin_isdir(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    let path = eval_one_path_builtin("ISDIR", args, env, span, debug)?;
    Ok(Value::Boolean(system::is_dir(&path)))
}

fn eval_builtin_tempdir(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> EvalResult {
    expect_no_args("TEMPDIR", args, env, span)?;
    Ok(Value::String(system::temp_dir()))
}

fn eval_builtin_user_dir(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    name: &str,
    kind: &str,
) -> EvalResult {
    expect_no_args(name, args, env, span)?;
    Ok(Value::String(sys_err(system::user_dir(kind), env, span)?))
}

// ---------------------------------------------------------------------------
// Facts about the machine
// ---------------------------------------------------------------------------

fn eval_machine_string(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    name: &str,
) -> EvalResult {
    expect_no_args(name, args, env, span)?;
    // The three compile-time constants and the interpreter version cost nothing
    // to read, so they skip the probe that the rest need.
    match name {
        "PLATFORM" => return Ok(Value::String(system::platform().to_string())),
        "ARCH" => return Ok(Value::String(system::arch().to_string())),
        "OSFAMILY" => return Ok(Value::String(system::family().to_string())),
        "VERSION" => {
            return Ok(Value::String(system::interpreter_version().to_string()));
        }
        "USERNAME" => return Ok(optional_string(system::username())),
        _ => {}
    }
    let info = system::machine_info();
    Ok(optional_string(match name {
        "OSNAME" => info.os_name,
        "OSVERSION" => info.os_version,
        "KERNELVERSION" => info.kernel_version,
        _ => info.hostname,
    }))
}

fn eval_machine_number(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    name: &str,
) -> EvalResult {
    expect_no_args(name, args, env, span)?;
    // Only the two memory figures need the probe that reads memory; the rest are
    // cheap lookups, and building a whole `System` for them was wasted work.
    match name {
        "CPUCOUNT" => return Ok(Value::Integer(BigInt::from(system::logical_cpus()))),
        "PHYSICALCPUS" => {
            return Ok(match system::physical_cpus() {
                Some(n) => Value::Integer(BigInt::from(n)),
                None => Value::Null,
            });
        }
        "UPTIME" => return Ok(u64_value(system::uptime_seconds())),
        _ => {}
    }
    let info = system::machine_info();
    Ok(match name {
        "TOTALMEMORY" => u64_value(info.total_memory),
        _ => u64_value(info.used_memory),
    })
}

fn eval_builtin_sysinfo(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> EvalResult {
    expect_no_args("SYSINFO", args, env, span)?;
    let info = system::machine_info();
    Ok(dict_of(vec![
        ("platform", Value::String(info.platform.to_string())),
        ("arch", Value::String(info.arch.to_string())),
        ("osfamily", Value::String(info.family.to_string())),
        ("osname", optional_string(info.os_name)),
        ("osversion", optional_string(info.os_version)),
        ("kernelversion", optional_string(info.kernel_version)),
        ("hostname", optional_string(info.hostname)),
        ("username", optional_string(info.username)),
        ("cpucount", Value::Integer(BigInt::from(info.logical_cpus))),
        (
            "physicalcpus",
            match info.physical_cpus {
                Some(n) => Value::Integer(BigInt::from(n)),
                None => Value::Null,
            },
        ),
        ("totalmemory", u64_value(info.total_memory)),
        ("usedmemory", u64_value(info.used_memory)),
        ("uptime", u64_value(info.uptime_seconds)),
        (
            "version",
            Value::String(system::interpreter_version().to_string()),
        ),
    ]))
}

// ---------------------------------------------------------------------------
// Meta programming
// ---------------------------------------------------------------------------

/// The name PseudoLang uses for a value's type, as reported by TYPEOF.
fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Boolean(_) => "boolean",
        Value::List(_) => "list",
        Value::Dictionary(_) => "dictionary",
        Value::Null => "null",
        Value::NaN => "nan",
        Value::Unit => "unit",
    }
}

fn eval_builtin_typeof(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("TYPEOF", args, 1, env, span)?;
    let value = evaluate_node(&args[0], Rc::clone(env), debug)?;
    Ok(Value::String(type_name(&value).to_string()))
}

/// Run `body` as a nested evaluation of source the program produced itself.
///
/// ```text
/// code <- "EXECUTE(code)"
/// EXECUTE(code)
/// ```
///
/// Nothing in that loop is a procedure call, so the recursion bypasses
/// [`MAX_STACK_DEPTH`] and overflows the real stack. [`MAX_META_DEPTH`] bounds it.
fn with_meta_frame(
    name: &str,
    source: &str,
    env: &Rc<RefCell<Environment>>,
    span: Span,
    body: impl FnOnce() -> EvalResult,
) -> EvalResult {
    let meta_depth = Rc::clone(&env.borrow().meta_depth);
    if meta_depth.get() >= MAX_META_DEPTH {
        return Err(runtime_err(
            format!(
                "Maximum {} nesting depth exceeded (limit: {}). Source evaluated by EVAL or EXECUTE may not go on evaluating itself.",
                name, MAX_META_DEPTH
            ),
            span,
            env,
        ));
    }
    meta_depth.set(meta_depth.get() + 1);
    // A frame as well as the counter: the counter is what stops the recursion,
    // and the frame is what makes the resulting error's stack trace show where
    // the nesting came from.
    env.borrow().push_frame(StackFrame {
        name: name.to_string(),
        span,
    });
    // The generated text has a span space of its own, so an error raised inside it
    // has to carry it. Without this the offsets were resolved against the entry
    // script and the caret landed on whatever text sat at that offset.
    let modules = Rc::clone(&env.borrow().modules);
    modules
        .borrow_mut()
        .generated
        .push((Rc::from(source), format!("in {}", name)));
    let result = body();
    modules.borrow_mut().generated.pop();
    env.borrow().pop_frame();
    meta_depth.set(meta_depth.get() - 1);
    result
}

fn eval_builtin_execute(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("EXECUTE", args, 1, env, span)?;
    let source = eval_string_arg("EXECUTE", &args[0], "source argument", env, span, debug)?;
    with_meta_frame("EXECUTE", &source, env, span, || {
        let mut lexer = crate::lexer::Lexer::new(&source);
        let tokens = lexer.tokenize();
        // A whole program, not a single expression: this is EVAL's counterpart
        // for statements, so assignments and procedure declarations inside the
        // string land in the calling scope.
        let ast = crate::parser::parse(tokens, debug).map_err(|e| {
            runtime_err(
                format!("EXECUTE could not parse its source: {}", e.format(&source)),
                span,
                env,
            )
        })?;
        evaluate_node(&ast, Rc::clone(env), debug)?;
        Ok(Value::Unit)
    })
}

fn eval_builtin_isdefined(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("ISDEFINED", args, 1, env, span)?;
    let name = eval_string_arg("ISDEFINED", &args[0], "variable name", env, span, debug)?;
    Ok(Value::Boolean(env.borrow().get(&name).is_some()))
}

fn eval_builtin_getvar(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.is_empty() || args.len() > 2 {
        return Err(runtime_err(
            "GETVAR requires one or two arguments",
            span,
            env,
        ));
    }
    let name = eval_string_arg("GETVAR", &args[0], "variable name", env, span, debug)?;
    // Bound to a local before the match, so the environment is *not* still
    // borrowed while the default expression is evaluated. A default with a side
    // effect -- `GETVAR("a", SETVAR("b", 1))` -- needs to take a mutable borrow,
    // and a match on `env.borrow().get(..)` holds the shared one across the arms.
    let existing = env.borrow().get(&name);
    match existing {
        Some(value) => Ok(value),
        None if args.len() == 2 => evaluate_node(&args[1], Rc::clone(env), debug),
        None => Err(runtime_err(
            format!("Variable '{}' is not defined", name),
            span,
            env,
        )),
    }
}

fn eval_builtin_setvar(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("SETVAR", args, 2, env, span)?;
    let name = eval_string_arg("SETVAR", &args[0], "variable name", env, span, debug)?;
    if !is_assignable_name(&name) {
        return Err(runtime_err(
            format!(
                "'{}' is not a usable variable name: it must start with a letter and contain only letters, digits and underscores",
                name
            ),
            span,
            env,
        ));
    }
    let value = evaluate_node(&args[1], Rc::clone(env), debug)?;
    let returned = value.clone();
    // Writes into the current scope, exactly as `name <- value` does.
    env.borrow_mut().set(name, value);
    Ok(returned)
}

fn eval_builtin_unsetvar(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    expect_arity("UNSETVAR", args, 1, env, span)?;
    let name = eval_string_arg("UNSETVAR", &args[0], "variable name", env, span, debug)?;
    Ok(Value::Boolean(env.borrow_mut().unset(&name)))
}

/// Whether a string could have been written as an identifier in source.
///
/// SETVAR is the one way to make a binding from a computed name, so it is the one
/// place a name could appear that no ordinary code could read back. The test is the
/// lexer itself: the name must tokenize to exactly one `Identifier` holding the
/// whole string. That rejects punctuation and digits-first names, and it also
/// rejects every keyword -- `SETVAR("IF", 1)` used to succeed and leave a binding
/// `DISPLAY(IF)` could never reach -- without keeping a second copy of the keyword
/// list in step with the lexer.
fn is_assignable_name(name: &str) -> bool {
    let mut lexer = crate::lexer::Lexer::new(name);
    matches!(
        lexer.tokenize().as_slice(),
        [(crate::lexer::Token::Identifier(ident), _)] if ident == name
    )
}

fn eval_builtin_variables(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> EvalResult {
    expect_no_args("VARIABLES", args, env, span)?;
    Ok(Value::List(
        env.borrow()
            .visible_variable_names()
            .into_iter()
            .map(Value::String)
            .collect(),
    ))
}

fn eval_builtin_procedures(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> EvalResult {
    expect_no_args("PROCEDURES", args, env, span)?;
    Ok(Value::List(
        env.borrow()
            .procedure_names()
            .into_iter()
            .map(Value::String)
            .collect(),
    ))
}

fn eval_builtin_call(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
    debug: bool,
) -> EvalResult {
    if args.is_empty() || args.len() > 2 {
        return Err(runtime_err("CALL requires one or two arguments", span, env));
    }
    let name = eval_string_arg("CALL", &args[0], "procedure name", env, span, debug)?;
    let call_args = match args.get(1) {
        None => Vec::new(),
        Some(arg) => match evaluate_node(arg, Rc::clone(env), debug)? {
            Value::List(items) => items,
            _ => {
                return Err(runtime_err(
                    "CALL requires a list of arguments as its second argument",
                    span,
                    env,
                ));
            }
        },
    };
    // Deliberately user-defined procedures only. Built-ins take unevaluated
    // arguments so that INPUT, RANDOM and the assignment-style list operations
    // can see their own syntax, and there is no need to reach them by name: a
    // built-in's name is known when the program is written.
    if env.borrow().get_procedure(&name).is_none() {
        return Err(runtime_err(
            format!(
                "CALL could not find a procedure named '{}'. CALL dispatches to procedures declared with PROCEDURE, not to built-in functions.",
                name
            ),
            span,
            env,
        ));
    }
    invoke_procedure(&name, call_args, env, span, debug)
}

// ---------------------------------------------------------------------------
// The files a program is made of
// ---------------------------------------------------------------------------

fn eval_builtin_scriptpath(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> EvalResult {
    expect_no_args("SCRIPTPATH", args, env, span)?;
    // NULL when the program did not come from a file at all -- EVAL, the library
    // API and the browser playground. A program that wants to locate its own
    // data files has to be able to tell that case apart from a real path.
    let path = env
        .borrow()
        .modules
        .borrow()
        .current_file()
        .map(|p| p.to_string_lossy().into_owned());
    Ok(optional_string(path))
}

fn eval_builtin_ismain(args: &[Spanned], env: &Rc<RefCell<Environment>>, span: Span) -> EvalResult {
    expect_no_args("ISMAIN", args, env, span)?;
    let modules = Rc::clone(&env.borrow().modules);
    let modules = modules.borrow();
    // True exactly when the code running was written in the file the interpreter
    // was pointed at. Compared against the entry rather than testing an empty
    // stack, because a procedure call now enters its own defining file -- and a
    // procedure written in the entry script is still part of the entry script.
    // This is what lets a library carry a demo or a self-test that stays quiet
    // when the file is imported, the way `if __name__ == "__main__"` does.
    Ok(Value::Boolean(
        match (&modules.entry, modules.current_file()) {
            (Some(entry), Some(current)) => Rc::ptr_eq(entry, &current) || entry == &current,
            _ => false,
        },
    ))
}

fn eval_builtin_modules(
    args: &[Spanned],
    env: &Rc<RefCell<Environment>>,
    span: Span,
) -> EvalResult {
    expect_no_args("MODULES", args, env, span)?;
    let loaded: Vec<Value> = env
        .borrow()
        .modules
        .borrow()
        .loaded
        .iter()
        .map(|p| Value::String(p.to_string_lossy().into_owned()))
        .collect();
    Ok(Value::List(loaded))
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
        // A value that is not NULL really is unequal to NULL. These two arms used
        // to fall into the catch-all below, which answered `false` to *every*
        // NULL comparison -- so `x = NULL` and `x NOT= NULL` were both false and
        // there was no way to ask whether something was NULL at all. NULL is not
        // NaN: it is a definite absence, and it compares as one.
        (Value::Null, BinaryOperator::Eq, _) | (_, BinaryOperator::Eq, Value::Null) => {
            Ok(Value::Boolean(false))
        }
        (Value::Null, BinaryOperator::NotEq, _) | (_, BinaryOperator::NotEq, Value::Null) => {
            Ok(Value::Boolean(true))
        }
        // Ordering against NULL stays meaningless, and answers `false` either way.
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
        (Value::Float(a), BinaryOperator::Mod, Value::Float(b)) => {
            if *b == 0.0 {
                Err("Modulo by zero".to_string())
            } else {
                Ok(Value::Float(a % b))
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

        // List equality, so that a comparison behaves the same hoisted out of a
        // dictionary as it does inside one.
        (Value::List(_), BinaryOperator::Eq, Value::List(_)) => {
            Ok(Value::Boolean(values_equal(left, right)))
        }
        (Value::List(_), BinaryOperator::NotEq, Value::List(_)) => {
            Ok(Value::Boolean(!values_equal(left, right)))
        }

        // Dictionary equality (order-insensitive) and merge
        (Value::Dictionary(_), BinaryOperator::Eq, Value::Dictionary(_)) => {
            Ok(Value::Boolean(values_equal(left, right)))
        }
        (Value::Dictionary(_), BinaryOperator::NotEq, Value::Dictionary(_)) => {
            Ok(Value::Boolean(!values_equal(left, right)))
        }
        (Value::Dictionary(a), BinaryOperator::Add, Value::Dictionary(b)) => {
            let mut result = a.clone();
            for (key, value) in b.iter() {
                result.insert(key.clone(), value.clone());
            }
            Ok(Value::Dictionary(result))
        }

        _ => Err(format!(
            "Invalid operation: {:?} {:?} {:?}",
            left, op, right
        )),
    }
}

/// Deep, structural equality. Dictionaries compare order-insensitively; NaN is
/// never equal to anything, mirroring the binary-operator rules above.
fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::NaN, _) | (_, Value::NaN) => false,
        (Value::Integer(a), Value::Integer(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Integer(a), Value::Float(b)) => bigint_to_f64(a) == *b,
        (Value::Float(a), Value::Integer(b)) => *a == bigint_to_f64(b),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::List(a), Value::List(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| values_equal(x, y))
        }
        (Value::Dictionary(a), Value::Dictionary(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| values_equal(value, other)))
        }
        (Value::Null, Value::Null) | (Value::Unit, Value::Unit) => true,
        _ => false,
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
        Value::Dictionary(entries) => {
            let entries_str: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("{}: {}", key_to_string(k), value_to_string(v)))
                .collect();
            format!("{{{}}}", entries_str.join(", "))
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
