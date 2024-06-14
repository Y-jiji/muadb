#![allow(unused)]

// standalone modules
mod util_bytes;

// sql modules
mod sql_parser;
mod sql_parser_combinator;
mod sql_optimizer;
mod sql_compiler;

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

// physical op modules
mod op_collect_hashmap;

fn main() {
    println!("Hello, world!");
}