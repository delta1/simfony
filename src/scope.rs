use crate::array::{DirectedTree, Direction};
use crate::parse::{Identifier, Pattern, WitnessName};
use crate::{named::ProgExt, ProgNode};

/// Tracker of variable bindings and witness names.
///
/// Internally there is a stack of scopes.
/// A new scope is pushed for each (nested) block expression.
///
/// Bindings from higher scopes (in the stack) overwrite bindings from lower scopes.
#[derive(Debug, Clone)]
pub struct GlobalScope {
    variables: Vec<Vec<Pattern>>,
    witnesses: Vec<Vec<WitnessName>>,
}

impl Default for GlobalScope {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalScope {
    /// Creates a new [`GlobalScope`].
    pub fn new() -> Self {
        GlobalScope {
            variables: vec![Vec::new()],
            witnesses: vec![Vec::new()],
        }
    }

    /// Pushes a new scope to the stack.
    pub fn push_scope(&mut self) {
        self.variables.push(Vec::new());
        self.witnesses.push(Vec::new());
    }

    /// Pops the latest scope from the stack.
    ///
    /// # Panics
    ///
    /// Panics if the stack is empty.
    pub fn pop_scope(&mut self) {
        self.variables.pop().expect("Popping scope zero");
        self.witnesses.pop().expect("Popping scope zero");
    }

    /// Pushes a new variable to the latest scope.
    pub fn insert(&mut self, pattern: Pattern) {
        self.variables.last_mut().unwrap().push(pattern);
    }

    /// Pushes a new witness to the latest scope.
    pub fn insert_witness(&mut self, key: WitnessName) {
        self.witnesses.last_mut().unwrap().push(key);
    }

    /// Get a Simplicity expression that returns the value of the given `identifier`.
    ///
    /// The expression is a sequence of `take` and `drop` followed by `iden`,
    /// which extracts the seeked value from the environment.
    ///
    /// The environment starts as the unit value.
    ///
    /// Each let statement updates the environment to become
    /// the product of the assigned value (left) and of the previous environment (right).
    ///
    /// (Block) expressions consume the environment and return their output.
    ///
    /// ## Example
    ///
    /// ```
    /// let a: u8 = 0;
    /// let b = {
    ///     let b: u8 = 1;
    ///     let c: u8 = 2;
    ///     a  // here we seek the value of `a`
    /// };
    /// ```
    ///
    /// The stack of scopes looks like this:
    ///
    /// `[a] [b c]`
    ///
    /// The environment looks like this:
    ///
    /// ```text
    ///   .
    ///  / \
    /// C   .
    ///    / \
    ///   B   .
    ///      / \
    ///     A   1
    /// ```
    ///
    /// To extract `a`, we need the expression `drop drop take iden`.
    ///
    /// ## Panics
    ///
    /// The `identifier` is undefined.
    pub fn get(&self, identifier: &Identifier) -> ProgNode {
        let mut pos = 0;

        // Highest scope has precedence
        for scope in self.variables.iter().rev() {
            // Last let statement has precedence
            if let Some((pattern_pos, mut expr)) =
                scope.iter().rev().enumerate().find_map(|(idx, pattern)| {
                    pattern.get_program(identifier).map(|expr| (idx, expr))
                })
            {
                pos += pattern_pos;

                expr = ProgNode::take(expr);
                for _ in 0..pos {
                    expr = ProgNode::drop_(expr);
                }

                return expr;
            } else {
                pos += scope.len();
            }
        }

        panic!("\"{identifier}\" is undefined");
    }
}

impl Pattern {
    pub fn get_identifier(&self) -> Option<&Identifier> {
        match self {
            Pattern::Identifier(i) => Some(i),
            _ => None,
        }
    }

    pub fn get_program(&self, identifier: &Identifier) -> Option<ProgNode> {
        let base_pattern = self.to_base();
        let directed_tree = DirectedTree::from(&base_pattern);
        let equals_identifier = |pattern: &Pattern| {
            pattern
                .get_identifier()
                .map(|i| i == identifier)
                .unwrap_or(false)
        };
        let (_, mut path) = directed_tree.find(equals_identifier)?;

        let mut output = ProgNode::iden();
        while let Some(direction) = path.pop() {
            match direction {
                Direction::Left => output = ProgNode::take(output),
                Direction::Right => output = ProgNode::drop_(output),
                Direction::Down => unreachable!("There are no unary patterns"),
                Direction::Index(..) => unreachable!("Base patterns exclude arrays"),
            }
        }

        Some(output)
    }
}
