use std::collections::BTreeMap;

use crate::{
    block::Block,
    eval::{atom::Atom, eval_expr},
    parser::*,
};

#[derive(Debug, Clone)]
pub struct State {
    pub scopes: Vec<Scope>,
}

impl Default for State {
    fn default() -> Self {
        State {
            scopes: vec![Scope::default()],
        }
    }
}

impl State {
    pub fn get_variable(&self, var: &str) -> Option<&Atom> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.vars.get(var))
    }

    fn modify_variable(&mut self, var: &str, val: Atom) {
        // dbg!(self.scopes.clone());
        for scope in &mut self.scopes.iter_mut().rev() {
            if scope.vars.contains_key(var) {
                scope.vars.insert(var.to_string(), val);
                break;
            }
        }
    }

    pub fn declare(&mut self, dec: Declaration) {
        let (val, disc) = {
            let var = self.get_variable(&dec.lhs);
            (var.cloned(), var.map(std::mem::discriminant))
        };

        match (disc, dec.alias) {
            (_, true) => {
                let new_val = eval_expr(&dec.rhs, self);
                self.scopes
                    .last_mut()
                    .unwrap()
                    .vars
                    .insert(dec.lhs, new_val);
            }
            (Some(d), false) => {
                let rhs_val = eval_expr(&dec.rhs, self);
                if d == std::mem::discriminant(&&rhs_val) {
                    // dbg!(dec.lhs.clone(), val.clone());
                    let new_val = match dec.plus_or_minus {
                        Some(true) => val.unwrap() + rhs_val,
                        Some(false) => val.unwrap() - rhs_val,
                        None => rhs_val,
                    };

                    // dbg!(new_val.clone());
                    self.modify_variable(&dec.lhs, new_val);
                    // dbg!(self.get_variable(&dec.lhs));
                } else {
                    panic!("Cannot assign {:?} to {:?}", rhs_val, val);
                }
            }
            (None, false) => {
                panic!("Uninitialized variable {}", dec.lhs)
            }
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Scope {
    pub vars: BTreeMap<String, Atom>,
}

#[derive(Default, Debug, Clone)]
pub struct CompileScope {
    pub vars: BTreeMap<String, usize>,
    pub label_count: usize,
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub lhs: String,
    pub rhs: S,
    pub alias: bool,
    pub plus_or_minus: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct If {
    pub cond: S,
    pub then_block: Block,
    pub else_block: Block,
}

#[derive(Debug, Clone)]
pub struct While {
    pub cond: S,
    pub loop_block: Block,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    ExprStmt(S),
    PrintStmt(S),
    Dec(Declaration),
    IfStmt(If),
    WhileStmt(While),
    Block(Block),
    Break,
}

impl Stmt {
    pub fn execute(self, state: &mut State) -> Option<Atom> {
        match self {
            Stmt::ExprStmt(expr) => Some(eval_expr(&expr, state)),
            Stmt::PrintStmt(expr) => {
                println!("{}", eval_expr(&expr, state));
                None
            }
            Stmt::Dec(dec) => {
                state.declare(dec);
                None
            }
            Stmt::IfStmt(if_data) => {
                let If {
                    cond,
                    mut then_block,
                    mut else_block,
                } = if_data;

                if eval_expr(&cond, state) == Atom::Bool(true) {
                    then_block.execute(state)
                } else {
                    else_block.execute(state)
                }
            }
            Stmt::WhileStmt(while_data) => {
                let While {
                    cond,
                    mut loop_block,
                } = while_data;

                let mut res = None;

                while eval_expr(&cond, state) == Atom::Bool(true) {
                    res = loop_block.execute(state);
                    if matches!(res, Some(Atom::Break)) {
                        res = None;
                        break;
                    }
                }

                res
            }
            Stmt::Block(mut b) => b.execute(state),
            Stmt::Break => Some(Atom::Break),
        }
    }

    pub fn compile(&self, scope: &mut CompileScope) {
        use Stmt::*;

        println!();
        match self {
            ExprStmt(s) => {
                s.compile(scope);
            }
            PrintStmt(s) => {
                s.compile(scope);
                println!("Print");
                println!("Push 10");
                println!("PrintC");
                println!("Pop");
                println!("Pop");
            }
            Dec(Declaration {
                lhs,
                rhs,
                alias,
                plus_or_minus,
            }) => {
                if scope.vars.keys().any(|k| k == lhs) {
                    if !alias && plus_or_minus.is_some() {
                        let i = *scope.vars.get(lhs).unwrap();
                        println!("Get {}", i);
                        rhs.compile(scope);
                        if plus_or_minus.unwrap() {
                            println!("Add");
                        } else {
                            println!("Sub");
                        }
                        println!("Set {}", i);
                        println!("Pop");
                    } else {
                        let i = *scope.vars.get(lhs).unwrap();
                        rhs.compile(scope);
                        println!("Set {}", i);
                        println!("Pop");
                    }
                } else {
                    scope.vars.insert(lhs.to_string(), scope.vars.len());
                    rhs.compile(scope);
                }
            }
            IfStmt(If {
                cond,
                then_block,
                else_block,
            }) => {
                cond.compile(scope);
                println!("JE {}", scope.label_count);
                println!("Pop");
                then_block.compile(scope);
                println!("Jump {}", scope.label_count + 1);
                println!("label {}", scope.label_count);
                else_block.compile(scope);
                println!("label {}", scope.label_count + 1);
                scope.label_count += 2;
            }
            WhileStmt(While { cond, loop_block }) => {
                println!("label {}", scope.label_count + 1);
                cond.compile(scope);
                println!("JE {}", scope.label_count + 2);
                loop_block.compile(scope);
                println!("Jump {}", scope.label_count + 1);
                println!("label {}", scope.label_count + 2);
                scope.label_count += 2;
            }
            Block(crate::block::Block { statements }) => {
                statements.iter().for_each(|s| s.compile(scope));
            }
            Break => {}
        }
        println!();
    }
}

#[cfg(test)]
mod stmt_tests {
    use crate::run_file;
    use crate::Atom;
    use crate::State;

    macro_rules! test_files {
        () => {};
        ( $fn_name:ident, $file:expr => $expected:expr; $($tail:tt)* ) => {
            #[test]
            fn $fn_name() {
                let mut top_state = State::default();
                let output = run_file(format!("test_files/{}", $file), &mut top_state).unwrap();
                assert_eq!(output, $expected);
            }

            test_files!($($tail)*);
        };
        ( $fn_name:ident, $file:expr; $($tail:tt)* ) => {
            #[test]
            #[should_panic]
            fn $fn_name() {
                let mut top_state = State::default();
                run_file(format!("test_files/{}", $file), &mut top_state).unwrap();
            }

            test_files!($($tail)*);
        };
    }

    test_files!(
        basic1, "basic1.slang" => Some(Atom::Int(20));
        basic2, "basic2.slang" => Some(Atom::Int(5));
        if1, "if.slang" => Some(Atom::Str("hello".to_string()));
        if2, "else.slang" => Some(Atom::Str("goodbye".to_string()));
        scope_modify, "scope_modify.slang" => Some(Atom::Int(2));
        while1, "while1.slang" => Some(Atom::Int(10));
        for1, "for1.slang" => Some(Atom::Int(1053));
        fn1, "fn1.slang" => Some(Atom::Int(120));
        euler01, "project_euler_01.slang" => Some(Atom::Int(233168));
        euler02, "project_euler_02.slang" => Some(Atom::Int(4613732));
        scoped_loop, "scoped_loop.slang" => Some(Atom::Int(45));
        loop_break, "loop_break.slang" => Some(Atom::Int(5));
        nested_loop_break, "nested_loop_break.slang" => Some(Atom::Int(25));
        recur1, "recursion01.slang" => Some(Atom::Int(987));
        error1, "error1.slang";
        scope_typecheck, "scope_typecheck.slang";
    );
}
