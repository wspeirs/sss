use super::Rule;
use pest::iterators::Pair;

use std::collections::HashMap;
use std::fmt;

use crate::parse_error::ParseError;

use crate::expression::*;

// helpful type alias
type SymbolTable = HashMap<String, Variable>;
type FunctionTable = HashMap<String, Function>;

#[derive(Debug, Clone)]
pub struct Script {
    functions: FunctionTable,   // the functions defined in this script + built-ins
    variables: SymbolTable,     // variables and their current values
    code: Vec<Expression>       // list of code to execute in order
}

static mut tmp_num: usize = 0; // counter to generate unique temporary variables

impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "FUNCTIONS: {:?}", self.functions)?;
        writeln!(f, "VARIABLES: {:?}", self.variables)?;

        for e in &self.code {
            writeln!(f, "{}", e)?;
        }

        Ok( () )
    }
}

impl Script {
    /// Generates a temp variable with the same type as the variable passed
    fn generate_temp(var_def: &VarDef) -> Variable {
        let var_name = unsafe {
            tmp_num += 1;
            String::from(format!("_t{}", tmp_num))
        };

        Variable{ name:var_name, var_def: var_def.clone() }
    }

    /// Constructs a Script object from a set of rules return from the parser
    pub fn new(script: Pair<Rule>) -> Result<Script, ParseError> {
        let mut functions :FunctionTable = HashMap::new();
        let mut variables :SymbolTable = HashMap::new();

        // construct all of our built-in functions
        let run_fun = Function::built_in("run", vec![
                Variable{name: String::from("input"), var_def: VarDef{var_type:VarType::Pipe, is_array:false}},
                Variable{name: String::from("exec"), var_def: VarDef{var_type:VarType::String, is_array:false}}
            ], Some(VarDef{var_type:VarType::Pipe, is_array:true}));

        functions.insert(String::from("run"), run_fun);

/*
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
*/
        let inner = script.into_inner();
        let mut code = Vec::new();

        // now go through all the program lines
        for inner in inner {
            let exp = match inner.as_rule() {
                Rule::program_line => {
                    Script::process_program_line(inner, &mut variables, &functions)?
                },
                Rule::fun => { continue },
                Rule::EOI => { break }
                _ => { panic!("UNKNOWN: {:?}", inner) }
            };

            code.extend(exp);
        }

        Ok(Script {
            functions,
            variables,
            code
        })
    }

    fn process_program_line(program_line: Pair<Rule>, variables: &mut SymbolTable, functions: &FunctionTable) -> Result<Vec<Expression>, ParseError> {
        let program_line = program_line.into_inner().next().unwrap();
        let pl_str = String::from(program_line.as_str());

        let exp = match program_line.as_rule() {
            Rule::declaration => {
                // var_def, expression
                let mut inner = program_line.into_inner();

                let var_def = inner.next().unwrap();
                let lhs = Variable::new(var_def);

                if variables.insert(lhs.name.clone(), lhs.clone()).is_some() {
                    panic!("Variable redefinition: {}", pl_str);
                }

                // process the expression on the right-hand-side
                let (mut exps, rhs) = Script::process_expression(inner.next().unwrap(), variables, functions)?;

                exps.push(Expression::Assignment(pl_str, Assignment{ lhs, rhs }));

                exps
            },
            Rule::assignment => {
                // identifier, expression
                let mut inner = program_line.clone().into_inner();

                let ident = inner.next().unwrap().as_str();

                // check to make sure we've previously declared this variable
                if !variables.contains_key(ident) {
                    return Err(ParseError::new(program_line, format!("Assignment to undeclared variable: {}", ident)));
                }

                let var = variables.get(ident).unwrap().clone();

                let (mut exps, rhs) = Script::process_expression(inner.next().unwrap(), variables, functions)?;

                exps.push(Expression::Assignment(pl_str, Assignment {lhs:var.clone(), rhs}));

                exps
            },
            Rule::method_call => {
                let pl_str = String::from(program_line.as_str());
                let fun_call = Script::process_method_call(program_line.into_inner().next().unwrap(), variables, functions)?;

                vec![Expression::FunctionCall(pl_str, fun_call)]
            },
            Rule::fun_call => {
                let pl_str = String::from(program_line.as_str());
                let fun_call = Script::process_fun_call(program_line.into_inner().next().unwrap(), variables, functions)?;

                vec![Expression::FunctionCall(pl_str, fun_call)]
            },
            _ => { panic!("Unknown program line expansion: {}", pl_str); }
        };

        Ok(exp)
    }

    fn process_expression(expression: Pair<Rule>, variables: &mut SymbolTable, functions: &FunctionTable) -> Result<(Vec<Expression>, RightHandSide), ParseError> {
        let mut exps = Vec::new();

        let exp_str = String::from(expression.as_str());
        let mut inner = expression.clone().into_inner();

        let (primary_exp, op1) = Script::process_primary(inner.next().unwrap(), variables, functions)?;

        exps.extend(primary_exp);

        // we'll always generate at least one assignment
        // as we assume the type is the same as the first operand
        let lhs = Script::generate_temp(&op1.var_def);

        variables.insert(lhs.name.clone(), lhs.clone());

        let rhs = if inner.peek().is_some() {
            let op_rule = inner.next().unwrap();
            let op = match op_rule.as_str() {
                "+" => Operator::Add,
                "-" => Operator::Sub,
                "*" => Operator::Mul,
                "/" => Operator::Div,
                _ => return Err(ParseError::new(expression, format!("Unknown operator {}", op_rule.as_str())))
            };

            let (primary_exps, op2) = Script::process_primary(inner.next().unwrap(), variables, functions)?;

            if op1.var_def != op2.var_def {
                return Err(ParseError::new(expression, format!("Attempting to combine values of different types {:?} != {:?}", op1.var_def, op2.var_def)))
            }

            exps.extend(primary_exps);

            RightHandSide::Operation(op1, op, op2)
        } else {
            RightHandSide::Variable(op1)
        };

        exps.push(Expression::Assignment(exp_str.clone(), Assignment{lhs:lhs.clone(), rhs}));

        while inner.peek().is_some() {
            let op1 = lhs.clone();
            let lhs = Script::generate_temp(&op1.var_def);

            variables.insert(lhs.name.clone(), lhs.clone());

            let op_rule = inner.next().unwrap();
            let op = match op_rule.as_str() {
                "+" => Operator::Add,
                "-" => Operator::Sub,
                "*" => Operator::Mul,
                "/" => Operator::Div,
                _ => return Err(ParseError::new(expression, format!("Unknown operator {}", op_rule.as_str())))
            };

            let (primary_exps, op2) = Script::process_primary(inner.next().unwrap(), variables, functions)?;

            exps.extend(primary_exps);

            let rhs = RightHandSide::Operation(op1, op, op2);

            exps.push(Expression::Assignment(exp_str.clone(), Assignment{lhs, rhs}));
        }

        Ok( (exps, RightHandSide::Variable(lhs)) )
    }

    fn process_primary(primary: Pair<Rule>, variables: &mut SymbolTable, functions: &FunctionTable) -> Result<(Vec<Expression>, Variable), ParseError> {
        let mut exps = Vec::new();
        let ret_var;

        let p_str = String::from(primary.as_str());
        let inner = primary.clone().into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::method_call => {
                let fc = Script::process_method_call(inner, variables, functions)?;

                if fc.fun.ret_type.is_none() {
                    return Err(ParseError::new(primary, String::from("Attempting to use a method that not return a value in an expression")))
                }

                let lhs = Script::generate_temp(&fc.clone().fun.ret_type.unwrap());

                exps.push(Expression::Assignment(p_str, Assignment{
                    lhs: lhs.clone(),
                    rhs: RightHandSide::FunctionCall(fc)
                }));

                ret_var = lhs;
            },
            Rule::fun_call => {
                let fc = Script::process_fun_call(inner, variables, functions)?;

                if fc.fun.ret_type.is_none() {
                    return Err(ParseError::new(primary, String::from("Attempting to call a function that not return a value in an expression")))
                }

                let lhs = Script::generate_temp(&fc.clone().fun.ret_type.unwrap());

                exps.push(Expression::Assignment(p_str, Assignment{
                    lhs: lhs.clone(),
                    rhs: RightHandSide::FunctionCall(fc)
                }));

                ret_var = lhs;
            },
            Rule::expression => {
                let (t_exp, rhs) = Script::process_expression(inner, variables, functions)?;

                exps.extend(t_exp);

                let var_def = match rhs.clone() {
                    RightHandSide::Variable(v) => v.var_def,
                    RightHandSide::Operation(v, _, _) => v.var_def,
                    RightHandSide::Term(t) => {
                        match t {
                            Term::String(_) => VarDef::from_type(&VarType::String),
                            Term::Number(_) => VarDef::from_type(&VarType::Number),
                            Term::Variable(v) => v.var_def.clone()
                        }
                    }
                    RightHandSide::FunctionCall(f) => {
                        if f.fun.ret_type.is_none() {
                            return Err(ParseError::new(primary, String::from("Attempting to call a function that not return a value in an expression")))
                        } else {
                            f.fun.ret_type.unwrap()
                        }
                    }
                };

                let lhs = Script::generate_temp(&var_def);

                exps.push(Expression::Assignment(p_str, Assignment{
                    lhs: lhs.clone(),
                    rhs: rhs
                }));

                ret_var = lhs;
            },
            Rule::term => {
                let inner = inner.into_inner().next().unwrap();

                ret_var = match inner.as_rule() {
                    Rule::identifier => {
                        let ident = inner.as_str();

                        if let Some(v) = variables.get(ident) {
                            v.clone()
                        } else {
                            return Err(ParseError::new(primary, String::from(format!("Undefined variable {}", inner.as_str()))))
                        }
                    },
                    Rule::string => {
                        let term = Term::String(String::from(inner.as_str()));
                        let lhs = Script::generate_temp(&VarDef::from_type(&VarType::String));

                        exps.push(Expression::Assignment(p_str, Assignment{
                            lhs: lhs.clone(),
                            rhs: RightHandSide::Term(term)
                        }));

                        lhs
                    },
                    Rule::number => {
                        let term = Term::Number(inner.as_str().parse::<f64>().unwrap());
                        let lhs = Script::generate_temp(&VarDef::from_type(&VarType::Number));

                        exps.push(Expression::Assignment(p_str, Assignment{
                            lhs: lhs.clone(),
                            rhs: RightHandSide::Term(term)
                        }));

                        lhs
                    },
                    _ => return Err(ParseError::new(primary, String::from(format!("Unknown term type: {} ({:?})", inner.as_str(), inner.as_rule()))))
                };
            },
            _ => { return Err(ParseError::new(primary, String::from("Unknown primary"))) }
        }

        Ok( (exps, ret_var) )
    }

    fn process_method_call(method_call: Pair<Rule>, variables: &SymbolTable, functions: &FunctionTable) -> Result<FunctionCall, ParseError> {
        let mc_str = method_call.as_str();
        // fun_call | identifier, fun_call
        let mut inner = method_call.clone().into_inner();

        let first = inner.next().unwrap(); // either fun_call or ident
        let fun_call = inner.next().unwrap();

        let fun_call = match first.as_rule() {
            Rule::identifier => {
                if let Some(var) = variables.get(first.as_str()) {
                    if let VarType::Pipe = var.var_def.var_type {
                        if let Some(fun) = functions.get(fun_call.as_str()) {
                            if let FunctionType::BuiltIn = fun.fun_type {
                                Script::process_fun_call(fun_call, variables, functions)?
                            } else {
                                return Err(ParseError::new(method_call, format!("Attempting to call user defined function {} on pipe {}", fun_call.as_str(), first.as_str())));
                            }
                        } else {
                            return Err(ParseError::new(method_call, format!("Unknown function {}", fun_call.as_str())));
                        }
                    } else {
                        return Err(ParseError::new(method_call, format!("Cannot call a method on a non-pipe variable: {}", first.as_str())));
                    }
                } else {
                    return Err(ParseError::new(method_call, format!("Unknown variable {}", first.as_str())));
                }
            },
            Rule::fun_call => {
                Script::process_fun_call(first, variables, functions)?
            },
            _ => { panic!("Unknown expansion for method_call: {}", mc_str); }
        };

        Ok(fun_call)
    }

    fn process_fun_call(fun_call: Pair<Rule>, variables: &SymbolTable, functions: &FunctionTable) -> Result<FunctionCall, ParseError> {
        let fc_str = fun_call.as_str();
        let mut inner = fun_call.clone().into_inner();

        let name = String::from(inner.next().unwrap().as_str());

        let fun = if let Some(fun) = functions.get(&name) {
            fun.clone()
        } else {
            return Err(ParseError::new(fun_call, String::from(format!("Unknown function {}", name))));
        };

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

        Ok(FunctionCall{ fun, var_list })
    }
}
