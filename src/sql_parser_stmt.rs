// use crate::sql_schema::*;
// use crate::sql_parser_expr::*;

// // SQL Statements that have side effects. 
// pub enum SQLStmt<'a> {
//     // CREATE TABLE <table> COLUMNS (<schema>)
//     Create {
//         table : &'a str,
//         schema: &'a SQLSchema<'a>,
//     },
//     // INSERT INTO <table> VALUES (<query>)
//     Insert {
//         table: &'a str,
//         query: &'a SQLExpr<'a>
//     },
//     // DELETE FROM <table> WHERE (<condition>)
//     Delete {
//         table: &'a str,
//         condition: &'a SQLExpr<'a>
//     },
//     // UPDATE INTO <table> WHERE (<condition>) VALUES (<query>) 
//     Update {
//         table: &'a str,
//         query: &'a SQLExpr<'a>,
//         condition: &'a SQLExpr<'a>,
//     },
//     // <query>
//     Output {
//         query: SQLExpr<'a>
//     },
// }

