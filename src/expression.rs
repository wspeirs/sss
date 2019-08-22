use super::Rule;
use pest::iterators::Pair;

use crate::script::SymbolTable;

use std::fmt;

/// An expression is either an assignment or a function call
/// - an assignment to a variable
/// - a function that must be called
#[derive(Debug, Clone)]
pub enum Expression {
    Assignment(String, Assignment),
    FunctionCall(String, FunctionCall)
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expression::Assignment(_, a) => { write!(f, "{}", a) },
            Expression::FunctionCall(_, fc) => { write!(f, "{}", fc) }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum VarType {
    String,
    Number,
    Pipe
}

#[derive(Clone, Debug, PartialEq)]
pub struct VarDef {
    pub var_type:VarType,
    pub is_array:bool
}

impl VarDef {
    pub fn new(var_def: Pair<Rule>) -> VarDef {
        let mut inner = var_def.clone().into_inner();

        let var_type = match inner.next().unwrap().as_str() {
            "str" => { VarType::String },
            "num" => { VarType::Number },
            "pipe" => { VarType::Pipe },
            _ => { panic!("Unknown variable type: {:?}", var_def) }
        };

        VarDef { var_type, is_array: inner.peek().is_some() }
    }

    pub fn from_type(var_type: &VarType) -> VarDef {
        VarDef{ var_type:var_type.clone(), is_array:false }
    }

    pub fn from_array(var_type: &VarType) -> VarDef {
        VarDef{ var_type:var_type.clone(), is_array:true }
    }
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub name:String,
    pub var_def:VarDef
}

impl Variable {
    /// Given a var_dec rule, constructs a variable
    pub fn new(var_dec: Pair<Rule>) -> Variable {
        let mut inner = var_dec.into_inner();
        let name = String::from(inner.next().unwrap().as_str());

        let var_def = VarDef::new(inner.next().unwrap());

        Variable {
            name,
            var_def
        }
    }
}

#[derive(Debug, Clone)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div
}

#[derive(Debug, Clone)]
pub enum RightHandSide {
    Variable(Variable),
    Term(Term),
    Operation(Variable, Operator, Variable),
    FunctionCall(FunctionCall)
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub lhs:Variable,
    pub rhs:RightHandSide
}

impl fmt::Display for Assignment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} = {:?}", self.lhs.name, self.rhs)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub fun:Function,
    pub var_list:Vec<Variable>
}

impl fmt::Display for FunctionCall {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}({:?})", self.fun.name, self.var_list)
    }
}


#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: SymbolTable,      // parameters to the function
    pub ret_type: Option<VarDef>, // return type of the function
    pub code: Vec<Expression>     // code that makes-up the function
}

impl Function {
    /// Constructs a new Function without checking to see if param names are duplicates
    pub fn new(name: &str, params: Vec<Variable>, ret: Option<VarDef>) -> Function {
        let mut symbols = SymbolTable::new();

        // put all the params into a symbol table
        for param in params.iter() {
            symbols.insert(param.clone().name, param.clone());
        }

        Function {
            name: String::from(name),
            params: symbols,
            ret_type: ret,
            code: Vec::<Expression>::new()
        }
    }
}

#[derive(Clone, Debug)]
pub enum Term {
    String(String),
    Number(f64),
    Variable(Variable)
}