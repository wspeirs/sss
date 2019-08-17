use super::Rule;
use pest::iterators::Pair;

use std::collections::HashMap;

use crate::expression::{Expression, Assignment, VarType, Variable, FunctionCall, Executable};

// helpful type alias
type SymbolTable = HashMap<String, Variable>;
type FunctionTable = HashMap<String, String>;

pub struct Script {
    functions: FunctionTable,
    variables: SymbolTable
}

impl Script {
    /// Constructs a Script object from a set of rules return from the parser
    pub fn new(script: Pair<Rule>) -> Script {
        let mut functions :FunctionTable = HashMap::new();
        let mut variables :SymbolTable = HashMap::new();

        let inner = script.clone().into_inner();

        // loop through all functions first, to build up functions hash map
        for inner in inner {
            match inner.as_rule() {
                Rule::program_line => { continue },
                Rule::fun => {
                    let ret = Script::process_fun(inner);

                    if functions.insert(ret.0.clone(), ret.1).is_some() {
                        panic!("Function re-definition: {}", ret.0);
                    }
                },
                Rule::EOI => { }
                _ => { println!("UNKNOWN: {:?}", inner) }
            }
        }

        let inner = script.into_inner();

        // now go through all the program lines
        for inner in inner {
            match inner.as_rule() {
                Rule::program_line => {
                    Script::process_program_line(inner, &mut variables, &functions);
                },
                Rule::fun => { continue },
                Rule::EOI => { }
                _ => { println!("UNKNOWN: {:?}", inner) }
            }
        }

        println!("FUNCTIONS:");
        functions.iter().for_each(|f| println!("{:?}", f));

        println!("VARIABLES:");
        variables.iter().for_each(|v| println!("{:?}", v));

        Script {
            functions,
            variables
        }
    }

    fn process_program_line(program_line: Pair<Rule>, variables: &mut SymbolTable, functions: &FunctionTable) -> Expression {
        let program_line = program_line.into_inner().next().unwrap();
        let pl_str = program_line.as_str();

        match program_line.as_rule() {
            Rule::declaration => {
                // var_def, expression
                let mut inner = program_line.into_inner();

                let mut var_def = inner.next().unwrap();
                let lhs = Variable::new(var_def);

                if variables.insert(lhs.name.clone(), lhs.clone()).is_some() {
                    panic!("Variable redefinition: {}", pl_str);
                }

                let exp = Executable{ contents:String::from(inner.next().unwrap().as_str()) };

                Expression::Assignment(Assignment{ lhs, rhs:exp })
            },
            Rule::assignment => {
                // identifier, expression
                let mut inner = program_line.into_inner();

                let ident = String::from(inner.next().unwrap().as_str());
                let exp = Executable{ contents:String::from(inner.next().unwrap().as_str()) };

                // check to make sure we've previously declared this variable
                if let Some(var) = variables.get(&ident) {
                    Expression::Assignment(Assignment {lhs:var.clone(), rhs:exp})
                } else {
                    panic!("Assigning to undeclared variable: {}", pl_str);
                }
            },
            Rule::method_call => {
                // fun_call | identifier, fun_call
                let mut inner = program_line.into_inner();

                let first = inner.next().unwrap();

                match first.as_rule() {
                    Rule::identifier => {
                        if let Some(var) = variables.get(first.as_str()) {
                            if let VarType::Pipe = var.var_type {
                                panic!("Cannot call a method on a non-pipe variable: {}", pl_str);
                            }
                        }
                    },
                    Rule::fun_call => {
                        Script::process_fun_call(first, variables);
                    },
                    _ => { panic!("Unknown expansion for method_call: {}", pl_str); }
                }

                // punting
                Expression::Executable(Executable{ contents: String::from("exec")})
            },
            Rule::fun_call => {
                // punting
                Expression::Executable(Executable{ contents: String::from("exec")})
            },
            Rule::expression => {
                // punting
                Expression::Executable(Executable{ contents: String::from("exec")})
            },
            _ => { panic!("Unknown program line expansion: {}", pl_str); }
        }
    }

    fn process_fun_call(fun_call: Pair<Rule>, variables: &SymbolTable) -> FunctionCall {
        let fc_str = fun_call.as_str();
        let mut inner = fun_call.into_inner();

        let name = String::from(inner.next().unwrap().as_str());

        let var_list = if inner.peek().is_some() {
            inner.next().unwrap().into_inner().map(|v| Variable::new(v)).collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // make sure all the variables in the list are known
        for var in &var_list {
            if !variables.contains_key(&var.name) {
                panic!("Unknown variable {} in: {}", var.name, fc_str);
            }
        }

        FunctionCall{ name, var_list }
    }

    fn process_fun(fun: Pair<Rule>) -> (String, String) {
        let mut inner = fun.into_inner();
        let signature = Script::process_fun_signature(inner.next().unwrap());
        let block = inner.next().unwrap();

        (format!("{:?}", signature.0), String::from("world"))
    }

    /// Break a function signature down into it's 3 parts
    fn process_fun_signature(fun_signature :Pair<Rule>) -> (String, String, Option<String>) {
        (String::from("hello"), String::from("world"), Some(String::from("blah")))
    }
}
