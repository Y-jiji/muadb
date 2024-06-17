use crate::schema::*;
use crate::parser_query::*;

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
        query: &'a SQLQuery<'a>
    },
    // DELETE FROM <table> WHERE (<condition>)
    Delete {
        table: &'a str,
        condition: &'a SQLQuery<'a>
    },
    // UPDATE INTO <table> WHERE (<condition>) VALUES (<query>) 
    Update {
        table: &'a str,
        query: &'a SQLQuery<'a>,
        condition: &'a SQLQuery<'a>,
    },
    // <query>
    Output {
        query: SQLQuery<'a>
    },
}