use super::Rule;
use pest::iterators::Pair;

use std::collections::HashMap;
use std::fmt;

use crate::parse_error::ParseError;

use crate::expression::*;

// helpful type alias
pub type SymbolTable = HashMap<String, Variable>;
type FunctionTable = HashMap<String, Function>;

#[derive(Debug, Clone)]
pub struct Script {
    user_functions: FunctionTable,     // the functions defined in this script + built-ins
    builtin_functions: FunctionTable,  // built-in functions
    variables: SymbolTable,            // variables and their current values
    code: Vec<Expression>,             // list of code to execute in order
    tmp_num: usize
}

impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "FUNCTIONS: {:?}", self.user_functions)?;
        writeln!(f, "VARIABLES: {:?}", self.variables)?;

        for e in &self.code {
            writeln!(f, "{}", e)?;
        }

        Ok( () )
    }
}

impl Script {
    /// Generates a temp variable with the same type as the variable passed
    fn generate_temp(&mut self, var_def: &VarDef) -> Variable {
        let var_name = unsafe {
            self.tmp_num += 1;
            String::from(format!("_t{}", self.tmp_num))
        };

        let ret = Variable{ name:var_name.clone(), var_def: var_def.clone() };

        self.variables.insert(var_name, ret.clone());

        ret
    }

    /// Constructs a Script object from a set of rules return from the parser
    pub fn new(pairs: Pair<Rule>) -> Result<Script, ParseError> {
        let mut script = Script {
            user_functions: FunctionTable::new(),
            builtin_functions: FunctionTable::new(),
            variables: SymbolTable::new(),
            code: Vec::new(),
            tmp_num: 0
        };


        // construct all of our built-in functions
        let run_fun = Function::new("run", vec![
                Variable{name: String::from("input"), var_def: VarDef{var_type:VarType::Pipe, is_array:false}},
                Variable{name: String::from("exec"), var_def: VarDef{var_type:VarType::String, is_array:false}}
            ], Some(VarDef{var_type:VarType::Pipe, is_array:true}));

        script.builtin_functions.insert(String::from("run"), run_fun);

        let inner = pairs.clone().into_inner();

        // loop through all functions first, to build up functions hash map
        for inner in inner {
            match inner.as_rule() {
                Rule::program_line => { continue },
                Rule::fun => {
                    let fun = script.process_fun(inner.clone())?;

                    let fun_name = fun.clone().name;

                    if script.builtin_functions.contains_key(&fun_name) {
                        return Err(ParseError::new(inner, format!("Re-definition of built-in function: {:?}", fun)));
                    }

                    if script.user_functions.insert(fun_name, fun.clone()).is_some() {
                        return Err(ParseError::new(inner, format!("Function re-definition: {:?}", fun)));
                    }
                },
                Rule::EOI => { }
                _ => { println!("UNKNOWN: {:?}", inner) }
            }
        }

        let inner = pairs.into_inner();

        // now go through all the program lines
        for inner in inner {
            match inner.as_rule() {
                Rule::program_line => {
                    script.process_program_line(inner)?
                },
                Rule::fun => { continue },
                Rule::EOI => { break }
                _ => { panic!("UNKNOWN: {:?}", inner) }
            };
        }

        Ok(script)
    }

    pub fn run(&self) {

    }

    fn process_fun(&mut self, fun: Pair<Rule>) -> Result<Function, ParseError> {
        let fun_str = String::from(fun.as_str());
        let mut inner = fun.clone().into_inner();

        let mut signature = inner.next().unwrap().into_inner();

        let fun_name = String::from(signature.next().unwrap().as_str());

        let mut next = signature.next();
        let mut fun_vars = SymbolTable::new();
        let mut ret_val = Option::None;

        while next.is_some() {
            match next.clone().unwrap().as_rule() {
                Rule::param_list => {
                    let param_list = next.unwrap().into_inner().map(|dec| {
                        Variable::new(dec)
                    }).collect::<Vec<_>>();

                    // insert them all into the function's symbol table
                    param_list.iter().for_each(|v| {
                        fun_vars.insert(v.clone().name, v.clone());
                    });

                },
                Rule::var_def => {
                    ret_val = Some(VarDef::new(next.unwrap()));
                },
                _ => { return Err(ParseError::new(fun, format!("Unexpected token: {:?}", next.unwrap()))) }
            }

            next = signature.next();
        }

        let block = inner.next().unwrap().into_inner();

        for pl in block {
            self.process_program_line(pl);
        }

        // this is a bit of a hack, process_program_lines is going to fill self.code
        // but we need these code attached to a function, not the "main" code
        // so we copy that Vec, and drain the other
        let mut fun_code = Vec::with_capacity(self.code.len());

        fun_code.extend(self.code.drain(1..));

        Ok( Function {
            name: fun_name,
            params: fun_vars,
            ret_type: ret_val,
            code: fun_code
        } )
    }

    fn process_program_line(&mut self, program_line: Pair<Rule>) -> Result<(), ParseError> {
        let program_line = program_line.into_inner().next().unwrap();
        let pl_str = String::from(program_line.as_str());

        match program_line.as_rule() {
            Rule::declaration => {
                // var_def, expression
                let mut inner = program_line.clone().into_inner();

                let var_def = inner.next().unwrap();
                let lhs = Variable::new(var_def);

                if self.variables.insert(lhs.name.clone(), lhs.clone()).is_some() {
                    return Err(ParseError::new(program_line, format!("Redeclaration of {}", lhs.name)));
                }

                debug!("Declared variable: {:?}", lhs);

                // process the expression on the right-hand-side
                let rhs = self.process_expression(inner.next().unwrap())?;

                self.code.push(Expression::Assignment(pl_str, Assignment{ lhs, rhs }));
            },
            Rule::assignment => {
                // identifier, expression
                let mut inner = program_line.clone().into_inner();

                let ident = inner.next().unwrap().as_str().trim();

                // check to make sure we've previously declared this variable
                if !self.variables.contains_key(ident) {
                    return Err(ParseError::new(program_line, format!("Assignment to undeclared variable: {}", ident)));
                }

                let var = self.variables.get(ident).unwrap().clone();

                let rhs = self.process_expression(inner.next().unwrap())?;

                self.code.push(Expression::Assignment(pl_str, Assignment {lhs:var.clone(), rhs}));
            },
            Rule::method_call => {
                let pl_str = String::from(program_line.as_str());
                let fun_call = self.process_method_call(program_line)?;

                self.code.push(Expression::FunctionCall(pl_str, fun_call));
            },
            Rule::fun_call => {
                let pl_str = String::from(program_line.as_str());
                let fun_call = self.process_fun_call(program_line)?;

                self.code.push(Expression::FunctionCall(pl_str, fun_call));
            },
            _ => {
                return Err(ParseError::new(program_line, String::from("Unknown program line")));
            }
        };

        Ok( () )
    }

    fn process_expression(&mut self, expression: Pair<Rule>) -> Result<RightHandSide, ParseError> {
        let exp_str = String::from(expression.as_str());
        let mut inner = expression.clone().into_inner();

        let op1 = self.process_primary(inner.next().unwrap())?;

        let mut rhs = if inner.peek().is_some() {
            let op_rule = inner.next().unwrap();
            let op = match op_rule.as_str() {
                "+" => Operator::Add,
                "-" => Operator::Sub,
                "*" => Operator::Mul,
                "/" => Operator::Div,
                _ => return Err(ParseError::new(expression, format!("Unknown operator {}", op_rule.as_str())))
            };

            let op2 = self.process_primary(inner.next().unwrap())?;

            if op1.var_def != op2.var_def {
                return Err(ParseError::new(expression, format!("Attempting to combine values of different types {:?} != {:?}", op1.var_def, op2.var_def)))
            }

            RightHandSide::Operation(op1.clone(), op, op2)
        } else {
            RightHandSide::Variable(op1.clone())
        };

        while inner.peek().is_some() {
            let lhs = self.generate_temp(&op1.var_def);

            self.code.push(Expression::Assignment(exp_str.clone(), Assignment{lhs:lhs.clone(), rhs}));

            let op1 = lhs;

            let op_rule = inner.next().unwrap();
            let op = match op_rule.as_str() {
                "+" => Operator::Add,
                "-" => Operator::Sub,
                "*" => Operator::Mul,
                "/" => Operator::Div,
                _ => return Err(ParseError::new(expression, format!("Unknown operator {}", op_rule.as_str())))
            };

            let op2 = self.process_primary(inner.next().unwrap())?;

            rhs = RightHandSide::Operation(op1, op, op2);
        }

        Ok(rhs)
    }

    fn process_primary(&mut self, primary: Pair<Rule>) -> Result<Variable, ParseError> {
        let ret_var;

        let p_str = String::from(primary.as_str());
        let inner = primary.clone().into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::method_call => {
                let fc = self.process_method_call(inner)?;

                if fc.fun.ret_type.is_none() {
                    return Err(ParseError::new(primary, String::from("Attempting to use a method that not return a value in an expression")))
                }

                let lhs = self.generate_temp(&fc.clone().fun.ret_type.unwrap());

                self.code.push(Expression::Assignment(p_str, Assignment{
                    lhs: lhs.clone(),
                    rhs: RightHandSide::FunctionCall(fc)
                }));

                ret_var = lhs;
            },
            Rule::fun_call => {
                let fc = self.process_fun_call(inner)?;

                if fc.fun.ret_type.is_none() {
                    return Err(ParseError::new(primary, String::from("Attempting to call a function that not return a value in an expression")))
                }

                let lhs = self.generate_temp(&fc.clone().fun.ret_type.unwrap());

                self.code.push(Expression::Assignment(p_str, Assignment{
                    lhs: lhs.clone(),
                    rhs: RightHandSide::FunctionCall(fc)
                }));

                ret_var = lhs;
            },
            Rule::expression => {
                let rhs = self.process_expression(inner)?;

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

                let lhs = self.generate_temp(&var_def);

                self.code.push(Expression::Assignment(p_str, Assignment{
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

                        if let Some(v) = self.variables.get(ident) {
                            v.clone()
                        } else {
                            return Err(ParseError::new(primary, String::from(format!("Undefined variable {}", inner.as_str()))))
                        }
                    },
                    Rule::string => {
                        let term = Term::String(String::from(inner.as_str()));
                        let lhs = self.generate_temp(&VarDef::from_type(&VarType::String));

                        self.code.push(Expression::Assignment(p_str, Assignment{
                            lhs: lhs.clone(),
                            rhs: RightHandSide::Term(term)
                        }));

                        lhs
                    },
                    Rule::number => {
                        let term = Term::Number(inner.as_str().parse::<f64>().unwrap());
                        let lhs = self.generate_temp(&VarDef::from_type(&VarType::Number));

                        self.code.push(Expression::Assignment(p_str, Assignment{
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

        Ok(ret_var)
    }

    fn process_method_call(&mut self, method_call: Pair<Rule>) -> Result<FunctionCall, ParseError> {
        let mc_str = String::from(method_call.as_str());
        // fun_call | identifier, fun_call
        let mut inner = method_call.clone().into_inner();

        let first = inner.next().unwrap(); // either fun_call or ident
        let first_str = first.as_str();
        let fun_call = inner.next().unwrap();

        let var = match first.as_rule() {
            Rule::identifier => {
                if let Some(var) = self.variables.get(first_str) {
                    if let VarType::Pipe = var.var_def.var_type {
                        var.clone()
                    } else {
                        return Err(ParseError::new(method_call, format!("Cannot call a method on a non-pipe variable: {}", first_str)));
                    }
                } else {
                    return Err(ParseError::new(method_call, format!("Unknown variable {}", first_str)));
                }
            },
            Rule::fun_call => {
                let fc = self.process_fun_call(first)?;
                let ret_type = fc.clone().fun.ret_type;

                if ret_type.is_none() {
                    return Err(ParseError::new(method_call, format!("Attempting to call a function on a function that does not return a value: {}", first_str)));
                }

                let lhs = self.generate_temp(&ret_type.unwrap());

                self.code.push(Expression::Assignment(mc_str, Assignment {
                    lhs: lhs.clone(),
                    rhs: RightHandSide::FunctionCall(fc)
                }));

                lhs
            },
            _ => { panic!("Unknown expansion for method_call: {}", mc_str); }
        };

        let mut ret = self.process_fun_call(fun_call)?;

        // insert the variable as the first argument to the list
        ret.var_list.insert(0, var);

        Ok(ret)
    }

    fn process_fun_call(&mut self, fun_call: Pair<Rule>) -> Result<FunctionCall, ParseError> {
        let fc_str = fun_call.as_str();
        let mut inner = fun_call.clone().into_inner();

        debug!("INNER: {:?}", inner);

        let name = String::from(inner.next().unwrap().as_str());

        let fun = if let Some(fun) = self.user_functions.get(&name) {
            fun.clone()
        } else if let Some(fun) = self.builtin_functions.get(&name) {
            fun.clone()
        } else {
            return Err(ParseError::new(fun_call, String::from(format!("Unknown function {}", name))));
        };

        let var_list = if inner.peek().is_some() {
            inner.next().unwrap().into_inner().map(|exp| {
                let var = self.process_expression(exp).expect("Error processing expression");

                debug!("VAR LIST VAR: {:?}", var);
                debug!("VARIABLES: {:?}", self.variables);

                match var {
                    RightHandSide::Variable(v) => v,
                    _ => panic!("Found non-variable: {:?}", var)
                }
            }).collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // make sure all the variables in the list are known
        for var in &var_list {
            if !self.variables.contains_key(&var.name) {
                panic!("Unknown variable {} in: {}", var.name, fc_str);
            }
        }

        Ok(FunctionCall{ fun, var_list })
    }
}
