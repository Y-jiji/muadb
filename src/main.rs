#![allow(unused)]

// standalone modules
mod util_bytes;
mod util_logging;
mod util_pratt_parser;

// sql modules
mod sql_parser_expr;
mod sql_parser_stmt;
mod sql_parser_space;
mod sql_schema;
mod sql_planner;
mod sql_compiler;
mod sql_error;

// runtime modules
mod rt_transaction;
mod rt_executor;
mod rt_storage;

// in-memory data format modules
mod data_schema;
mod data_buffer;
mod data_parser_primitive;
mod data_parser_string;
mod data_parser_list;
mod data_parser_pair;

// file modules
mod storage_parquet;
mod storage_in_memory;

// physical op modules (sub-op in some literatures)
mod op_collect_hashmap;


#[cfg(test)]
#[ctor::ctor]
fn init() {
    crate::util_logging::init();
}

fn main() {
    println!("hello world");
}