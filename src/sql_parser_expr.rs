use crate::sql_schema::*;

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

// SQLError
pub enum SQLError<'a> {
    UndefinedSymbol(usize, &'a str),
    Unknown,
}