use super::Rule;
use pest::iterators::Pair;

/// An expression is 1 of 3 types:
/// - an assignment to a variable
/// - a function that must be called
/// - some set of code we must execute
pub enum Expression {
    Assignment(Assignment),
    FunctionCall(FunctionCall),
    Executable(Executable)
}

#[derive(Clone, Debug)]
pub enum VarType {
    String,
    Number,
    Pipe
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub name:String,
    pub var_type:VarType
}

impl Variable {
    pub fn new(var_def: Pair<Rule>) -> Variable {
        let vd_str = var_def.as_str();
        let mut inner = var_def.into_inner();
        let name = String::from(inner.next().unwrap().as_str());

        let var_type = match inner.next().unwrap().as_str() {
            "str" => { VarType::String },
            "num" => { VarType::Number },
            "pipe" => { VarType::Pipe },
            _ => { panic!("Unknown variable type: {:?}", vd_str) }
        };

        Variable{ name, var_type }
    }
}

pub struct Assignment {
    pub lhs:Variable,
    pub rhs:Executable
}

pub struct FunctionCall {
    pub name:String,
    pub var_list:Vec<Variable>
}

pub struct Executable {
    pub contents:String // totally punting on this for now
}