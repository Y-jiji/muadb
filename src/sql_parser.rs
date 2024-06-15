use std::ops::Add;
use bumpalo::Bump;
use crate::util_pratt_parser::*;

// Each SQL is allocated in this holder structure. 
// We also use it for symbol table and as parsing cache. 
pub struct SQLSpace<'a> {
    bump: &'a Bump,
    tag_slice: &'a [u8],
}

// SQL Statements that have side effects. 
pub enum SQLStmt<'a> {
    // CREATE TABLE <table> COLUMNS (<schema>)
    Create {
        table : &'a str,
        schema: &'a SQLSchema<'a>,
    },
    // INSERT INTO <table> VALUES (<query>)
    Insert {
        table: &'a str,
        query: &'a SQLExpr<'a>
    },
    // DELETE FROM <table> WHERE (<condition>)
    Delete {
        table: &'a str,
        condition: &'a SQLExpr<'a>
    },
    // UPDATE INTO <table> WHERE (<condition>) VALUES (<query>) 
    Update {
        table: &'a str,
        query: &'a SQLExpr<'a>,
        condition: &'a SQLExpr<'a>,
    },
    // <query>
    Output {
        query: SQLExpr<'a>
    },
}

// In general, only expressions get compiled to physical operators
pub enum SQLExpr<'a> {
    // SELECT * FROM <table> WHERE <filter>
    Select {
        table:  &'a SQLExpr<'a>,
        filter: &'a SQLExpr<'a>,
    },
    // SELECT 0,3,2,1 FROM <table>
    // <table>.<column>
    Project {
        table:   &'a SQLExpr<'a>,
        columns: &'a [usize],
    },
    // JOIN <lhs>, <rhs> ON <filter>
    Join {
        lhs:    &'a SQLExpr<'a>,
        rhs:    &'a SQLExpr<'a>,
        dir:    SQLJoinMethod,
        filter: &'a SQLExpr<'a>,
    },
    // <table> -- a table's name
    Name {
        table:  &'a str
    },
    // "<string>"
    Literal {
        string: &'a str
    },
    // <number>
    Integer {
        number: i64
    },
    // <float>
    Float {
        number: f64
    }
}

// JOIN Direction
pub enum SQLJoinMethod {
}

// SQLSchema
pub enum SQLSchema<'a> {
    NamedTuple {
        name:  &'a[&'a str],
        tuple: &'a[SQLSchema<'a>]
    },
    Tuple {
        tuple: &'a[SQLSchema<'a>]
    },
    I64, I32, I16, I8,
    U64, U32, U16, U8,
    Nil, F32, F64, Str, 
}

// SQLError
pub enum SQLError<'a> {
    UndefinedSymbol(usize, &'a str),
}

pub fn parser_stmt<'a>() -> Recursive<'a, SQLStmt<'a>, SQLError<'a>, SQLSpace<'a>> {
    panic!()
}

pub fn parser_stmt_create<'a>() -> Tag<'a, Recursive<'a, SQLStmt<'a>, SQLError<'a>, SQLSpace<'a>>> {
    panic!()
}

pub fn parser_stmt_output<'a>() -> Recursive<'a, SQLStmt<'a>, SQLError<'a>, SQLSpace<'a>> {
    panic!()
}

pub fn parser_expr<'a>() -> Recursive<'a, SQLStmt<'a>, SQLError<'a>, SQLSpace<'a>> {
    panic!()
}

pub fn parser_expr_select() {
    panic!()
}