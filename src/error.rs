use crate::node::{ChainNode, CommandNode, Nodes, PipeNode};
use gtmpl_value::{FuncError, Value};
use std::{fmt, num::ParseIntError, string::FromUtf8Error};
use thiserror::Error;

#[derive(Debug)]
pub struct ErrorContext {
    pub name: String,
    pub line: usize,
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.name, self.line)
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("unexpected {0} in define clause")]
    UnexpectedInDefineClause(Nodes),
    #[error("unexpected end")]
    UnexpectedEnd,
    #[error("template: {0}:{1}")]
    WithContext(ErrorContext, String),
    #[error("no tree")]
    NoTree,
    #[error(transparent)]
    NodeError(#[from] NodeError),
    #[error("enable gtmpl_dynamic_template to use a pipeline as name")]
    NoDynamicTemplate,
    #[error("unable to parse string: {0}")]
    UnableToParseString(String),
}

impl ParseError {
    pub fn with_context(name: impl ToString, line: usize, msg: impl ToString) -> Self {
        Self::WithContext(
            ErrorContext {
                name: name.to_string(),
                line,
            },
            msg.to_string(),
        )
    }
}

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("unable to unquote")]
    UnquoteError,
    #[error("NaN")]
    NaN,
    #[error("not a tree node")]
    NaTN,
}

#[derive(Error, Debug)]
pub enum PrintError {
    #[error("unable to process verb: {0}")]
    UnableToProcessVerb(String),
    #[error("{0:X} is not a valid char")]
    NotAValidChar(i128),
    #[error("unable to format {0} as {1}")]
    UnableToFormat(Value, char),
    #[error("unable to terminate format arg: {0}")]
    UnableToTerminateFormatArg(String),
    #[error("missing ] in {0}")]
    MissingClosingBracket(String),
    #[error("unable to parse index: {0}")]
    UnableToParseIndex(ParseIntError),
    #[error("unable to parse width: {0}")]
    UnableToParseWidth(ParseIntError),
    #[error("width after index (e.g. %[3]2d)")]
    WithAfterIndex,
    #[error("precision after index (e.g. %[3].2d)")]
    PrecisionAfterIndex,
}

#[derive(Error, Debug)]
pub enum ExecError {
    #[error("{0} is an incomplete or empty template")]
    IncompleteTemplate(String),
    #[error("{0}")]
    IOError(#[from] std::io::Error),
    #[error("unknown node: {0}")]
    UnknownNode(Nodes),
    #[error("expected if or with node, got {0}")]
    ExpectedIfOrWith(Nodes),
    #[error("unable to convert output to uft-8: {0}")]
    Utf8ConversionFailed(FromUtf8Error),
    #[error("empty var stack")]
    EmptyStack,
    #[error("var context smaller than {0}")]
    VarContextToSmall(usize),
    #[error("invalid range {0:?}")]
    InvalidRange(Value),
    #[error("pipeline must yield a String")]
    PipelineMustYieldString,
    #[error("template {0} not defined")]
    TemplateNotDefined(String),
    #[error("exceeded max template depth")]
    MaxTemplateDepth,
    #[error("error evaluating pipe: {0}")]
    ErrorEvaluatingPipe(PipeNode),
    #[error("no arguments for command node: {0}")]
    NoArgsForCommandNode(CommandNode),
    #[error("cannot evaluate command: {0}")]
    CannotEvaluateCommand(Nodes),
    #[error("field chain without fields :/")]
    FieldChainWithoutFields,
    #[error("{0} has arguments but cannot be invoked as function")]
    NotAFunctionButArguments(String),
    #[error("no fields in eval_chain_node")]
    NoFieldsInEvalChainNode,
    #[error("indirection through explicit nul in {0}")]
    NullInChain(ChainNode),
    #[error("cannot handle {0} as argument")]
    InvalidArgument(Nodes),
    #[error("{0} is not a defined function")]
    UndefinedFunction(String),
    #[error(transparent)]
    FuncError(#[from] FuncError),
    #[error("can't give argument to non-function {0}")]
    ArgumentForNonFunction(Nodes),
    #[error("only maps and objects have fields")]
    OnlyMapsAndObjectsHaveFields,
    #[error("no field {0} for {1}")]
    NoFiledFor(String, Value),
    #[error("variable {0} not found")]
    VariableNotFound(String),
}

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error(transparent)]
    ExecError(#[from] ExecError),
    #[error(transparent)]
    ParseError(#[from] ParseError),
}
