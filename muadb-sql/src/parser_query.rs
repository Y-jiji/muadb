use crate::schema::*;

// In general, only expressions get compiled to physical operators
pub enum SQLQuery<'a> {
    // SELECT * FROM <table> WHERE <filter>
    Select {
        table:  &'a SQLQuery<'a>,
        filter: &'a SQLQuery<'a>,
    },
    // SELECT 0,3,2,1 FROM <table>
    // <table>.<column>
    Project {
        table:   &'a SQLQuery<'a>,
        columns: &'a [usize],
    },
    // JOIN <lhs>, <rhs> ON <filter>
    // [INNER] JOIN <lhs>, <rhs> ON <filter>
    Join {
        lhs:    &'a SQLQuery<'a>,
        rhs:    &'a SQLQuery<'a>,
        dir:    SQLJoinMethod,
        filter: &'a SQLQuery<'a>,
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
    Left,
    Right,
    Inner,
    Outer,
}

